# Independent verification 3 — FAIL

**Work order:** `rubric-comment-queue-verify-3`

**Candidate:** `2f688865eb09557e883d60919b2e7b0253657a78` (`main`)

**Live URL:** https://rubric-comment-queue.sociobot.in

**Verified:** 2026-08-28 UTC

## Verdict

**FAIL.** The requested candidate is deployed, builds reproducibly, and the
core local-first teacher workflow works end to end on desktop and at 390 px.
The prior dark-mode, dialog-dismissal, cached-license, HSTS, and persistent
mobile-control defects are repaired. Release acceptance is still not met:

1. **High:** the advertised $29 Desk Pass checkout still returns HTTP 404, so
   encrypted backup cannot be purchased or verified end to end;
2. **Medium:** four live mobile targets remain below the required 44 × 44 px
   hit area outside the five targets covered by the repair regression; and
3. **Low:** mobile body text computes to 16 px, contrary to the 17 px visual
   thesis and attached mobile type baseline.

No product code was changed during verification.

## Candidate, checkout, and live identity

- Verification began with a clean `main` checkout at exactly
  `2f688865eb09557e883d60919b2e7b0253657a78`; freshly fetched `origin/main`
  was the same SHA.
- Live `/health` and `/api/health` both return HTTP 200 with
  `build_sha: "2f688865eb09557e883d60919b2e7b0253657a78"`.
- Live HTML, JS, CSS, both WebP images, SVG mark, manifest, service worker, and
  robots file are byte-for-byte equal to the clean local `dist/` output.
  Representative SHA-256 values are JS
  `c4ecdcb0682f925ddf212cd94589161c792e0c8c510e1b04b3d62bd01ba79b37`
  and CSS
  `fbecac363743ea721c679d651a9934688b574546a3bf537db841e20df04f582b`.
- HTTP redirects to HTTPS. The live document and asset `Last-Modified` time is
  2026-08-28 04:58:42 UTC, consistent with the candidate deployment.

This independently resolves the earlier deployment-identity uncertainty. The
current live backend and frontend both identify as the requested candidate.

## Clean checks and exact production builds

| Command | Fresh result |
| --- | --- |
| `npm ci` | passed; 121 packages installed from lockfile |
| `npm run check` | passed; 0 errors, 0 warnings |
| `npm test` | passed; 5/5 Vitest tests |
| `cargo fmt --all -- --check` | passed |
| `cargo test --locked` | passed; 6/6 backend tests |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed |
| `npm run build` | passed; produced `dist/` |
| `BUILD_SHA=2f688… cargo build --release --locked` | passed; health reports the exact candidate SHA |
| `npm run test:e2e` | passed; 15 passed, 1 expected project-specific skip |
| `npm audit --omit=dev --audit-level=low` | passed; 0 production vulnerabilities |
| `/opt/fleet/lib/verify-url.sh` (local and live) | passed; 200, title/lang/one h1/main/alt/button checks, no errors |

There is no separate frontend lint script. The full development dependency
audit reports one moderate Svelte advisory, one high Vite advisory, and one
critical Vitest-UI advisory. This product does not use Svelte SSR or Vitest UI,
and Vite/Vitest are not shipped in the runtime; the production dependency audit
is clean, so these are recorded as development-tool upgrade work rather than a
release defect.

No Docker-compatible engine (`docker`, `podman`, or `buildah`) is installed in
the verifier image, so the Dockerfile itself could not be rebuilt locally. Both
locked constituent production builds passed, the live container reports the
candidate SHA, and every deployed frontend artifact matches the clean build.

## End-to-end product exercise

Fresh independent runs exercised both the local release binary and the live
deployment at 1440 × 960 and 390 × 844:

- blank import produced the announced “Paste at least one response” recovery;
- a 1,000,001-byte `.txt` file produced the documented over-1-MB recovery;
- three responses were imported with paragraphs, an omitted label, commas, and
  quotes; the fallback label and paragraph preservation were correct;
- a rubric criterion was changed, teacher-authored blocks were selected and
  edited, a custom block was created, and a personal next step was required;
- attempting to advance with both fields blank focused Feedback draft; after
  feedback was present, a missing next step focused that field;
- Ctrl+Enter advanced the queue, local reload retained all work, clipboard copy
  included `Next step:`, and the batch reached `3 / 3 ready`;
- CSV export contained exactly three data rows, valid doubled quote escaping,
  criteria, feedback, next steps, and statuses; local JSON backup contained the
  three submissions and custom comment;
- response deletion was dismissed once and then confirmed; the queue changed
  only after confirmation;
- malformed workspace and license-verdict JSON recovered to an interactive
  empty workspace with no page error and removed the invalid verdict cache;
- an imported `<img onerror>`/`<script>` payload remained inert text (no
  injected image, script execution, console error, or page error);
- a license return query was saved to the documented local-storage key and
  stripped from the URL; an invalid token locked the paid state with “This
  license is no longer active” while leaving the free workspace usable.

A separate WebCrypto boundary test encrypted a workspace containing private
student and teacher text. The envelope exposed only `v`, `salt`, `iv`, and
`data`, contained no plaintext, round-tripped under the correct passphrase, and
failed closed for a wrong or too-short passphrase.

The product does not score, profile, detect plagiarism, or generate prose. The
teacher chooses and edits every comment.

## Accessibility, keyboard, mobile, and motion

- Playwright axe 4.11 found **zero serious/critical findings** in empty,
  populated, import-dialog, Desk Pass, privacy, and terms states across light
  and dark treatments and desktop/mobile viewports.
- Live and local pages have `lang=en`, a title, exactly one h1, a main landmark,
  labelled controls, and no image missing alt text. Dialog close/cancel controls
  work by pointer and keyboard and restore focus to their openers.
- The first Tab reveals the skip link with a visible 4 px orange outline.
  Activating it sets `location.hash` to `#main`, but the active element becomes
  `<body>` rather than `<main>`. A follow-up Tab lands directly on the first
  workspace action and bypasses every header control, so the skip behavior is
  functionally correct and is not treated as a defect.
- At 390 × 844, empty and populated layouts both measured
  `scrollWidth === clientWidth === 390`; screenshots showed no clipping or
  obscured fixed actions.
- Under `prefers-reduced-motion: reduce`, the media query matched, transitions
  computed to `0.00001s`, and scroll behavior was `auto`.
- The five targets fixed in the preceding repair now meet 44 px. Broader live
  measurement found the remaining undersized targets listed below.

## Privacy, requests, policies, and storage boundaries

A fresh default browser load requested only same-origin HTML, hashed JS/CSS,
the product mark/hero, and the disclosed aggregate `POST /api/pageview` (204).
There were no analytics scripts, webfonts, advertising cookies, CDNs, or other
third-party requests. Captured request bodies contained no student or teacher
text. The sole external browser request occurred only during explicit license
restore and went to the documented Sociobot verification endpoint.

Unauthenticated and short-token backup reads returned 401 JSON without backup
data. A valid-length but invalid token also returned 401 after Sociobot
verification. Cross-origin requests to the product backup API received no CORS
allow-origin header. The billing verifier allows the live product origin and
returned `{valid:false, reason:"invalid"}` with `Cache-Control: no-store`.

Live responses include HSTS (`max-age=31536000; includeSubDomains`), CSP with
`frame-ancestors 'none'`, `nosniff`, `Referrer-Policy: no-referrer`, restrictive
Permissions Policy, and same-origin COOP. HTML/legal routes use `no-cache`, API
and health use `no-store`, hashed JS/CSS/images/SVG use one-year immutable
caching, and service worker/manifest/robots entry points use `no-cache`.

## PWA, performance, concurrency, and persistence

- The live 390 px browser obtained an activated, controlling service worker;
  explicit `registration.update()` completed, cache `rcq-shell-v1` existed, and
  a full offline reload rendered successfully with no page error. The expected
  `ERR_INTERNET_DISCONNECTED` resource message occurred only during forced
  offline mode.
- Fresh Lighthouse 13.4.1 mobile results were live **97 / 100 / 100 / 100** and
  local **99 / 100 / 100 / 100** for Performance / Accessibility / Best
  Practices / SEO. Live FCP was 1.4 s, LCP 1.6 s, TBT 170 ms, CLS 0, Speed Index
  1.4 s, and the cache insight scored 1.0.
- Initial JS is 62,651 bytes raw / 23.91 KB gzip; CSS is 14,128 bytes raw /
  3.94 KB gzip; the mobile hero is 21,356 bytes; no font payload loads. All
  explicit budgets pass.
- Fixed-rate 100 req/s, 10 s health smokes completed 1,000 requests locally
  (1.88 ms average, 15 ms maximum) and live (21.13 ms average, 91 ms maximum)
  with no non-2xx responses.
- The optimized release binary started with literally only `PORT=4193` in its
  environment. Startup JSON identified `PORT` as supplied and database,
  frontend, and billing base as defaulted without logging values or secrets.
- One hundred concurrent pageview writes all returned 204 and advanced the
  SQLite aggregate exactly from 11 to 111. The database contained only the
  migration metadata, aggregate pageviews, and encrypted-backup table, with
  zero backup rows. After graceful SIGINT and restart against the same default
  database, count 111 persisted and health still reported the candidate SHA.

## Defects

### High — advertised paid checkout is still unavailable

The visible `Buy Desk Pass securely` link correctly targets
`https://api.sociobot.in/api/v1/products/rubric-comment-queue/checkout`, but a
fresh GET returns HTTP 404 with
`{"error":"enabled factory product","status":404}`. The equivalent pilot
endpoint also returns 404, and the product is absent from the public enabled
products response. Therefore a teacher cannot purchase the advertised backup,
and checkout return, valid/revoked license, and live encrypted-backup lifecycle
cannot be certified. This is external factory billing registration, but it is
still a user-visible release blocker under the acceptance contract.

### Medium — four remaining live mobile targets are below 44 × 44 px

At exactly 390 × 844, broader measurement than the existing regression found:

| Target | Measured box |
| --- | --- |
| Remove custom comment | 30 × 30 px |
| Desk Pass `privacy` link | 38.4 × 14 px |
| Desk Pass `terms` link | 28.4 × 14 px |
| Privacy/terms `← Back to the queue` link | 116.5 × 14 px |

The remove control is both destructive and overlaid at the comment-block edge,
making its undersized touch area especially error-prone. Increase the interactive
boxes while preserving the visible layout and at least 8 px adjacent spacing.
The hidden 1 px native file input was not treated as a defect because its visible
associated file-button label is 44 px high.

### Low — mobile body text contradicts the 17 px design contract

At the 390 px breakpoint, `body` computes to 16 px because the max-width 430 px
rule overrides the 17 px base declared in `.factory/design.md`. The attached
mobile typography baseline also requires 17 pt. Restore at least 17 px body text
and recheck the responsive layout at 390 px and 200% text zoom.

## Retest scope

After repair, retest the hosted checkout with a real purchase and return token,
valid/revoked verification, and encrypted backup save/restore/delete; remeasure
every interactive target in populated, legal, and Desk Pass mobile states; and
confirm 17 px mobile type without overflow. Preserve the verified candidate
identity, local-first workflow,
zero-serious axe result, response policies, offline shell, and passing builds.
