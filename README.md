# Rubric Comment Queue

Rubric Comment Queue helps teachers turn batches of writing excerpts into
specific, sendable feedback without auto-grading. Teachers choose their own
rubric criterion and comment block, edit every draft, add a personal next step,
then copy feedback or export the batch as CSV.

The free workspace is local-first, works offline after first load, and sends no
student writing to an AI model. A $29 one-time Desk Pass adds optional browser-
encrypted cloud backup; the server stores only ciphertext.

Live: https://rubric-comment-queue.sociobot.in

## Develop

Requirements: Node 22+, Rust 1.88+, and SQLite development libraries.

```sh
npm ci
npm run dev                    # frontend at http://localhost:5173

# In a second terminal
npm run build
DATABASE_URL='sqlite://data/dev.db?mode=rwc' cargo run
```

Vite proxies `/api` to port 8080 while developing. The production server serves
the built `dist/` directory itself.

## Test and build

```sh
npm run check
npm test
npm run build                  # exact frontend build command; outputs dist/
cargo test
cargo build --release --locked
docker build -t rubric-comment-queue .
```

Backend configuration is environment-only:

- `PORT` — HTTP port, default `8080`
- `DATABASE_URL` — SQLite URL, default `sqlite://data/rubric-comment-queue.db?mode=rwc`
- `FRONTEND_DIR` — built frontend directory, default `dist`
- `BILLING_API_BASE` — Sociobot billing base, default production API

Persist `/app/data` when running the container. `GET /health` includes the
build SHA. Logs are structured JSON and shutdown is graceful.

## Privacy and scope

The product does not score essays, detect plagiarism, profile students, or
generate comments. Use minimal labels such as initials or roster numbers.
Licenses are verified by Sociobot; payment details never touch this service.
See `/privacy` and `/terms` in the app for the full policies.

The researched scope is in `.factory/brief.json`; the original visual system
and generated-asset provenance are in `.factory/design.md`.

## License

MIT — see [LICENSE](LICENSE).
