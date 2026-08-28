# syntax=docker/dockerfile:1
FROM node:22-alpine AS frontend
WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci
COPY index.html tsconfig.json vite.config.ts svelte.config.js ./
COPY src ./src
COPY public ./public
RUN npm run build

FROM rust:1.88-alpine AS backend
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src/main.rs ./src/main.rs
ARG BUILD_SHA=unknown
ENV BUILD_SHA=$BUILD_SHA
RUN cargo build --release --locked

FROM alpine:3.22
RUN apk add --no-cache ca-certificates libgcc && addgroup -S app && adduser -S -G app app
WORKDIR /app
COPY --from=backend /build/target/release/rubric-comment-queue /usr/local/bin/rubric-comment-queue
COPY --from=frontend /build/dist ./dist
RUN mkdir -p /app/data && chown -R app:app /app
USER app
ENV PORT=8080 FRONTEND_DIR=/app/dist DATABASE_URL=sqlite:///app/data/rubric-comment-queue.db?mode=rwc
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 CMD wget -q -O /dev/null http://127.0.0.1:8080/health || exit 1
CMD ["rubric-comment-queue"]
