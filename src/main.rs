use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Request, State},
    http::{header, HeaderName, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{
    migrate::MigrateError,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Connection, SqliteConnection, SqlitePool,
};
use std::{env, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc, time::Duration};
use tokio::signal;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::{info, warn};

const PRODUCT_SLUG: &str = "rubric-comment-queue";
const MAX_BACKUP_BYTES: usize = 5_000_000;

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    client: reqwest::Client,
    billing_base: String,
    verify_billing: bool,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    build_sha: &'static str,
}

#[derive(Deserialize)]
struct BackupInput {
    payload: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct BackupOutput {
    payload: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct BillingVerdict {
    valid: bool,
}

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("authorization required")]
    Unauthorized,
    #[error("backup not found")]
    NotFound,
    #[error("invalid backup payload")]
    InvalidPayload,
    #[error("license verification is temporarily unavailable")]
    VerificationUnavailable,
    #[error("service error")]
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InvalidPayload => StatusCode::UNPROCESSABLE_ENTITY,
            Self::VerificationUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rubric_comment_queue=info".parse().unwrap()),
        )
        .init();

    let (port, port_source) = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok().map(|port| (port, "supplied")))
        .unwrap_or((8080, "default"));
    let default_database = default_database_url();
    let (database_url, database_url_source) = env_or_default("DATABASE_URL", &default_database);
    if let Some(path) = sqlite_parent(&database_url) {
        std::fs::create_dir_all(path).expect("database directory must be writable");
    }
    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("valid SQLite database URL")
        .busy_timeout(Duration::from_secs(30));
    migrate_database(&options)
        .await
        .expect("database migrations after retry window");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("database connection");
    let (billing_base, billing_api_base_source) =
        env_or_default("BILLING_API_BASE", "https://api.sociobot.in/api/v1");
    let (frontend, frontend_dir_source) = env_or_default("FRONTEND_DIR", "dist");
    let state = AppState {
        pool,
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .expect("http client"),
        billing_base,
        verify_billing: true,
    };
    let app = build_router(state, PathBuf::from(frontend));
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("listen address");
    info!(
        port,
        port_source,
        database_url_source,
        frontend_dir_source,
        billing_api_base_source,
        "runtime configuration resolved"
    );
    info!(port, "rubric comment queue listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server error");
}

/// Resolve non-secret runtime settings without ever writing their values to the
/// startup log. The deployment only supplies `PORT`; the other defaults keep a
/// first boot usable and the source labels make that visible to operators.
fn env_or_default(name: &str, default: &str) -> (String, &'static str) {
    match env::var(name) {
        Ok(value) if !value.is_empty() => (value, "supplied"),
        _ => (default.to_owned(), "default"),
    }
}

fn default_database_url() -> String {
    let path = if std::path::Path::new("/data").is_dir() {
        "/data/rubric-comment-queue.db"
    } else {
        "data/rubric-comment-queue.db"
    };
    format!("sqlite://{path}?mode=rwc")
}

fn sqlite_parent(url: &str) -> Option<PathBuf> {
    let path = url.strip_prefix("sqlite://")?.split('?').next()?;
    PathBuf::from(path)
        .parent()
        .map(PathBuf::from)
        .filter(|parent| !parent.as_os_str().is_empty())
}

async fn migrate_database(options: &SqliteConnectOptions) -> Result<(), MigrateError> {
    const ATTEMPTS: u8 = 12;
    for attempt in 1..=ATTEMPTS {
        let mut connection = SqliteConnection::connect_with(options)
            .await
            .map_err(MigrateError::Execute)?;
        let result = sqlx::migrate!().run(&mut connection).await;
        let _ = connection.close().await;
        match result {
            Ok(()) => return Ok(()),
            Err(_error) if attempt < ATTEMPTS => {
                warn!(attempt, "database migration is busy; retrying");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("migration retry loop always returns")
}

fn build_router(state: AppState, frontend: PathBuf) -> Router {
    let governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_millisecond(50)
            .burst_size(40)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("rate limit config"),
    );
    let api = Router::new()
        .route("/health", get(health))
        .route("/pageview", post(pageview))
        .route(
            "/backup",
            get(get_backup).put(put_backup).delete(delete_backup),
        )
        .layer(DefaultBodyLimit::max(MAX_BACKUP_BYTES + 2048))
        .layer(GovernorLayer::new(governor));
    let not_found = std::fs::read_to_string(frontend.join("404.html")).unwrap_or_else(|_| {
        "<!doctype html><html lang=\"en\"><title>Page not found — Rubric Comment Queue</title><main><h1>Page not found</h1><a href=\"/\">Return to the queue</a></main></html>".to_owned()
    });
    Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .route_service("/", ServeFile::new(frontend.join("index.html")))
        .route_service("/demo", ServeFile::new(frontend.join("index.html")))
        .route_service("/privacy", ServeFile::new(frontend.join("index.html")))
        .route_service("/terms", ServeFile::new(frontend.join("index.html")))
        .nest_service("/assets", ServeDir::new(frontend.join("assets")))
        .route_service("/mark.svg", ServeFile::new(frontend.join("mark.svg")))
        .route_service(
            "/queue-desk.webp",
            ServeFile::new(frontend.join("queue-desk.webp")),
        )
        .route_service(
            "/queue-desk-640.webp",
            ServeFile::new(frontend.join("queue-desk-640.webp")),
        )
        .route_service(
            "/social-card.webp",
            ServeFile::new(frontend.join("social-card.webp")),
        )
        .route_service(
            "/apple-touch-icon.png",
            ServeFile::new(frontend.join("apple-touch-icon.png")),
        )
        .route_service(
            "/manifest.webmanifest",
            ServeFile::new(frontend.join("manifest.webmanifest")),
        )
        .route_service("/sw.js", ServeFile::new(frontend.join("sw.js")))
        .route_service("/robots.txt", ServeFile::new(frontend.join("robots.txt")))
        .route_service("/sitemap.xml", ServeFile::new(frontend.join("sitemap.xml")))
        .fallback(move || {
            let body = not_found.clone();
            async move { (StatusCode::NOT_FOUND, Html(body)) }
        })
        .layer(middleware::from_fn(security_headers))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .with_state(state)
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        build_sha: build_identity(),
    })
}

/// The release builder supplies BUILD_SHA at compile time. Keep local builds
/// identifiable too, but never report the ambiguous `unknown` value that made
/// a deployed backend impossible to tie to a source revision.
fn build_identity() -> &'static str {
    option_env!("BUILD_SHA")
        .filter(|sha| !sha.is_empty() && *sha != "unknown")
        .unwrap_or("development")
}

async fn pageview(State(state): State<AppState>) -> Result<StatusCode, ApiError> {
    let day = Utc::now().format("%Y-%m-%d").to_string();
    sqlx::query("INSERT INTO pageviews(day, count) VALUES(?, 1) ON CONFLICT(day) DO UPDATE SET count = count + 1")
        .bind(day).execute(&state.pool).await.map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn put_backup(
    State(state): State<AppState>,
    request: Request,
) -> Result<StatusCode, ApiError> {
    let (parts, body) = request.into_parts();
    let token = authorize(&state, parts.headers.get(header::AUTHORIZATION)).await?;
    let bytes = axum::body::to_bytes(body, MAX_BACKUP_BYTES + 2048)
        .await
        .map_err(|_| ApiError::InvalidPayload)?;
    let input: BackupInput =
        serde_json::from_slice(&bytes).map_err(|_| ApiError::InvalidPayload)?;
    validate_payload(&input.payload)?;
    let hash = token_hash(&token);
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO encrypted_backups(license_hash, payload, updated_at) VALUES(?, ?, ?) ON CONFLICT(license_hash) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at")
        .bind(hash).bind(input.payload).bind(now).execute(&state.pool).await.map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_backup(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<BackupOutput>, ApiError> {
    let token = authorize(&state, request.headers().get(header::AUTHORIZATION)).await?;
    let row = sqlx::query_as::<_, BackupOutput>(
        "SELECT payload, updated_at FROM encrypted_backups WHERE license_hash = ?",
    )
    .bind(token_hash(&token))
    .fetch_optional(&state.pool)
    .await
    .map_err(internal)?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(row))
}

async fn delete_backup(
    State(state): State<AppState>,
    request: Request,
) -> Result<StatusCode, ApiError> {
    let token = authorize(&state, request.headers().get(header::AUTHORIZATION)).await?;
    sqlx::query("DELETE FROM encrypted_backups WHERE license_hash = ?")
        .bind(token_hash(&token))
        .execute(&state.pool)
        .await
        .map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn authorize(state: &AppState, value: Option<&HeaderValue>) -> Result<String, ApiError> {
    let token = value
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| (12..=2048).contains(&token.len()))
        .ok_or(ApiError::Unauthorized)?;
    if !state.verify_billing {
        return Ok(token.to_owned());
    }
    let url = format!(
        "{}/products/{}/verify",
        state.billing_base.trim_end_matches('/'),
        PRODUCT_SLUG
    );
    let response = state
        .client
        .get(url)
        .query(&[("license", token)])
        .send()
        .await
        .map_err(|error| {
            warn!(?error, "license verification request failed");
            ApiError::VerificationUnavailable
        })?;
    if !response.status().is_success() {
        return Err(ApiError::VerificationUnavailable);
    }
    let verdict: BillingVerdict = response
        .json()
        .await
        .map_err(|_| ApiError::VerificationUnavailable)?;
    if !verdict.valid {
        return Err(ApiError::Unauthorized);
    }
    Ok(token.to_owned())
}

fn validate_payload(payload: &str) -> Result<(), ApiError> {
    if payload.len() > MAX_BACKUP_BYTES {
        return Err(ApiError::InvalidPayload);
    }
    let value: serde_json::Value =
        serde_json::from_str(payload).map_err(|_| ApiError::InvalidPayload)?;
    let valid = value.get("v").and_then(|v| v.as_u64()) == Some(1)
        && value
            .get("salt")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.len() >= 16)
        && value
            .get("iv")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.len() >= 12)
        && value
            .get("data")
            .and_then(|v| v.as_str())
            .is_some_and(|v| !v.is_empty());
    if valid {
        Ok(())
    } else {
        Err(ApiError::InvalidPayload)
    }
}

fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
fn internal(error: sqlx::Error) -> ApiError {
    warn!(?error, "database operation failed");
    ApiError::Internal
}

async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path();
    let cache_control = cache_control_for(path);
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self' https://api.sociobot.in; base-uri 'self'; frame-ancestors 'none'; form-action 'self' https://api.sociobot.in"));
    if let Some(cache_control) = cache_control {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(cache_control),
        );
    }
    response
}

/// Cache files that are safe to reuse for a year, but always revalidate the
/// document and service-worker entry points that discover a new release.
fn cache_control_for(path: &str) -> Option<&'static str> {
    const IMMUTABLE: &str = "public, max-age=31536000, immutable";
    const REVALIDATE: &str = "no-cache";

    if path == "/health" || path.starts_with("/api/") {
        return Some("no-store");
    }
    if matches!(path, "/" | "/demo" | "/privacy" | "/terms")
        || path.ends_with(".html")
        || matches!(
            path,
            "/sw.js" | "/manifest.webmanifest" | "/robots.txt" | "/sitemap.xml"
        )
    {
        return Some(REVALIDATE);
    }
    if matches!(
        path.rsplit_once('.').map(|(_, extension)| extension),
        Some(
            "js" | "css"
                | "svg"
                | "webp"
                | "avif"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "ico"
                | "woff2"
        )
    ) {
        return Some(IMMUTABLE);
    }
    None
}

async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("install Ctrl+C handler") };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    info!("graceful shutdown requested");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tempfile::tempdir;
    use tower::ServiceExt;

    fn request() -> axum::http::request::Builder {
        Request::builder().extension(axum::extract::ConnectInfo(SocketAddr::from((
            [127, 0, 0, 1],
            3000,
        ))))
    }

    async fn test_app() -> Router {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.html"),
            "<!doctype html><title>test</title>",
        )
        .unwrap();
        build_router(
            AppState {
                pool,
                client: reqwest::Client::new(),
                billing_base: String::new(),
                verify_billing: false,
            },
            dir.keep(),
        )
    }

    #[tokio::test]
    async fn health_reports_ok() {
        let response = test_app()
            .await
            .oneshot(request().uri("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["status"],
            "ok"
        );
        assert_ne!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["build_sha"],
            "unknown"
        );
    }

    #[tokio::test]
    async fn caching_keeps_release_documents_fresh_and_assets_immutable() {
        let app = test_app().await;
        for (path, expected) in [
            ("/", "no-cache"),
            ("/privacy", "no-cache"),
            ("/terms", "no-cache"),
            ("/health", "no-store"),
            ("/api/health", "no-store"),
            (
                "/assets/index-abcd1234.js",
                "public, max-age=31536000, immutable",
            ),
            (
                "/queue-desk-640.webp",
                "public, max-age=31536000, immutable",
            ),
            ("/mark.svg", "public, max-age=31536000, immutable"),
            ("/sw.js", "no-cache"),
        ] {
            let response = app
                .clone()
                .oneshot(request().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                expected,
                "unexpected cache policy for {path}"
            );
        }
    }

    #[tokio::test]
    async fn known_routes_use_the_app_shell_and_unknown_routes_return_the_designed_404() {
        let app = test_app().await;
        for path in ["/", "/demo", "/privacy", "/terms"] {
            let response = app
                .clone()
                .oneshot(request().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "route {path}");
        }
        let response = app
            .oneshot(request().uri("/missing-page").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(String::from_utf8_lossy(&body).contains("Page not found"));
    }

    #[tokio::test]
    async fn rate_limit_uses_forwarded_client_and_includes_retry_after() {
        let app = test_app().await;
        let mut limited = None;
        for _ in 0..80 {
            let response = app
                .clone()
                .oneshot(
                    request()
                        .uri("/api/backup")
                        .header("x-forwarded-for", "203.0.113.8, 10.0.0.4")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                limited = Some(response);
                break;
            }
        }
        let limited = limited.expect("first forwarded client should exhaust its allowance");
        assert!(limited.headers().contains_key(header::RETRY_AFTER));

        let other_client = app
            .oneshot(
                request()
                    .uri("/api/backup")
                    .header("x-forwarded-for", "203.0.113.9")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(other_client.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn sends_hsts_with_the_secure_response_policy() {
        let response = test_app()
            .await
            .oneshot(request().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response
                .headers()
                .get(header::STRICT_TRANSPORT_SECURITY)
                .unwrap(),
            "max-age=31536000; includeSubDomains"
        );
    }

    #[test]
    fn configuration_defaults_have_explicit_provenance() {
        let (value, source) = env_or_default("RCQ_TEST_MISSING_SETTING", "fallback");
        assert_eq!(value, "fallback");
        assert_eq!(source, "default");
    }

    #[tokio::test]
    async fn backup_round_trip_and_delete() {
        let app = test_app().await;
        let payload =
            r#"{"v":1,"salt":"abcdefghijklmnop","iv":"abcdefghijkl","data":"ciphertext"}"#;
        let response = app
            .clone()
            .oneshot(
                request()
                    .method("PUT")
                    .uri("/api/backup")
                    .header("authorization", "Bearer valid-test-license")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"payload":{}}}"#,
                        serde_json::to_string(payload).unwrap()
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let response = app
            .clone()
            .oneshot(
                request()
                    .uri("/api/backup")
                    .header("authorization", "Bearer valid-test-license")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = app
            .oneshot(
                request()
                    .method("DELETE")
                    .uri("/api/backup")
                    .header("authorization", "Bearer valid-test-license")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn encrypted_backups_are_isolated_by_license_hash() {
        let app = test_app().await;
        for (token, marker) in [
            ("valid-license-tenant-a", "ciphertext-a"),
            ("valid-license-tenant-b", "ciphertext-b"),
        ] {
            let payload = format!(
                r#"{{"v":1,"salt":"abcdefghijklmnop","iv":"abcdefghijkl","data":"{marker}"}}"#
            );
            let response = app
                .clone()
                .oneshot(
                    request()
                        .method("PUT")
                        .uri("/api/backup")
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::json!({ "payload": payload }).to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }

        for (token, own_marker, other_marker) in [
            ("valid-license-tenant-a", "ciphertext-a", "ciphertext-b"),
            ("valid-license-tenant-b", "ciphertext-b", "ciphertext-a"),
        ] {
            let response = app
                .clone()
                .oneshot(
                    request()
                        .uri("/api/backup")
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body = response.into_body().collect().await.unwrap().to_bytes();
            let body = String::from_utf8_lossy(&body);
            assert!(body.contains(own_marker));
            assert!(!body.contains(other_marker));
        }
    }

    #[tokio::test]
    async fn sqlite_pageview_state_survives_pool_restart() {
        let dir = tempdir().unwrap();
        let database = dir.path().join("restart.db");
        let url = format!("sqlite://{}?mode=rwc", database.display());
        let first = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!().run(&first).await.unwrap();
        let day = "2099-01-01";
        sqlx::query("INSERT INTO pageviews(day, count) VALUES(?, 3)")
            .bind(day)
            .execute(&first)
            .await
            .unwrap();
        first.close().await;

        let restarted = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count FROM pageviews WHERE day = ?")
            .bind(day)
            .fetch_one(&restarted)
            .await
            .unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn concurrent_startup_migrations_recover_from_sqlite_lock_contention() {
        let dir = tempdir().unwrap();
        let database = dir.path().join("concurrent-start.db");
        let url = format!("sqlite://{}?mode=rwc", database.display());
        let first = SqliteConnectOptions::from_str(&url).unwrap();
        let second = SqliteConnectOptions::from_str(&url).unwrap();
        let (first_result, second_result) =
            tokio::join!(migrate_database(&first), migrate_database(&second));
        assert!(first_result.is_ok());
        assert!(second_result.is_ok());
    }

    #[tokio::test]
    async fn rejects_missing_license_and_tracks_anonymous_view() {
        let app = test_app().await;
        let unauthorized = app
            .clone()
            .oneshot(request().uri("/api/backup").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
        let pageview = app
            .oneshot(
                request()
                    .method("POST")
                    .uri("/api/pageview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(pageview.status(), StatusCode::NO_CONTENT);
    }
}
