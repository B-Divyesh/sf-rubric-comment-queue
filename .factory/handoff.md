# Handoff — Rubric Comment Queue

## Repair verification update — 2026-08-28 — **PASS and deployed**

This repair resolves both medium findings in `.factory/verification.md` without
changing the teacher review workflow:

1. The Dockerfile declares `BUILD_SHA=dev` globally and passes it into the Rust
   compile stage. `/health` now refuses the ambiguous `unknown` value; a
   production build compiled with `BUILD_SHA=repair-candidate-e87a8759` returned
   `{"status":"ok","build_sha":"repair-candidate-e87a8759"}`.
2. The backend now sends `Cache-Control: public, max-age=31536000, immutable`
   for JS, CSS, images, SVG, and font assets. HTML and app routes (`/`,
   `/privacy`, `/terms`) revalidate with `no-cache`; `/health` and `/api/*`
   remain `no-store`. This preserves service-worker and release discovery.

Exact regression coverage is in Rust: `health_reports_ok` asserts that health
never reports `unknown`, and
`caching_keeps_release_documents_fresh_and_assets_immutable` checks the root,
legal pages, health/API, hashed JS, hero image, icon, and service worker cache
policies through the complete Axum router.

The configured Azure Container App deployed image
`sociobotregistry.azurecr.io/sf-rubric-comment-queue:99d1192cf866` from repair
commit `99d1192cf866a931162089567fcb8a4991205753`. Live verification at
`https://rubric-comment-queue.sociobot.in` returned that exact `build_sha`;
the hashed JS/CSS, hero image, and SVG returned the one-year immutable policy,
while `/`, `/privacy`, and `/api/health` returned `no-cache`, `no-cache`, and
`no-store` respectively.

Completed locally 2026-08-28 for work order `rubric-comment-queue-repair-1`.

## What shipped

- A teacher-controlled review queue for importing plain-text writing excerpts,
  assigning a rubric criterion, choosing/editing teacher-written comment blocks,
  adding a required personal next step, and moving each response to ready.
- Local-first autosave, individual deletion confirmation, copy feedback, CSV
  export, downloadable JSON backup, dark treatment, responsive 390px layout,
  visible keyboard focus, `Ctrl/Cmd+Enter` advancement, live announcements, and
  a service-worker offline shell.
- First-class empty, import-error, storage-error, offline, and license-error
  states. No scoring, AI prose generation, plagiarism detection, or profiling.
- A $29 one-time Desk Pass through the required Sociobot hosted checkout and
  license verification contract. The free review/export workflow is not gated.
  License return, daily verdict cache, offline cached unlock, restore-by-token,
  and revoked-license handling are implemented.
- Optional paid encrypted backup. AES-256-GCM encryption and PBKDF2 key
  derivation happen in the browser; the passphrase never leaves the device. The
  axum service independently verifies the license and stores only ciphertext,
  an update time, and a SHA-256 license hash in SQLite. Save, restore, overwrite,
  and deletion are implemented.
- Rust/axum service with SQLite migrations, parameterized queries, request size
  limits, per-IP rate limiting, secure response headers/CSP, aggregate daily page
  count, JSON logs, graceful shutdown, and `/health` build metadata.
- A multi-stage Dockerfile builds the Vite frontend and optimized Rust binary,
  then runs as an unprivileged Alpine user on `PORT=8080`.
- Original neo-brutalist desk illustration generated for this product, reviewed
  for artifacts, and optimized to 21 KB mobile / 53 KB desktop WebP. Prompt and
  deployment provenance are in `.factory/design.md` and `assets/src/`.
- Privacy and terms pages, MIT license, complete README, web manifest, icon, and
  robots policy.

The researched brief described subscription monetization, while the attached
paid-unlock contract requires a one-time license. The implementation follows the
required contract and states the one-time $29 price clearly.

## Run and verify

```sh
npm ci
npm run check
npm test
npm run build
cargo test --locked
cargo build --release --locked
npm run test:e2e
```

Local production run:

```sh
DATABASE_URL='sqlite://data/local.db?mode=rwc' FRONTEND_DIR=dist cargo run
```

Container deployment uses `docker build -t rubric-comment-queue .` and should
persist `/app/data`. Required frontend build command is exactly `npm run build`;
its deploy output is `dist/`, with `dist/index.html` at the root.

## Verification results

- `npm run check`: 0 errors, 0 warnings.
- `npm test`: 5/5 Vitest tests passed (import boundaries/paragraphs, feedback,
  CSV, default bank).
- `cargo test --locked`: 4/4 integration tests passed (health identity,
  cache-policy regression, anonymous page count,
  authorization, encrypted backup round trip and deletion).
- `npm run test:e2e`: 8/8 Playwright tests passed across Desktop Chrome and
  Pixel 5, covering import → edit → personalized ready state → CSV export,
  keyboard shortcut, paid unlock, legal page, and offline editing.
- Playwright axe: zero serious or critical violations on empty and legal states,
  desktop and mobile.
- Lighthouse 13.4.1 mobile after the repair: Performance 100, Accessibility
  100, Best Practices 100, SEO 100; FCP 1.1 s, LCP 1.6 s, TBT 30 ms, CLS 0,
  and `cache-insight` 1.0.
- Bundles: initial JS 62.13 KB (23.73 KB gzip), CSS 13.95 KB (3.93 KB gzip),
  no runtime CDN assets, and no webfont payload.
- Release load smoke: autocannon fixed at 100 req/s for 10 seconds against
  `/health`; 1,000 requests, 102.2 req/s average, 1.47 ms average latency,
  14 ms maximum latency.
- `npm audit --omit=dev`: 0 production vulnerabilities.
- `cargo build --release --locked`: passed with an explicit build identity.
- Production response-policy smoke on the release binary: build identity was
  `repair-candidate-e87a8759`; hashed JS, hero WebP, and SVG each returned
  `public, max-age=31536000, immutable`; `/` and `/privacy` returned
  `no-cache`; `/api/health` returned `no-store`; all 100 concurrent
  `POST /api/pageview` requests returned 204.
- `verify-url.sh` against the repair server: HTTP 200, 641 ms load, no console
  or page errors, title and `lang=en` present, exactly one h1 and main
  landmark, 0 images missing alt, and 0 unlabeled buttons.

## Known gaps and factory next steps

- Docker is not installed in this worker image, so a local `docker build` was
  not run. The configured Azure ACR container build did succeed and deployed
  the verified image named above; both constituent clean builds also pass.
- The factory still needs to register the test/live Sociobot paid product and
  confirm checkout return URLs. No product ID or payment-provider secret is
  hardcoded.
- A real purchased/revoked license was unavailable in the worker, so live
  Sociobot checkout and verification were not exercised. The browser and server
  both use the documented production endpoints; local contract behavior and
  unauthorized paths are covered.
- Pilot measurement (30 responses, 30% faster, 90% sendable) requires teacher
  use after deployment and is not a build-time claim.
