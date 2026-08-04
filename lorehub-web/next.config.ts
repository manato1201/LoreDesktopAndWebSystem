import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Produces a self-contained `.next/standalone` build (a pruned
  // `node_modules` + a `server.js` entrypoint) for the Docker image — see
  // Dockerfile and node_modules/next/dist/docs/01-app/03-api-reference/
  // 05-config/01-next-config-js/output.md. Without this, the runtime image
  // would need the full `node_modules` tree instead of just the files each
  // page actually traces to.
  output: "standalone",

  // Proxies browser-facing /api/* calls to the real lorehub-api backend
  // server-side, ONLY when API_INTERNAL_URL is set (e.g. a Vercel
  // deployment pointed at a backend on a different domain — local dev and
  // the Docker Compose setup leave this unset and are unaffected).
  //
  // Why this exists: lorehub-api's session cookie is scoped to whatever
  // domain actually issues the Set-Cookie response. When lorehub-web and
  // lorehub-api share an effective "site" (same hostname, any port — true
  // for local dev and for the Docker Compose setup, where the browser
  // reaches both through `localhost`), that domain is close enough to
  // shared that cookies just work. When they're on genuinely different
  // domains (e.g. lorehub-web on Vercel, lorehub-api behind a tunnel/other
  // host), a cookie set by the API's own domain is never sent back on
  // requests the browser makes to lorehub-web's domain — no CORS or
  // SameSite setting fixes this, because the cookie was never stored under
  // the frontend's domain in the first place.
  //
  // The fix is to make the browser never talk to the backend's real domain
  // directly: every browser-side call in src/lib/api.ts goes to a relative
  // `/api/...` path (see that file's `API_BASE`, which resolves to `""`
  // client-side whenever `NEXT_PUBLIC_API_URL` is left empty), landing on
  // this same origin, and this rewrite silently forwards it server-side to
  // the real backend named by `API_INTERNAL_URL`. The browser only ever
  // sees this origin's own Set-Cookie response, so the cookie is correctly
  // scoped here — solving the cross-domain problem without needing a
  // shared parent domain or switching off cookie-based auth.
  async rewrites() {
    const backend = process.env.API_INTERNAL_URL;
    if (!backend) return [];
    return [{ source: "/api/:path*", destination: `${backend}/api/:path*` }];
  },
};

export default nextConfig;
