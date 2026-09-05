# Rubric Comment Queue

Rubric Comment Queue is for teachers reviewing many writing responses. It keeps
the teacher in control of every comment.

Paste excerpts or import a plain-text file up to 1 MB. Choose a rubric
criterion, edit a teacher-written comment block, and add one personal next step.
Copy finished feedback or export one CSV row per response. You can also download
the complete local workspace as JSON.

The free workflow needs no account or payment. Queue edits stay in browser
storage and return after reload. The installed app works offline after its first
visit. Student writing is not sent during the free review workflow. The app
does not score, rewrite, or generate feedback.

Try the isolated sample at
https://rubric-comment-queue.sociobot.in/demo. Its three responses use a
separate `demo:` storage key and never change the real workspace.

## Run locally

Requirements: Node 22+, current stable Rust, and SQLite development libraries.

```sh
npm ci
npm run build
PORT=8080 cargo run --locked
```

Open `http://127.0.0.1:8080`. The server serves the built frontend.

For frontend development, run `npm run dev`. Vite proxies `/api` to port 8080.

## Test

```sh
npm run check
npm test
npm run build
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
npm run test:e2e
```

Each public claim and its clean command are listed in
`.factory/claims.json`. The browser tests use the isolated `/demo` route.

## Deploy

Build the multi-stage container from the repository root:

```sh
docker build --build-arg BUILD_SHA="$(git rev-parse HEAD)" -t rubric-comment-queue .
docker run --rm -p 8080:8080 -v rcq-data:/data rubric-comment-queue
```

The container starts with only `PORT` set. SQLite uses `/data` when that mount
exists and selects SQLite's network-filesystem lock mode there. Without the
mount it uses `data/` beside the process working directory. Keep one replica for
the SQLite writer. `GET /health` reports status and build identity.

Optional environment overrides are `DATABASE_URL`, `FRONTEND_DIR`, and
`BILLING_API_BASE`. The backend rate-limits API requests by the first forwarded
client address and returns `Retry-After` with HTTP 429.

## Privacy and scope

The service counts aggregate page views without analytics scripts, tracking
cookies, or third-party fonts. See `/privacy` and `/terms` for user-facing
policies. The server keeps aggregate counts in SQLite under `/data` in the
deployed container.

This release does not sell a paid plan. The Sociobot billing product must be
registered before a paid backup feature can be offered.

The researched scope is in `.factory/brief.json`. The visual system and asset
provenance are in `.factory/design.md`.

## License

MIT — see [LICENSE](LICENSE).
