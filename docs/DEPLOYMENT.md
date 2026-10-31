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

## Different domains (Vercel + a tunnel)

![Where the session cookie lives: same host vs different domains](diagrams/deploy-cross-domain.svg)

With `docker compose` the browser talks to `localhost:3000` and `localhost:4000`
— the same host name, so the session cookie `lorehub-api` sets is sent to
both. Put the web app and the API on **different domains** (for example
Vercel for `lorehub-web` and a Cloudflare Quick Tunnel to your own PC for
`lorehub-api`) and that stops working: a cookie is stored only for the domain
that set it, so the browser never has one for the web app's domain, and no
CORS or `SameSite` setting can fix that.

The fix is that the browser never sees the API's domain. `next.config.ts`
rewrites `/api/*` on the **server** to `API_INTERNAL_URL`, so every
`Set-Cookie` comes from the web app's own origin.

### What to set

| Where | Variable | Value | Notes |
|---|---|---|---|
| Vercel (web) | `API_INTERNAL_URL` | the tunnel URL, e.g. `https://xxxx.trycloudflare.com` | Needed at **build** time: `rewrites()` is evaluated during `npm run build`, not at start. Change it and you must redeploy. |
| Vercel (web) | `NEXT_PUBLIC_USE_API_PROXY` | `true` | Tells the browser bundle to call relative `/api/...` paths. Spell the name exactly; a missing letter silently turns it off. |
| Your PC (API) | `LOREHUB_WEB_ORIGIN` | the Vercel URL, e.g. `https://my-app.vercel.app` | The single origin CORS allows. |
| Your PC (API) | `LOREHUB_INSECURE_COOKIES` | `true` — **only if** LoreForge Client / Server Admin connect to the same API | See below. |

```bash
# on your PC
cloudflared tunnel --url http://localhost:4000          # prints the https URL
LOREHUB_WEB_ORIGIN=https://my-app.vercel.app \
LOREHUB_INSECURE_COOKIES=true \
  ./lorehub-api
```

### Serving the web app and the desktop apps from one API

The desktop apps (LoreForge Client and Server Admin) connect to
`http://localhost:4000` over plain HTTP. By default the API marks its cookies
`Secure`, and a `Secure` cookie is **never stored over plain HTTP** — by a
browser or by Qt's network stack. The symptom in the desktop client is a
successful login followed by a repository list that fails with
**"Host requires authentication"** (Qt's wording for a bare HTTP 401: the
cookie was never kept, so the next request had none).

Setting `LOREHUB_INSECURE_COOKIES=true` drops the `Secure` attribute. That does
not break the HTTPS path: `Secure` only *restricts* a cookie to HTTPS, it does
not require HTTPS. So one API process can serve both, **as long as both
variables are set together** (`LOREHUB_WEB_ORIGIN` for the web app's CORS,
`LOREHUB_INSECURE_COOKIES` for the desktop apps). If you serve only the
public web app, leave `LOREHUB_INSECURE_COOKIES` unset.

### Things that went wrong in practice

| Symptom | Cause | How to check |
|---|---|---|
| Login button stays on "SIGNING IN…" | The deployed JS still calls the old absolute API URL, because the variable was not applied at build time | Download the deployed JS chunks and search for the URL, rather than trusting the dashboard |
| Same, after setting the variable | The name was mistyped (`…_PROX`) | Compare the name character by character |
| Login fails everywhere after a while | The tunnel or the API process was not running (a restart of `cloudflared` also gives a **new URL**) | `tasklist` / `ps`; then update `API_INTERNAL_URL` and redeploy |
| Desktop client: "Host requires authentication" | `Secure` cookie over plain HTTP (above) | `curl -i` the login and look for `Secure` in `Set-Cookie` |

A Quick Tunnel needs no account and costs nothing, but its URL changes every
time `cloudflared` restarts. That is fine for a prototype and not for anything
you hand to other people; use a named tunnel or a hosted API for that.

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

## Container isolation

Both services in `docker-compose.yml` run with `cap_drop: [ALL]` and
`security_opt: [no-new-privileges:true]`. Neither is privileged or mounts a
host path; both run as an unprivileged user on ports above 1024, and
`lorehub-api`'s only writable location is the named volume (`/data`). So there
is no Linux capability for a compromised process to use and no way to gain
privileges.

A read-only root filesystem (`read_only: true`) is deliberately **not** set
yet: `lorehub-web` (Next.js) may write to `.next/cache` at runtime, so it
needs a `tmpfs` mount and a test run before it is safe to turn on.

## Backup

The durable state of this application is **two things**, both inside the
named Docker volume `docker-compose.yml` mounts at `/data`
(`lorehub-db`):

- `/data/lorehub.db` — the SQLite database. Non-VCS data (org members,
  sessions, audit log, PR data, etc.) is stored here as JSON blobs; VCS data
  (repositories, commits, branches, tree snapshots, file metadata) is stored
  here as real normalized tables — see `lorehub-api/src/db.rs`'s module doc
  comment for exactly which is which.
- `/data/blobs/` — the content-addressed file store the VCS redesign
  introduced (`lorehub-api/src/blob_store.rs`). Every uploaded/committed
  file's actual bytes live here as `blobs/{repo_slug}/{hash[..2]}/{hash}`;
  `lorehub.db`'s `file_blobs`/`tree_entries` rows only hold hashes and
  metadata pointing into this directory, not the bytes themselves.

**Both are required for a complete backup or restore.** Backing up
`lorehub.db` alone and losing `blobs/` would leave every `file_blobs`/
`tree_entries` row pointing at content that no longer exists on disk —
commits and tree entries would still list correctly, but every file read
(content/image/audio) referencing that content would fail.

```bash
# Example: copy both the database and the blob store out of the named
# volume to the host (docker compose cp handles directories too).
docker compose cp lorehub-api:/data/lorehub.db ./lorehub.db.bak
docker compose cp lorehub-api:/data/blobs ./blobs.bak
```

Restoring is the reverse: stop the stack, put both `lorehub.db` and the
`blobs/` directory back at the volume's `/data/lorehub.db` and
`/data/blobs/`, and start it again. In practice it's simplest to back up
the whole named volume in one shot rather than the two paths separately —
e.g. `docker run --rm -v lorehub-db:/data -v "$(pwd)":/backup debian tar
czf /backup/lorehub-db.tar.gz -C /data .` — since that captures both by
construction.
