# Handoff — Rubric Comment Queue

Completed 2026-08-28 for work order `rubric-comment-queue-build-1`.

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
cargo test
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
- `cargo test`: 3/3 integration tests passed (health, anonymous page count,
  authorization, encrypted backup round trip and deletion).
- `npm run test:e2e`: 8/8 Playwright tests passed across Desktop Chrome and
  Pixel 5, covering import → edit → personalized ready state → CSV export,
  keyboard shortcut, paid unlock, legal page, and offline editing.
- Playwright axe: zero serious or critical violations on empty and legal states,
  desktop and mobile.
- Factory `verify-url.sh`: HTTP 200, 636 ms load, no console/page errors,
  title present, `lang=en`, exactly one h1, main landmark present, 0 images
  missing alt, and 0 unlabeled buttons.
- Lighthouse 12.8.2 mobile: Performance 100, Accessibility 100, Best Practices
  100, SEO 100; FCP 1.1 s, LCP 1.6 s, TBT 30 ms, CLS 0.
- Bundles: initial JS 62.13 KB (23.73 KB gzip), CSS 13.95 KB (3.93 KB gzip),
  no runtime CDN assets, and no webfont payload.
- Release load smoke: autocannon fixed at 100 req/s for 10 seconds against
  `/health`; 1,000 requests, 102.2 req/s average, 1.47 ms average latency,
  14 ms maximum latency.
- `npm audit --omit=dev`: 0 production vulnerabilities.
- `cargo build --release --locked`: passed.

## Known gaps and factory next steps

- Docker is not installed in this worker image, so `docker build` could not be
  executed here. Both constituent clean builds (`npm ci`/Vite and locked Rust
  release) pass; the factory should run the Docker build in its normal image
  pipeline.
- The factory still needs to register the test/live Sociobot paid product and
  confirm checkout return URLs. No product ID or payment-provider secret is
  hardcoded.
- A real purchased/revoked license was unavailable in the worker, so live
  Sociobot checkout and verification were not exercised. The browser and server
  both use the documented production endpoints; local contract behavior and
  unauthorized paths are covered.
- Pilot measurement (30 responses, 30% faster, 90% sendable) requires teacher
  use after deployment and is not a build-time claim.
