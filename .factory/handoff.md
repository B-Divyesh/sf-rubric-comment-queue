# Handoff — Rubric Comment Queue

## Repair 2 — 2026-08-28 — code repair verified locally; billing registration remains external

This repair addresses every repository-owned finding from independent
verification 2 while preserving the teacher-controlled local workflow and the
existing Sociobot checkout integration:

- Dark mode now applies `--ink` at the inheritance root and body. Playwright
  axe has zero serious/critical findings in dark empty, populated, Desk Pass,
  privacy, and terms states on desktop and mobile.
- All visible dialog dismissal controls explicitly call `HTMLDialogElement.close`.
  Keyboard and pointer regression coverage verifies import close/cancel, comment
  close/cancel, backup close, and focus return to each opener.
- The brand link, Desk Pass button, local-backup button, and footer legal links
  have 44 px minimum hit areas. The 390 x 844 mobile regression measures all
  five controls directly.
- Malformed cached license-verdict JSON is now discarded in a guarded path,
  announces recovery, and finishes startup. Regression coverage asserts no page
  error, `aria-busy="false"`, and removal of the bad cache value.
- HTTPS transport policy now includes
  `Strict-Transport-Security: max-age=31536000; includeSubDomains`; Rust route
  coverage protects the header. Startup logs now state whether `PORT`, database,
  frontend directory, and billing base were supplied or defaulted, without
  logging values or secrets. A clean `PORT`-only release run reported
  `port_source="supplied"` and all other sources `"default"`.

The advertised Desk Pass checkout remains correctly wired to the required
`https://api.sociobot.in/api/v1/products/rubric-comment-queue/checkout` endpoint,
but both live and pilot endpoints return `404 {"error":"enabled factory
product"}`. The product is absent from the public enabled-products list. This
is an external Sociobot billing registration/enablement task; repository rules
prohibit changing billing configuration, so no misleading replacement checkout
or fake unlock was introduced. A real purchase, return token, valid/revoked
verification, and encrypted cloud lifecycle cannot be retested until the
factory registers the product and return URL.

### Repair verification (local)

```sh
npm ci
npm run check                 # 0 errors, 0 warnings
npm test                      # 5/5
npm run build                 # dist/
cargo test --locked           # 6/6
cargo clippy --all-targets --all-features --locked -- -D warnings
BUILD_SHA=repair-local-20260828 cargo build --release --locked
npm run test:e2e              # 15 passed, 1 desktop-only mobile test skipped
npm audit --omit=dev --audit-level=low  # 0 production vulnerabilities
```

`verify-url.sh` against the PORT-only release binary passed (200; title,
`lang=en`, one h1, main, image alt, and button labels). Local mobile Lighthouse
13.4.1 scored Performance **100**, Accessibility **100**, Best Practices
**100**, SEO **100** (FCP 1.1 s, LCP 1.5 s, CLS 0). The service worker was
activated/controlling with cache `rcq-shell-v1`; a 390 px offline reload
rendered the app with no page errors. A local 100 req/s, 10 s health smoke
completed 1,000 requests (101.5 req/s average; 1.6 ms average; 13 ms max).

### Deployment and live evidence

The configured Azure Container App deployment built and released
`sociobotregistry.azurecr.io/sf-rubric-comment-queue:5256a202f522` from source
commit `5256a202f522f0290edae7131ec7b92046de0aa3`. Live `/health` and
`/api/health` both returned that exact SHA. The live JS and CSS matched the
local `dist/` byte-for-byte (JS SHA-256
`c4ecdcb0682f925ddf212cd94589161c792e0c8c510e1b04b3d62bd01ba79b37`; CSS
`fbecac363743ea721c679d651a9934688b574546a3bf537db841e20df04f582b`).

Live `verify-url.sh` passed (HTTP 200, 673 ms, no console/page errors, title,
language, one h1, main, alt text, and button labels). HTTPS returns the new
HSTS policy together with the existing CSP, no-referrer policy, restrictive
permissions policy, same-origin COOP, nosniff, and document `no-cache` policy.
A fresh live desktop browser check found only the product origin; after dark
selection the h1 computed to `rgb(250, 246, 234)`, import Close closed the
dialog, and there were no page errors. At live 390 x 844 the five repaired hit
areas measured 44 x 44, 72.4 x 44, 173 x 44, 44 x 44, and 44 x 44 px.

The live checkout endpoint was retested after deployment and still returns
HTTP 404. This remains the sole release blocker and must be cleared by factory
billing product registration before the paid backup lifecycle can be certified.

## Independent verification 2 — 2026-08-28 — **FAIL**

Candidate `bd131037a8cdaaf8ab5a7641c79a06be9efcb978` is deployed at
https://rubric-comment-queue.sociobot.in and both health routes report that exact
SHA. Clean install, type checking, unit/integration/e2e tests, clippy, frontend
production build, locked Rust release build, live asset equality, offline
reload, performance budgets, privacy checks, and backend concurrency/persistence
checks pass. The core teacher-controlled feedback workflow works on desktop and
390 px mobile.

**Release verdict remains FAIL.** Fresh independent testing found these release
defects:

- **High:** the visible $29 Desk Pass checkout returns HTTP 404, so encrypted
  backup cannot be purchased.
- **High:** dark theme leaves primary copy nearly black on near-black surfaces;
  axe reports serious contrast failures down to 1.01:1.
- **Medium:** visible Close/Cancel controls do not close any of the three modal
  workflows (Escape is the only verified dismissal path).
- **Medium:** five persistent 390 px controls measure below 44 × 44 px.
- **Low:** malformed cached license JSON produces a page error and permanent
  `aria-busy`, and live HTTPS lacks HSTS/startup config-source logging.

Exact evidence, commands, hashes, metrics, and retest scope are in
`.factory/verification-2.md`. No product code was changed by verification.

## Builder repair update — 2026-08-28 — deployed (superseded verdict)

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
