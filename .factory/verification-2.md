# Independent verification 2 — FAIL

**Work order:** `rubric-comment-queue-verify-2`

**Candidate:** `bd131037a8cdaaf8ab5a7641c79a06be9efcb978` (`main`)

**Live URL:** https://rubric-comment-queue.sociobot.in

**Verified:** 2026-08-28 UTC

## Verdict

**FAIL.** The requested candidate is deployed, its clean builds and automated
tests pass, and the core local-first import → teacher edit → personal next step
→ copy/CSV workflow works on desktop and 390 px mobile. It is not release-ready:

1. the advertised $29 Desk Pass checkout returns HTTP 404, so a user cannot buy
   the encrypted-backup feature;
2. the dark theme renders primary text nearly black on a near-black background
   and has axe `serious` contrast findings as low as 1.01:1;
3. visible Close/Cancel controls in all three modal workflows do nothing; and
4. several persistent mobile controls are below the required 44 × 44 px target.

The earlier deployment identity/cache failure is resolved. Both `/health` and
`/api/health` report the exact requested SHA, and current cache policies pass.

## Candidate and deployment identity

- Verification started from a clean detached worktree at exactly
  `bd131037a8cdaaf8ab5a7641c79a06be9efcb978`; `origin/main` was the same SHA.
- Live `/health` and `/api/health` both returned
  `{"status":"ok","build_sha":"bd131037a8cdaaf8ab5a7641c79a06be9efcb978"}`.
- The live JS, CSS, both hero images, SVG mark, service worker, and manifest were
  byte-for-byte equal to the candidate `dist/` output. Representative hashes:
  JS `659343f7ee525b67540f1b7c5e5604ddef4236f85f6efc4efcea81eed5aa4444`;
  CSS `54b7ec7982a3247493d51005f0f11f574c43f9759daa0191233e1af5449551fc`.

## Clean checks and exact builds

| Command | Fresh result |
| --- | --- |
| `npm ci` | passed; 121 packages installed |
| `npm run check` | passed; 0 errors and 0 warnings |
| `npm test` | passed; 5/5 Vitest tests |
| `npm run build` | passed; produced `dist/` |
| `cargo test --locked` | passed; 4/4 backend integration tests |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed |
| `BUILD_SHA=bd131… cargo build --release --locked` | passed |
| `npm run test:e2e` | passed; 8/8 Playwright cases (desktop and Pixel 5) |
| `npm audit --omit=dev --audit-level=low` | 0 production vulnerabilities |
| `/opt/fleet/lib/verify-url.sh <live URL> <evidence dir>` | passed; HTTP 200, 799 ms, no errors, title/lang/main/alt/button checks passed |

There is no separate frontend lint script. The repository's Svelte/TypeScript
check and Rust clippy check both passed. A Docker engine is not installed in
this verifier image, so `docker build` could not be rerun; the constituent
locked production builds passed and the live container reports the candidate
SHA. The Vite build repeats the non-fatal Rollup warning about a misplaced
`/* @__PURE__ */` annotation in generated Svelte output.

The full development dependency audit reports three direct dev-tool findings
(Svelte moderate, Vite high, Vitest critical). They are not shipped Node runtime
dependencies and the implicated SSR/dev/UI server paths are not used in this
static frontend, so they are recorded but not treated as release defects.

## Independent product exercise

On the live deployment at 1440 × 960, I imported three excerpts including
paragraph breaks, an unlabeled response, commas, and quotes. I changed rubric
criteria, selected teacher-authored blocks, edited the feedback, added personal
next steps, copied complete feedback, used Ctrl+Enter, and reached `3 / 3 ready`.
The downloaded CSV had one header plus three rows, correctly doubled embedded
quotes, and included `Next step:` text. A custom comment persisted after reload.
Cancel/confirm paths for comment and response deletion behaved correctly.

Invalid-input and recovery evidence:

- blank import announces “Paste at least one response…”;
- a 1,000,001-byte text file is rejected with the documented 1 MB recovery;
- Save & next with both fields blank focuses `feedback-draft`;
- after feedback is supplied, a missing next step focuses `next-step`;
- a wrong encryption passphrase produces the documented recovery message; and
- corrupted cached license-verdict JSON is not recovered: it emits a page error
  and leaves `<main aria-busy="true">` indefinitely (low-severity defect below).

The core free workflow does not score or generate prose. The teacher chooses
and edits every comment. Local reload persistence, local backup download,
clipboard output, CSV export, and confirmed deletion all worked.

## Accessibility, keyboard, responsive behavior, and motion

- Fresh axe scans found zero serious/critical issues in the light empty,
  populated desktop, populated 390 px mobile, and light legal states.
- The dark empty state has one serious `color-contrast` rule affecting four
  nodes. Dark `/privacy` has the same serious rule affecting ten nodes.
- The first Tab exposes the skip link with a 4 px orange focus outline. Enter
  skips to main content; modal focus starts on Close, remains trapped natively,
  and Escape closes and returns focus to the opener. Ctrl+Enter advances review.
- At exactly 390 × 844, both empty and populated states had
  `scrollWidth === clientWidth === 390`; no console or page errors occurred.
- Under `prefers-reduced-motion: reduce`, the media query matched, transition
  duration was `0.00001s`, and document scrolling was `auto`.
- Five visible mobile targets measured below 44 px: the 36 × 36 home link,
  72.4 × 38 Desk Pass button, 173 × 40 local-backup button, 36.7 × 14 Privacy
  link, and 29.8 × 14 Terms link.

## Privacy, network, and response policies

A fresh default load made only these requests: same-origin HTML, CSS, JS, hero,
mark, and `POST /api/pageview` (204). No student text, analytics script, webfont,
cookie, or third-party request was observed. The only external request during a
license-return test was the documented Sociobot verification endpoint; it
returned `{valid:false, reason:"invalid"}` and the app removed the token from
the URL while retaining the free workspace.

An independent WebCrypto round trip confirmed that the backup envelope exposes
only `v`, `salt`, `iv`, and `data`, contains neither the student excerpt nor
teacher feedback in plaintext, decrypts with the correct passphrase, and fails
closed with the wrong one. Live backup requests with no token and with an
invalid token both returned 401 and no CORS allow-origin header; no backup data
was disclosed.

Live responses include CSP with `frame-ancestors 'none'`, `nosniff`,
`Referrer-Policy: no-referrer`, restrictive Permissions Policy, and same-origin
COOP. HTML and legal pages send `no-cache`; API/health send `no-store`; JS, CSS,
images, and SVG send a one-year immutable policy. HTTP redirects to HTTPS.
`Strict-Transport-Security` is absent (low-severity hardening gap below).

## PWA, performance, and backend behavior

- The service worker installed and controlled the live 390 px page. An explicit
  `registration.update()` completed, the active worker remained activated, and
  cache `rcq-shell-v1` existed. A full offline reload rendered the app with no
  page error.
- An uncontended Lighthouse 13.4.1 mobile run scored Performance **99**,
  Accessibility **100**, Best Practices **100**, and SEO **100**: FCP 1.3 s,
  LCP 1.5 s, TBT 100 ms, CLS 0, Speed Index 1.3 s, cache insight 1.0.
- Initial JS is 62.13 KB raw / 23.73 KB gzip; CSS is 13.95 KB raw / 3.93 KB
  gzip; mobile hero is 21,356 bytes; there is no font payload. All are within
  contract budgets.
- A fresh live health load held 100 requests/s for 10 seconds: 1,000 requests,
  103 requests/s average, 21.82 ms average latency, 104 ms maximum, no non-2xx.
- The release binary started with only `PORT=4190` in an otherwise clean
  environment. One hundred concurrent pageview writes all returned 204 and the
  SQLite aggregate was exactly 100. There were zero backup rows. After graceful
  SIGINT and restart against the default database, the count remained 100 and
  health still reported the candidate SHA.

## Defects

### High — advertised paid checkout is unavailable

The live `Buy Desk Pass securely` link targets
`https://api.sociobot.in/api/v1/products/rubric-comment-queue/checkout`.
Fresh HEAD and GET requests both returned HTTP 404; GET returned
`{"error":"enabled factory product","status":404}`. Consequently a user
cannot purchase the advertised encrypted-backup feature and the paid workflow
cannot be completed. Register/enable the live billing product and verify the
hosted checkout, return URL, valid license, revoked license, and live encrypted
backup lifecycle before release.

### High — dark theme is materially unreadable and fails axe

After selecting dark theme, primary text still computes to `#151515` while the
backgrounds are `#171713`/`#24231f`. Axe reports a serious contrast violation:
the main h1 is 1.01:1, the empty-state h2/body are about 1.16:1, and Desk Pass
is 1.56:1. On dark `/privacy`, the h1, all section headings, and body paragraphs
are about 1.01:1. This is visually unreadable and violates the explicit
both-theme 4.5:1 requirement. The root `color` needs to follow the dark `--ink`
token, followed by axe checks on empty, populated, dialogs, privacy, and terms.

### Medium — modal Close and Cancel controls are inert

Clicking or keyboard-activating import Close, import Cancel, comment Cancel,
or backup Close leaves the corresponding `<dialog>` open. All controls are
submit buttons inside forms whose submit handler always prevents the dialog
method's default close behavior. Escape works on desktop, but the visible and
touch-oriented recovery controls do not. Make close/cancel actions explicitly
close the bound dialog and add coverage for all three dialogs and focus return.

### Medium — mobile controls miss the 44 px target contract

At 390 px, five persistent interactive targets are below 44 px in at least one
dimension: home (36 × 36), Desk Pass (72.4 × 38), local backup (173 × 40),
Privacy (36.7 × 14), and Terms (29.8 × 14). Increase their hit areas without
reducing the current visible-focus treatment.

### Low — malformed license cache causes a page error and incomplete startup

With `sb_license_verdict:rubric-comment-queue` set to malformed JSON, reload
raises `Expected property name or '}'…`, leaves the main landmark permanently
`aria-busy="true"`, and skips keyboard/online listener setup. The import button
still works, but the app does not enter a valid ready state. Parse this cache in
a guarded path, discard invalid data, and announce the recovery as is already
done for a malformed workspace.

### Low — missing HSTS and incomplete startup configuration audit

HTTPS responses have good CSP/referrer/permissions/nosniff/COOP controls and
plain HTTP redirects to HTTPS, but they do not include
`Strict-Transport-Security`. The `PORT`-only backend startup log also records
only the port, not which configuration was defaulted versus supplied as required
by the backend runtime contract. Add HSTS at the application or ingress and log
configuration provenance without values or secrets.

## Retest scope

After fixes, retest the real hosted checkout and a valid/revoked license; all
dark-theme states with axe and visual inspection; pointer and keyboard closing
for every dialog; 390 px hit boxes; corrupt local caches; and the complete core
workflow. Preserve the now-correct candidate health identity, cache policy,
offline shell, local-first privacy behavior, and passing build/test gates.
