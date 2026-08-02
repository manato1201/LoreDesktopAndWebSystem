# syntax=docker/dockerfile:1

# No `engines` field in package.json pinning a Node version, so this uses
# current Node 20 LTS across all stages.

# --- Deps --------------------------------------------------------------
FROM node:20-slim AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci

# --- Builder -------------------------------------------------------------
FROM node:20-slim AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .

# `NEXT_PUBLIC_API_URL` is inlined into the client JS bundle at *build* time
# (see src/lib/api.ts: `process.env.NEXT_PUBLIC_API_URL`), not read from the
# environment when the container starts — so it must be supplied as a build
# ARG here, not a runtime `ENV` on the final stage. The default below
# matches the source's own fallback (`http://localhost:4000`) so `docker
# build` with no `--build-arg` still produces a working local image.
ARG NEXT_PUBLIC_API_URL=http://localhost:4000
ENV NEXT_PUBLIC_API_URL=$NEXT_PUBLIC_API_URL

RUN npm run build

# --- Runtime -------------------------------------------------------------
FROM node:20-slim AS runner
WORKDIR /app

ENV NODE_ENV=production

RUN useradd -m lorehub

# Standalone output layout (see next.config.ts and Next's own docs):
# `.next/standalone` holds the pruned server + node_modules and `server.js`;
# `public/` and `.next/static` are deliberately excluded from `standalone`
# by Next (meant to be served by a CDN) and must be copied in by hand.
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

RUN chown -R lorehub:lorehub /app
USER lorehub

EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

# `/` redirects an unauthenticated visitor to `/login` (see
# src/app/(app)/layout.tsx's `redirect()`), which would make a healthcheck
# against `/` chase a 307 instead of getting a clean 200. `/login` itself is
# a public page reachable with no session, so it's a clean liveness signal.
#
# Neither `curl` nor `wget` is present in this `node:20-slim` image by
# default, and installing one just for this would mean an extra `apt-get`
# layer — Node 20 has a built-in `fetch`, and this stage is a Node runtime by
# definition, so a one-line `node -e` script is the cheapest correct probe
# available.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD node -e "fetch('http://127.0.0.1:3000/login').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"

CMD ["node", "server.js"]
