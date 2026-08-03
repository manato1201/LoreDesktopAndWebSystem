# syntax=docker/dockerfile:1

# --- Builder ---------------------------------------------------------------
# `rust:1-slim` always tracks the latest stable Rust release, which is
# required here: Cargo.toml declares `edition = "2024"` (stabilized in Rust
# 1.85), so anything older than that won't compile this crate.
FROM rust:1-slim AS builder

# sqlx is used with `features = ["runtime-tokio", "sqlite"]` (see
# Cargo.toml). sqlx's plain "sqlite" feature turns on `sqlx-sqlite/bundled`
# (see sqlx-sqlite's Cargo.toml: `sqlite = ["_sqlite", "sqlx-sqlite/bundled",
# ...]`), which vendors SQLite's amalgamation source and compiles it via the
# `cc` crate — there is no dependency on a system `libsqlite3`/pkg-config at
# all. `rust:*-slim` images ship without a C toolchain though, and `cc` still
# needs one to compile that vendored C source, so `build-essential` (gcc +
# make + friends) is the one OS package actually required here.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# `sqlx::migrate!("./migrations")` (see src/db.rs) is a compile-time macro:
# it embeds every migration file's contents into the binary via
# `include_str!`, resolved relative to `CARGO_MANIFEST_DIR` (i.e. this
# `WORKDIR`) at *build* time. That means `migrations/` must be present here,
# in the builder stage, before `cargo build` runs — the runtime stage below
# does NOT need its own copy, since the migrations are already baked into
# the compiled binary by the time it's copied over.
COPY migrations ./migrations
RUN cargo build --release

# --- Runtime -----------------------------------------------------------
# debian:bookworm-slim rather than distroless/scratch: lettre (see
# src/email.rs) opens a STARTTLS SMTP connection, which needs an up-to-date
# CA certificate bundle to validate the relay's TLS certificate, and a
# minimal shell/coreutils are handy for the HEALTHCHECK/operating the
# container at all.
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root user; `-m` also creates its home directory, though the binary
# itself doesn't need one.
RUN useradd -m lorehub

COPY --from=builder /build/target/release/lorehub-api /usr/local/bin/lorehub-api

# `main.rs` opens the database via `db::connect("lorehub.db")` — a bare
# relative filename, not a configurable path — so `lorehub.db` is created
# relative to the process's *current working directory* at container start.
# `WORKDIR /data` is therefore the one thing that makes persistence work:
# it's both where the binary runs from and the mount point a named volume
# (see docker-compose.yml) attaches to, so `lorehub.db` survives container
# restarts/recreates instead of being wiped with the container's writable
# layer. See docs/DEPLOYMENT.md for the operator-facing version of this.
WORKDIR /data
RUN chown lorehub:lorehub /data

USER lorehub

EXPOSE 4000

# GET /api/health (see src/handlers.rs::health) is a trivial always-200
# liveness probe that doesn't touch AppState/the database — exactly what a
# container HEALTHCHECK wants, since it should report "is the process alive
# and serving HTTP" rather than depend on backend state.
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://127.0.0.1:4000/api/health || exit 1

CMD ["lorehub-api"]
