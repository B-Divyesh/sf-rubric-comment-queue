# Independent verification — FAIL

**Work order:** `rubric-comment-queue-verify-1`  
**Candidate:** `e87a8759fcf48b9b2fe1236d627000277b542776` (`main`)  
**Live URL:** https://rubric-comment-queue.sociobot.in  
**Verified:** 2026-08-28 (UTC)

## Verdict

**FAIL.** The teacher-controlled import → review/edit → personal-next-step →
CSV workflow works, and the deployed frontend is byte-for-byte the candidate
build. It does not meet the complete release acceptance contract because:

1. the live service returns `{"status":"ok","build_sha":"unknown"}` at
   `/health`, so its backend build cannot be identified as the candidate; and
2. the deployed hashed JS/CSS and static assets have no HTTP `Cache-Control`
   lifetime. This fails the required long-lived immutable static-asset caching
   policy and Lighthouse reports four cacheable resources with a 0 ms lifetime.

Neither issue was a deployment outage; both are release/readiness failures.

## Passing evidence

### Clean checkout, checks, and builds

The checkout was clean and at the requested SHA before installation. `npm ci`
completed successfully. The following all passed:

| Command | Result |
| --- | --- |
| `npm run check` | 0 errors, 0 warnings |
| `npm test` | 5/5 Vitest tests passed |
| `npm run build` | passed; `dist/` produced |
| `cargo test --locked` | 3/3 tests passed |
| `cargo build --release --locked` | passed (optimized binary built) |
| `npm run test:e2e` | 8/8 Playwright cases passed across Desktop Chrome and Pixel 5 |
| `npm audit --omit=dev --audit-level=low` | 0 production vulnerabilities |

`docker` is not installed in this worker image, so the Docker build could not
be executed. The frontend production build and locked Rust release build were
both executed directly.

The frontend production build is within the stated initial bundle budgets:
JS 62.13 kB / 23.73 kB gzip; CSS 13.95 kB / 3.93 kB gzip; no webfont payload.
The responsive hero is 53,808 bytes desktop and 21,356 bytes at the mobile
variant.

### Independent product exercise

I exercised the production build outside the repository test cases on desktop
1440×960 and mobile 390×844:

- Empty import shows the announced recovery error: “Paste at least one
  response…”.
- Imported three labelled excerpts, changed the rubric criterion, selected a
  teacher-written block, edited/personalized feedback, and used Ctrl+Enter to
  mark the item ready and advance.
- Attempting Save & next without feedback/next step moved focus to the missing
  feedback field.
- Created a custom comment block and confirmed the `1 / 3 ready` state and
  comment bank persist after reload.
- Confirmed no mobile horizontal overflow, a visible focused skip link, and
  reduced-motion transition duration `0.00001s`.
- CSV export, copy feedback, import, required next step, payment-unlock UI,
  and offline local editing are also covered by the passing 8-case Playwright
  suite.

The free workflow remains usable without a license. No scoring, AI generation,
or third-party browser request was observed. The only normal network requests
were same-origin static resources and the privacy-disclosed aggregate
`POST /api/pageview` (204). Browser capture on live mobile produced no console
or page errors.

### Accessibility, policies, offline, and backend

- Fresh axe scans had **0 serious/critical** findings on empty desktop and
  mobile states. Lighthouse mobile accessibility was **100**.
- The live document has `lang=en`, a title, exactly one `h1`, a `main`
  landmark, and no image missing an `alt` attribute. Keyboard use reached and
  visibly focused the skip link; native dialogs were keyboard-operable.
- The live service sends CSP, `X-Content-Type-Options: nosniff`,
  `Referrer-Policy: no-referrer`, Permissions Policy, and same-origin COOP.
  API responses correctly use `Cache-Control: no-store`; unauthenticated
  backup requests returned 401 without exposing data.
- A fresh production-origin browser registration obtained a controlling service
  worker. After setting the context offline, a full page reload succeeded. The
  expected `ERR_INTERNET_DISCONNECTED` fetch message occurred only during this
  forced offline reload.
- Release-server concurrency smoke: 100 concurrent `POST /api/pageview`
  requests all returned successfully. SQLite then contained exactly one
  aggregate pageview row with count 100, the expected two application tables
  (`pageviews`, `encrypted_backups`), and zero backup rows. Unit integration
  tests additionally passed encrypted backup round-trip/delete and missing
  authorization behavior.

### Performance measurement

Fresh Lighthouse 12.8.2 mobile against the locally served production release:
Performance **96**, Accessibility **100**, Best Practices **100**, SEO **100**;
FCP 1.9 s, LCP 2.5 s, TBT 0 ms, CLS 0. The cache audit is the exception noted
below.

### Live candidate comparison

The live HTML references `assets/index-DgiMImYA.js` and
`assets/index-D6CZKBqa.css`, exactly the two filenames produced by this
candidate build. SHA-256 comparison confirms both live contents equal local
`dist/` contents:

| Asset | SHA-256 |
| --- | --- |
| `index-DgiMImYA.js` | `659343f7ee525b67540f1b7c5e5604ddef4236f85f6efc4efcea81eed5aa4444` |
| `index-D6CZKBqa.css` | `54b7ec7982a3247493d51005f0f11f574c43f9759daa0191233e1af5449551fc` |

This confirms the deployed frontend matches the candidate. Backend identity is
not confirmable because of the `unknown` health value.

## Defects

### Medium — live backend has no build identity

`GET https://rubric-comment-queue.sociobot.in/health` returns HTTP 200 with
`build_sha: "unknown"`. The backend-service contract requires this endpoint to
return a build SHA. This prevents confirmation that the live backend matches
`e87a8759fcf48b9b2fe1236d627000277b542776`, including its backup/privacy
behavior. Deploy the image with the candidate SHA supplied as `BUILD_SHA` and
verify `/health` afterwards.

### Medium — hashed static assets are sent without an HTTP cache lifetime

Live `GET` responses for `/assets/index-DgiMImYA.js`,
`/assets/index-D6CZKBqa.css`, `/queue-desk-640.webp`, and `/mark.svg` have no
`Cache-Control` header. Lighthouse reports all four with `cacheLifetimeMs: 0`
and scores the cache audit 0.5. Configure immutable, long-lived caching (for
example `Cache-Control: public, max-age=31536000, immutable`) for content-hash
assets, while retaining appropriate short/no-cache behavior for HTML and API
responses.

## Not treated as defects

- `npm audit` including dev tooling reports three advisories, but production
  dependency audit is clean (0 vulnerabilities).
- The local Vite build emits a Rollup warning about a misplaced pure annotation
  in Svelte output; type checks, tests, browser execution, and production build
  all still pass.
- Live purchased/revoked-license checkout could not be exercised because no
  test license was supplied; free flow, restore-token validation UI,
  unauthorized backup behavior, encryption helper tests, and server contract
  paths were exercised.
