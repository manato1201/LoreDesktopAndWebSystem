# Deployment

How to stand up `lorehub-api` and `lorehub-web` with Docker Compose.

## Prerequisites

- Docker
- Docker Compose (bundled with current Docker Desktop; `docker compose version` should work)

## Quick start

```bash
cp .env.example .env
# Optionally edit .env — fill in LOREHUB_SMTP_* if you want real invite/
# password-reset emails instead of log-only ones. See lorehub-api/.env.example
# and lorehub-web/.env.example for the full variable list.

docker compose up --build
```

This builds both images and starts:

- `lorehub-api` on `http://localhost:4000`
- `lorehub-web` on `http://localhost:3000`

A fresh SQLite database (`lorehub.db`) is seeded automatically on first boot
with the same demo dataset the app ships with — no separate migration step.
Log in with `aiko.tanaka@nebula.studio` / `lorehub` (or any other seeded
account) to confirm it's working.

Both services expose a container `HEALTHCHECK`; `docker compose ps` shows
`healthy` once each is actually serving traffic, not just running.

## Environment variables

Each component documents its own variables inline:

- [`lorehub-api/.env.example`](../lorehub-api/.env.example) — CORS origin,
  request body size cap, cookie security, SMTP.
- [`lorehub-web/.env.example`](../lorehub-web/.env.example) — the API base
  URL.

The root [`.env.example`](../.env.example) is what `docker-compose.yml`
itself actually reads; it's a minimal subset pointing at the two files
above for the full picture, not a duplicate of every comment.

One detail worth calling out explicitly: `NEXT_PUBLIC_API_URL` is baked into
`lorehub-web`'s client JS bundle at **build** time, not read at container
start. Changing it means rebuilding the image (`docker compose up --build`),
not just restarting the container.

## TLS / reverse proxy

This Compose setup serves plain HTTP on `localhost:3000` and `localhost:4000`
— that's fine for local use or testing, but **not** how to expose this to
the public internet. A real deployment needs a reverse proxy (Caddy, nginx,
Traefik, etc.) in front of it terminating TLS and forwarding to these two
ports.

This isn't optional polish: `lorehub-api` issues its session and refresh
cookies with the `Secure` attribute by default (see
`lorehub-api/src/auth.rs`'s `cookies_secure`), so a browser will silently
refuse to store them at all unless the entire path — browser to reverse
proxy — is genuinely HTTPS. **Do not** reach for
`LOREHUB_INSECURE_COOKIES=true` to work around this in production; that flag
exists purely for local plain-HTTP development and drops real cookie
security (see `lorehub-api/.env.example`). Terminate real TLS in front of
the stack instead.

When you add a reverse proxy, also update `LOREHUB_WEB_ORIGIN` and
`NEXT_PUBLIC_API_URL` to the real public HTTPS origins the browser will
actually use — both currently default to `localhost` addresses meant for
this single-host Compose setup.

## Observability

`lorehub-api` exposes `GET /metrics` — Prometheus text-exposition format,
covering a request counter and a latency histogram labeled by method,
matched route, and status code (see `lorehub-api/src/main.rs`'s
`metrics_layer_and_handle`). It's deliberately **not** under `/api` (the
convention Prometheus operators' scrape configs already expect) and
deliberately **unauthenticated** (a scraper carries no session cookie).
That second point cuts both ways: **do not** expose `/metrics` on the
public internet without a reverse-proxy rule restricting it to your
monitoring network — same caution as the TLS/cookie-security note above,
just for request-volume/latency data instead of session cookies.

A minimal Prometheus `scrape_configs` entry, assuming the reverse proxy
setup described above:

```yaml
scrape_configs:
  - job_name: lorehub-api
    metrics_path: /metrics
    static_configs:
      - targets: ["lorehub-api-internal-host:4000"]
```

Per-request access logs (method, path, status, latency) are emitted at INFO
via `tower_http`'s `TraceLayer`. By default they're human-readable, meant
for a developer watching a terminal. Set `LOREHUB_LOG_FORMAT=json` (see
`lorehub-api/.env.example`) to switch to one JSON object per line instead —
what a real log aggregator (Loki, CloudWatch, etc.) needs to parse fields
out of a line rather than scraping free-form text.

## Backup

The entire durable state of this application is one file:
`lorehub-api`'s `/data/lorehub.db` inside the named Docker volume created by
`docker-compose.yml` (`lorehub-db`). Per `lorehub-api/src/db.rs`'s design,
every piece of application data — repositories, commits, org members,
sessions, everything — lives in that single SQLite file's `kv_store` table.
Back up that file (or the volume) and you've backed up the whole database;
there is nothing else to capture.

```bash
# Example: copy the live database out of the named volume to the host.
docker compose cp lorehub-api:/data/lorehub.db ./lorehub.db.bak
```

Restoring is the reverse: stop the stack, put the backed-up file back at
the volume's `/data/lorehub.db`, and start it again.
