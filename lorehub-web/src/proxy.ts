import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

const ACCESS_COOKIE = "lorehub_token";
const REFRESH_COOKIE = "lorehub_refresh";
const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000";

/**
 * Extracts the `name=value` pair from a raw `Set-Cookie` header string,
 * ignoring the trailing attributes (`Path=...; HttpOnly; ...`). Used to
 * fold lorehub-api's refresh response cookies into the *request* Cookie
 * header we forward downstream (see `proxy` below) — the response-side
 * `Set-Cookie` for the browser is forwarded verbatim instead, since that
 * one doesn't need reparsing.
 */
function cookieNameValue(setCookieStr: string): [string, string] | null {
  const eq = setCookieStr.indexOf("=");
  if (eq === -1) return null;
  const semi = setCookieStr.indexOf(";");
  const name = setCookieStr.slice(0, eq).trim();
  const value = setCookieStr
    .slice(eq + 1, semi === -1 ? undefined : semi)
    .trim();
  return [name, value];
}

/** Merges fresh `Set-Cookie` values into an existing `Cookie` request header string, overriding same-named entries. */
function mergeCookieHeader(
  existing: string | null,
  setCookies: string[],
): string {
  const jar = new Map<string, string>();
  if (existing) {
    for (const part of existing.split(";")) {
      const trimmed = part.trim();
      if (!trimmed) continue;
      const eq = trimmed.indexOf("=");
      if (eq === -1) continue;
      jar.set(trimmed.slice(0, eq), trimmed.slice(eq + 1));
    }
  }
  for (const setCookie of setCookies) {
    const parsed = cookieNameValue(setCookie);
    if (parsed) jar.set(parsed[0], parsed[1]);
  }
  return Array.from(jar.entries())
    .map(([name, value]) => `${name}=${value}`)
    .join("; ");
}

/**
 * Returns each individual `Set-Cookie` header value from a fetch Response.
 * Modern Node/undici `Headers` exposes `getSetCookie()` for exactly this
 * (a plain `.get("set-cookie")` folds multiple headers into one
 * comma-joined string, which is ambiguous to split back apart in general —
 * though not for our specific cookies, since none of them use `Expires`,
 * whose comma would collide with a naive split). Falls back to a
 * name-aware split if `getSetCookie` isn't available for some reason.
 */
function getSetCookies(headers: Headers): string[] {
  const withGetSetCookie = headers as Headers & {
    getSetCookie?: () => string[];
  };
  if (typeof withGetSetCookie.getSetCookie === "function") {
    return withGetSetCookie.getSetCookie();
  }
  const joined = headers.get("set-cookie");
  if (!joined) return [];
  return joined.split(/,(?=\s*[A-Za-z0-9_]+=)/).map((s) => s.trim());
}

/**
 * Proxy-side half of the transparent access-token refresh story (see
 * `src/lib/api.ts`'s `fetchWithRefresh` for the client-side half, which
 * handles a 401 hit directly by the browser).
 *
 * Server Components can only *read* cookies via `cookies()` (`next/headers`)
 * — they cannot set response cookies mid-render (see
 * `node_modules/next/dist/docs/01-app/03-api-reference/04-functions/cookies.md`,
 * "Setting cookies is not supported during Server Component rendering").
 * So a request that reaches a protected route with a missing/expired access
 * token but a still-valid refresh token would otherwise bounce straight to
 * `/login` via the `redirect()` in `(app)/layout.tsx`, even though the
 * user's session is actually still good. Proxy runs *before* that render
 * and has full read/write access to both the request and the response, so
 * it's the layer that can recover here: it calls lorehub-api's refresh
 * endpoint itself, forwards the fresh access token into the request that
 * continues to render (so the Server Component's `cookies()` sees it in
 * this same pass), and sets the new `Set-Cookie`s on the response so the
 * browser stores them for subsequent requests.
 *
 * Only triggers when the access cookie is entirely *absent* (the browser
 * evicted it past its `Max-Age`) — this is the cheap, reliable signal
 * available here, since the access token is an opaque random string with no
 * embedded expiry to inspect. An access token that's merely expired
 * server-side but not yet evicted client-side still reaches the API layer,
 * which 401s it; that case is instead handled by the CSR retry in
 * `src/lib/api.ts`, or — for a fresh SSR render — self-heals on the next
 * request once the browser drops the cookie at its `Max-Age`.
 */
export async function proxy(request: NextRequest) {
  const hasAccessToken = request.cookies.has(ACCESS_COOKIE);
  const refreshToken = request.cookies.get(REFRESH_COOKIE)?.value;

  if (hasAccessToken || !refreshToken) {
    return NextResponse.next();
  }

  let refreshRes: Response;
  try {
    refreshRes = await fetch(`${API_BASE}/api/auth/refresh`, {
      method: "POST",
      headers: { Cookie: `${REFRESH_COOKIE}=${refreshToken}` },
    });
  } catch {
    // lorehub-api unreachable — fall through to the normal unauthenticated
    // flow rather than failing the whole page request.
    return NextResponse.next();
  }

  if (!refreshRes.ok) {
    // Refresh token expired/invalid/rotated-away — nothing more Proxy can
    // do; `(app)/layout.tsx` will redirect to /login as usual.
    return NextResponse.next();
  }

  const setCookies = getSetCookies(refreshRes.headers);

  // Forward the new access (+ rotated refresh) token into *this* request's
  // render so Server Components see it immediately instead of only on the
  // next navigation.
  const forwardedHeaders = new Headers(request.headers);
  forwardedHeaders.set(
    "cookie",
    mergeCookieHeader(request.headers.get("cookie"), setCookies),
  );

  const response = NextResponse.next({
    request: { headers: forwardedHeaders },
  });

  // And set them on the outgoing response so the browser persists the
  // refreshed cookies for subsequent requests.
  for (const setCookie of setCookies) {
    response.headers.append("set-cookie", setCookie);
  }

  return response;
}

/**
 * Scoped to the same protected route group the app already gates behind
 * login — everything under the `(app)` route group in `src/app`
 * (`(app)/layout.tsx` is what redirects an unauthenticated visitor to
 * `/login`). `/login` itself lives outside that group and is intentionally
 * not matched here.
 */
export const config = {
  matcher: [
    "/",
    "/access-control/:path*",
    "/pulls/:path*",
    "/repositories/:path*",
    "/settings/:path*",
  ],
};
