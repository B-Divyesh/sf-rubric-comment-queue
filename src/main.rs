use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Request, State},
    http::{header, HeaderName, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::signal;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
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

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://data/rubric-comment-queue.db?mode=rwc".to_string());
    if let Some(path) = sqlite_parent(&database_url) {
        std::fs::create_dir_all(path).expect("database directory must be writable");
    }
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("database connection");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("database migrations");
    let state = AppState {
        pool,
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .expect("http client"),
        billing_base: env::var("BILLING_API_BASE")
            .unwrap_or_else(|_| "https://api.sociobot.in/api/v1".to_string()),
        verify_billing: true,
    };
    let frontend = env::var("FRONTEND_DIR").unwrap_or_else(|_| "dist".to_string());
    let app = build_router(state, PathBuf::from(frontend));
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("listen address");
    info!(port, "rubric comment queue listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server error");
}

fn sqlite_parent(url: &str) -> Option<PathBuf> {
    let path = url.strip_prefix("sqlite://")?.split('?').next()?;
    PathBuf::from(path)
        .parent()
        .map(PathBuf::from)
        .filter(|parent| !parent.as_os_str().is_empty())
}

fn build_router(state: AppState, frontend: PathBuf) -> Router {
    let governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_millisecond(10)
            .burst_size(200)
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
    let static_files =
        ServeDir::new(&frontend).fallback(ServeFile::new(frontend.join("index.html")));
    Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .fallback_service(static_files)
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
        build_sha: option_env!("BUILD_SHA").unwrap_or("development"),
    })
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
    let is_api = request.uri().path().starts_with("/api/");
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
    headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self' https://api.sociobot.in; base-uri 'self'; frame-ancestors 'none'; form-action 'self' https://api.sociobot.in"));
    if is_api {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
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
