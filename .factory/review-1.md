# Review teacher-written feedback before export — Review 1

**Work order:** `rubric-comment-queue-review-1`  
**Reviewed:** 2026-09-05 UTC  
**Live URL:** https://rubric-comment-queue.sociobot.in  
**Implementation candidate:** `5256a202f522f0290edae7131ec7b92046de0aa3`  
**Documentation baseline:** `d33fe129f08c44b75314902ee77ca56229e5b955`  
**Live build identity:** `2f688865eb09557e883d60919b2e7b0253657a78`

## Verdict

**FAIL — 8 findings and 23 untested public claims.**

The manual teacher workflow works, but the required sample sandbox is absent,
the advertised paid checkout is broken, and server data is not configured for
the assigned `/data` mount. Claims, mobile accessibility, first-screen copy,
routing, metadata, and the Dockerfile also remain incomplete. A successful test
run does not change this verdict.

The live build SHA is a report-only commit. Its frontend assets are byte-for-byte
equal to a clean build of implementation commit `5256a202…`; implementation
files are unchanged from that commit through documentation baseline `d33fe129…`.

## Before scrolling

Fresh contexts were used at 1440 × 960 and 390 × 844.

| Question | Desktop | Phone |
| --- | --- | --- |
| Job shown | “Feedback stays yours” does not name the job. The next line says responses become sendable feedback. | Same copy; it does not state the job plainly. |
| Audience shown | “Teacher-authored · teacher-approved” implies teachers, but there is no direct audience sentence. | Same. |
| First action | `Add the first responses` is visible. | No primary action is visible before scrolling; the illustration and copy push it below 844 px. |

The first screen does not provide the required `Try it with sample data` action
or three short facts beside it.

## Findings

### High — there is no sample demo and `/demo` reads real workspace data

A clean `/demo` opens the same empty application as `/`. It has no sample data,
no “Demo — sample data, nothing is saved” label, no reset, and no “Start for
real” action. After creating three records on `/`, opening `/demo` displayed the
same `1 / 3 ready` queue and used the same `rcq_workspace:v1` local-storage key.
This is the opposite of the required separate demo namespace. `.factory/demo.md`
is also absent. The required sample, persistent label, reset, and isolation
could not be exercised because they do not exist.

### High — the advertised $29 purchase path returns 404

`Buy Desk Pass securely` points to the required Sociobot URL, but a fresh GET
returned HTTP 404 with `{"error":"enabled factory product","status":404}`.
This is not an expected product 404: it is a broken purchase link. A valid and
revoked license, cross-device restore, encrypted backup save/get/delete, and
two-valid-tenant isolation therefore remain untestable live. Missing and invalid
tokens correctly return 401 without data.

### High — SQLite is not stored on the assigned durable mount

The Dockerfile sets `DATABASE_URL=sqlite:///app/data/rubric-comment-queue.db` and
the README tells operators to persist `/app/data`. This work order assigns the
fleet-created mount at `/data`, and the runtime contract requires durable files
there. With the factory supplying only `PORT`, the container bypasses `/data`;
paid backups and page counts can be lost on redeploy. A local process restart in
one working directory did preserve three page views, which proves SQLite restart
behavior but not fleet redeploy durability.

### High — all 23 public claim groups lack required claim tests

`.factory/claims.json` is missing and the repository contains no `@claim:` test.
There are therefore no declared claim commands to run. Existing unit and browser
tests and this review's manual checks provide useful evidence, but they do not
satisfy the required one-tagged-test-per-claim sandbox contract. The claim audit
below lists all 23 groups counted as untested. `.factory/copy-audit.md` is also
absent.

### Medium — the first screen and landing structure do not meet the plain-copy contract

The h1, “Feedback stays yours,” is not the user's job and relies on figurative
language. The supporting sentence does not directly name teachers. The mobile
first viewport has no primary action. The page also lacks a Demo navigation
link, a three-step “How it works” section, a plain limitations/privacy section,
and an in-page paid-tier section. The upgrade is available only inside a dialog.

### Medium — route titles, discovery metadata, and the 404 route are incomplete

`/`, `/demo`, `/privacy`, `/terms`, and an unknown route all use the title
“Rubric Comment Queue — feedback stays yours.” Legal and demo routes do not set
their required titles. Unknown paths and `/404.html` return HTTP 200 with the
queue rather than a designed 404. `/sitemap.xml` also returns that HTML shell.
Canonical, Open Graph, Twitter card, and apple-touch metadata are absent. The
footer omits “Built by Param Factory” and a build/version identifier. A deliberate
HTTP 404 would be expected; this finding is the missing 404 structure and the
unexpected 200 application response.

### Medium — four touch targets and mobile body text remain below contract

At 390 px, the custom-comment remove button measured 30 × 30 px, Desk Pass
privacy and terms links measured 38.4 × 14 and 28.4 × 14 px, and the legal-page
back link measured 116.5 × 14 px. Mobile body text computes to 16 px, not the
17 px specified by the product design and mobile baseline. Axe found no serious
or critical violations, but axe does not detect these contract failures.

### Medium — the Dockerfile pins a forbidden Rust minor version

The backend stage uses `rust:1.88-alpine`; the backend contract requires
`rust:1-alpine` or `rust:1-slim` so current locked dependencies can use the
factory's stable toolchain. The documented `docker build` command could not be
run in this worker because no Docker-compatible engine is installed. The Vite
and locked Rust release builds passed independently, so this finding is the
explicit Dockerfile contract violation, not a claimed observed image-build
failure.

## Public claim audit

All rows are **UNTESTED under the claims contract** because there is no claims
manifest and no tagged claim command. “Observed” records supplementary manual
or generic-suite evidence; it does not turn the row into a declared claim test.

| ID | Public claim group | Current evidence |
| --- | --- | --- |
| C01 | Paste excerpts into a response queue | Observed with three realistic responses. |
| C02 | Import a plain-text file up to 1 MB | Boundary rejection observed; successful file import is covered only by an untagged suite. |
| C03 | Optional labels and `---` separation work | Observed labels `Roster 12`, `Roster 13`, and `Response 3`. |
| C04 | Attach a rubric criterion | Observed in the populated workspace. |
| C05 | Choose and edit a teacher-written comment block | Observed. |
| C06 | Require a personal next step before ready | Observed focus recovery on `next-step`. |
| C07 | Copy complete feedback | Observed clipboard text with `Next step:`. |
| C08 | Export the batch as CSV | Observed three data rows and the expected fields. |
| C09 | Download a local JSON backup | Covered only by untagged code/tests. |
| C10 | Autosave and retain work locally | Observed after reload. |
| C11 | Work offline after the first visit | Observed controlled service worker and successful offline reload. |
| C12 | Send no student writing to an AI model | No student text appeared in captured requests. |
| C13 | Do no model training and create no student profiles | Stated publicly; no declared test. |
| C14 | Do no auto-grading, scoring, plagiarism detection, or generated comments | Manual behavior is consistent; no declared test. |
| C15 | Sell Desk Pass for $29 once | **False live:** checkout returns 404. |
| C16 | Keep the local queue, comment bank, copy, CSV, accessibility, and offline use free | Parts observed; no declared test. |
| C17 | Encrypt cloud backup in-browser with AES-256-GCM and keep the passphrase local | Covered only by untagged helper tests/manual prior evidence. |
| C18 | Store only ciphertext, update time, and a license hash | Schema/code inspected; live valid-license path is untested. |
| C19 | Prevent the service from reading backup contents or recovering a passphrase | Stated publicly; no declared test. |
| C20 | Restore on another device and delete the cloud backup | Untestable live because checkout is broken. |
| C21 | Count only aggregate page views and use no analytics scripts or advertising cookies | Request capture was consistent; no declared test. |
| C22 | Keep payment details outside this service; Sociobot/Dodo handles billing and refund revocation | Checkout is broken; no declared test. |
| C23 | Report build SHA, emit JSON logs, and shut down gracefully | Observed locally and at health; no declared test. |

**Untested claim count: 23.**

## Working paths and checks

The core non-demo flow works on desktop and phone. Three realistic excerpts
produced a populated queue, rubric choices, teacher-written blocks, editable
feedback, and a required personal next step. `Ctrl+Enter` advanced the queue.
Reload retained the queue and custom block. Copy returned the complete feedback.
CSV contained one header and three rows. Delete cancel kept the response; delete
confirm removed it.

Invalid and recovery checks passed: blank import announced a linked error;
a 1,000,001-byte file gave a clear 1 MB recovery message; missing feedback and
next step moved focus correctly; malformed workspace and license caches recovered
without page errors. Dark-mode and legal-state browser tests passed. There was no
horizontal overflow at 390 px.

Accessibility checks found zero serious or critical axe violations in live
empty, populated, privacy, and terms states. Keyboard order begins with the skip
link, every sampled focus outline is a visible 4 px orange ring, dialogs close
and restore focus, and reduced motion computes to near-instant transitions with
automatic scrolling. `verify-url.sh` passed with one h1, one main, `lang=en`,
alt text, button names, and no ordinary-load console errors.

The service worker was active and controlling, `registration.update()` completed,
and `rcq-shell-v1` existed. Offline reload worked. The only console error was the
expected failed network request during forced offline mode. Normal workflow
traffic stayed same-origin; no request body contained the sample student text.

Backend checks passed for `/health`, invalid authorization, basic restart
persistence, and rate limiting. A 1,000-request burst from one forwarded client
returned 598 × 401 and 402 × 429; every 429 included `Retry-After: 0`. A
1,000-request run with distinct `X-Forwarded-For` clients returned 1,000 × 401,
showing separate allowances. Health returned the live build SHA.

The prior live Lighthouse score remains applicable because every frontend byte
matches the reviewed implementation: Performance 97, Accessibility 100, Best
Practices 100, SEO 100. The fresh build is 62.65 KB JS raw / 23.91 KB gzip and
14.13 KB CSS raw / 3.94 KB gzip; the mobile hero is 21,356 bytes and no font is
loaded.

## Clean-checkout command results

| Command | Result |
| --- | --- |
| `npm ci` | Passed; 121 packages installed. Full audit reports three dev-tool advisories. |
| `npm run check` | Passed; 0 errors and 0 warnings. |
| `npm test` | Passed; 5/5 tests. |
| `npm run build` | Passed; produced `dist/`. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --locked` | Passed; 6/6 tests. |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed. |
| `BUILD_SHA=5256a202… cargo build --release --locked` | Passed. |
| `npm run test:e2e` | Passed; 15 passed, 1 expected project-specific skip. |
| `npm audit --omit=dev --audit-level=low` | Passed; 0 production vulnerabilities. |
| `/opt/fleet/lib/verify-url.sh <live URL>` | Passed; HTTP 200 in 668 ms, no reported errors. |
| Every command in `.factory/claims.json` | Not run: the required file is missing. |
| `docker build -t rubric-comment-queue .` | Not run: `docker` is not installed in this worker. |

## Earlier finding disposition

| Earlier finding | Current disposition |
| --- | --- |
| Unknown live build identity | Fixed; both health paths report `2f688865…`. |
| Missing immutable asset caching | Fixed; live assets retain one-year immutable caching. |
| Broken Desk Pass checkout | **Open**; live GET still returns 404. |
| Unreadable dark theme | Fixed; local full-state suite and matching live assets have zero serious/critical axe findings. |
| Inert dialog Close/Cancel controls | Fixed; pointer and keyboard cases pass with focus return. |
| First set of five undersized persistent controls | Fixed by regression coverage. |
| Malformed license cache blocks startup | Fixed; cache is removed and `aria-busy=false`. |
| Missing HSTS | Fixed; live sends one-year HSTS with subdomains. |
| Missing startup configuration provenance | Fixed; PORT-only run logs supplied/default sources without values. |
| Four additional undersized mobile targets | **Open**; all four measurements reproduced. |
| Mobile body text is 16 px | **Open**; reproduced at 390 px. |

## Evidence and next work

Screenshots and generated review artifacts are under `/work/.evidence/`, including
`live-desktop-first-screen.png`, `live-mobile-first-screen.png`,
`live-desktop-populated.png`, `live-demo-route.png`, `live-mobile-desk-pass.png`,
the exported CSV, and `verify-url/` output.

Repair order: implement an isolated one-click sample; enable and test billing;
move SQLite defaults and container persistence to `/data`; add the claim manifest
and tagged sandbox tests; then fix first-screen copy, routes/metadata/404, mobile
targets/type, and the Rust base tag. Re-run every claim command and the full
matrix before considering PASS.
