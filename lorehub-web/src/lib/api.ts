import type {
  AccessEntry,
  AuditLogEntry,
  Branch,
  Commit,
  InvitePreview,
  MemberRole,
  OrgMember,
  PendingInvite,
  PermissionLevel,
  PRStatus,
  PullRequest,
  Repository,
  StorageUsage,
  TreeNode,
} from "./types";

/**
 * `NEXT_PUBLIC_API_URL` is inlined into the client bundle at build time and
 * is what the browser uses — correct for CSR fetches, but when this module
 * runs server-side (Server Components, Route Handlers) inside a container
 * (see docker-compose.yml), "the browser-reachable address" and "the
 * address this process can actually reach" are two different things: the
 * browser gets there via a published host port, while this same process
 * needs the Docker-internal service address. `API_INTERNAL_URL` (a plain,
 * non-`NEXT_PUBLIC_` env var — read live at container start, not baked into
 * any bundle) lets an operator override the server-side address without
 * needing a second image build; it's `undefined` in local (non-Docker) dev,
 * where both "the browser" and "this process" mean the same host and
 * `NEXT_PUBLIC_API_URL` is already correct for both.
 *
 * `NEXT_PUBLIC_USE_API_PROXY` opts the BROWSER side into relative `/api/...`
 * paths instead — needed whenever lorehub-web and lorehub-api sit on
 * genuinely different domains (see next.config.ts's `rewrites()`, which
 * proxies those relative calls server-side to `API_INTERNAL_URL`): without
 * this, the session cookie lorehub-api sets is scoped to the API's own
 * domain and is never sent back on requests the browser makes to
 * lorehub-web's domain, breaking login. This is a dedicated `"true"`/unset
 * flag rather than keying off `NEXT_PUBLIC_API_URL` being empty, since not
 * every hosting dashboard lets an operator save an env var as a genuinely
 * empty string (a blank field is easy to leave accidentally still holding
 * its old value) — an explicit flag can't be silently ignored that way.
 */
const USE_API_PROXY = process.env.NEXT_PUBLIC_USE_API_PROXY === "true";

const API_BASE =
  typeof window === "undefined"
    ? (process.env.API_INTERNAL_URL ??
      process.env.NEXT_PUBLIC_API_URL ??
      "http://localhost:4000")
    : USE_API_PROXY
      ? ""
      : (process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000");

/**
 * Client-side 401 recovery. When a browser-driven request comes back 401
 * (the access token expired), we transparently call this once before
 * falling back to each helper's normal 401 handling — the browser picks up
 * the new `Set-Cookie`s from the response automatically since this call
 * also uses `credentials: "include"`.
 *
 * Concurrent 401s (e.g. a page firing several `apiGet`s in parallel right
 * after the access token expires) share a single in-flight refresh instead
 * of each triggering their own — the second-and-later callers just await
 * the same promise.
 *
 * This is the CSR half of the refresh story; the SSR half (Server
 * Components, which can't act on a `Set-Cookie` mid-render) is handled by
 * `src/proxy.ts` instead — see that file.
 */
let refreshInFlight: Promise<boolean> | null = null;

function refreshAccessToken(): Promise<boolean> {
  if (!refreshInFlight) {
    refreshInFlight = fetch(`${API_BASE}/api/auth/refresh`, {
      method: "POST",
      credentials: "include",
    })
      .then((res) => res.ok)
      .catch(() => false)
      .finally(() => {
        refreshInFlight = null;
      });
  }
  return refreshInFlight;
}

/**
 * Runs `doFetch` and, on a 401 from a browser-driven call (no `cookie` —
 * that means Server Component/SSR, where this in-memory retry can't help
 * because there's no cookie jar and no way to relay a fresh `Set-Cookie`
 * back to the real client anyway), attempts exactly one silent
 * refresh-and-retry before handing the response back to the caller. If the
 * refresh call itself fails, or the retried request 401s again for some
 * other reason, the (possibly still-401) response is returned unchanged so
 * each helper's existing 401 handling stays the final fallback.
 */
export async function fetchWithRefresh(
  doFetch: () => Promise<Response>,
  cookie?: string,
): Promise<Response> {
  let res = await doFetch();
  if (res.status === 401 && !cookie) {
    const refreshed = await refreshAccessToken();
    if (refreshed) {
      res = await doFetch();
    }
  }
  return res;
}

/**
 * `cookie` is only needed when calling from a Server Component (see
 * src/lib/auth-server.ts) — Node has no ambient cookie jar. Client
 * Components omit it; the browser attaches the session cookie itself
 * because every call already sets `credentials: "include"`.
 */
async function apiGet<T>(path: string, cookie?: string): Promise<T> {
  const res = await fetchWithRefresh(
    () =>
      fetch(`${API_BASE}${path}`, {
        cache: "no-store",
        credentials: "include",
        headers: cookie ? { Cookie: cookie } : undefined,
      }),
    cookie,
  );
  if (!res.ok) {
    throw new Error(`GET ${path} failed: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

async function apiGetOrNull<T>(
  path: string,
  cookie?: string,
): Promise<T | null> {
  const res = await fetchWithRefresh(
    () =>
      fetch(`${API_BASE}${path}`, {
        cache: "no-store",
        credentials: "include",
        headers: cookie ? { Cookie: cookie } : undefined,
      }),
    cookie,
  );
  if (res.status === 404 || res.status === 401) return null;
  if (!res.ok) {
    throw new Error(`GET ${path} failed: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

async function apiSend<T>(
  method: "POST" | "PATCH",
  path: string,
  body: unknown,
): Promise<T | null> {
  const res = await fetchWithRefresh(() =>
    fetch(`${API_BASE}${path}`, {
      method,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      cache: "no-store",
      credentials: "include",
    }),
  );
  if (res.status === 404 || res.status === 401) return null;
  if (!res.ok) {
    throw new Error(`${method} ${path} failed: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

async function apiDelete(path: string): Promise<boolean> {
  const res = await fetchWithRefresh(() =>
    fetch(`${API_BASE}${path}`, {
      method: "DELETE",
      cache: "no-store",
      credentials: "include",
    }),
  );
  if (res.status === 404) return false;
  if (!res.ok) {
    throw new Error(`DELETE ${path} failed: ${res.status}`);
  }
  return true;
}

export function getRepositories(cookie?: string): Promise<Repository[]> {
  return apiGet("/api/repositories", cookie);
}

export function getRepository(
  slug: string,
  cookie?: string,
): Promise<Repository | null> {
  return apiGetOrNull(`/api/repositories/${slug}`, cookie);
}

export function createRepository(data: {
  name: string;
  description: string;
  visibility: Repository["visibility"];
}): Promise<Repository | null> {
  return apiSend("POST", "/api/repositories", data);
}

export function updateRepository(
  slug: string,
  data: Partial<{
    name: string;
    description: string;
    visibility: Repository["visibility"];
  }>,
): Promise<Repository | null> {
  return apiSend("PATCH", `/api/repositories/${slug}`, data);
}

export function deleteRepository(slug: string): Promise<boolean> {
  return apiDelete(`/api/repositories/${slug}`);
}

export function getTree(
  slug: string,
  cookie?: string,
): Promise<TreeNode[] | null> {
  return apiGetOrNull(`/api/repositories/${slug}/tree`, cookie);
}

export function toggleLock(
  slug: string,
  path: string,
  lock: boolean,
): Promise<TreeNode[] | null> {
  return apiSend("POST", `/api/repositories/${slug}/tree/lock`, { path, lock });
}

/**
 * `GET /api/repositories/:slug/content/:path` now returns the file's raw
 * text bytes directly (`Content-Type: text/plain`) rather than
 * JSON-wrapped `{ content: string }` — a byte range of a JSON document
 * would be meaningless to a text preview UI, so this endpoint dropped the
 * JSON envelope when it gained real HTTP Range support alongside the other
 * streamed-content endpoints. `apiGetOrNull`'s `.json()` parsing doesn't
 * apply here, so this reads the response body as text directly instead.
 */
export async function getFileContent(
  slug: string,
  path: string,
): Promise<string | null> {
  const apiPath = `/api/repositories/${slug}/content/${path}`;
  const res = await fetchWithRefresh(() =>
    fetch(`${API_BASE}${apiPath}`, {
      cache: "no-store",
      credentials: "include",
    }),
  );
  if (res.status === 404 || res.status === 401) return null;
  if (!res.ok) {
    throw new Error(`GET ${apiPath} failed: ${res.status}`);
  }
  return res.text();
}

export function imageUrl(slug: string, path: string): string {
  return `${API_BASE}/api/repositories/${slug}/image/${path}`;
}

export function imageBeforeUrl(slug: string, path: string): string {
  return `${API_BASE}/api/repositories/${slug}/image-before/${path}`;
}

export function audioUrl(slug: string, path: string): string {
  return `${API_BASE}/api/repositories/${slug}/audio/${path}`;
}

export function getCommits(
  slug: string,
  cookie?: string,
): Promise<Commit[] | null> {
  return apiGetOrNull(`/api/repositories/${slug}/commits`, cookie);
}

export function getCommit(
  slug: string,
  hash: string,
  cookie?: string,
): Promise<Commit | null> {
  return apiGetOrNull(`/api/repositories/${slug}/commits/${hash}`, cookie);
}

export function getBranches(
  slug: string,
  cookie?: string,
): Promise<Branch[] | null> {
  return apiGetOrNull(`/api/repositories/${slug}/branches`, cookie);
}

export function getPullRequests(
  status: PRStatus,
  cookie?: string,
): Promise<PullRequest[]> {
  return apiGet(`/api/pulls?status=${status}`, cookie);
}

export function getPullRequest(
  id: string,
  cookie?: string,
): Promise<PullRequest | null> {
  return apiGetOrNull(`/api/pulls/${id}`, cookie);
}

export function addComment(
  id: string,
  body: string,
): Promise<PullRequest | null> {
  return apiSend("POST", `/api/pulls/${id}/comments`, { body });
}

export function getAccessEntries(
  cookie?: string,
): Promise<Record<string, AccessEntry[]>> {
  return apiGet("/api/access-control/entries", cookie);
}

export function togglePermission(
  path: string,
  principal: string,
  level: PermissionLevel,
): Promise<AccessEntry[] | null> {
  return apiSend("POST", "/api/access-control/entries/toggle", {
    path,
    principal,
    level,
  });
}

export function getMembers(cookie?: string): Promise<OrgMember[]> {
  return apiGet("/api/org/members", cookie);
}

export function updateMemberRole(
  email: string,
  role: MemberRole,
): Promise<OrgMember | null> {
  return apiSend("PATCH", `/api/org/members/${encodeURIComponent(email)}`, {
    role,
  });
}

export function getStorage(cookie?: string): Promise<StorageUsage> {
  return apiGet("/api/org/storage", cookie);
}

export function getAuditLog(cookie?: string): Promise<AuditLogEntry[]> {
  return apiGet("/api/org/audit-log", cookie);
}

export async function getCurrentUser(
  cookie?: string,
): Promise<OrgMember | null> {
  const res = await fetch(`${API_BASE}/api/auth/me`, {
    cache: "no-store",
    credentials: "include",
    headers: cookie ? { Cookie: cookie } : undefined,
  });
  if (!res.ok) return null;
  return res.json() as Promise<OrgMember>;
}

export async function login(
  email: string,
  password: string,
): Promise<{ ok: true; user: OrgMember } | { ok: false; error: string }> {
  const res = await fetch(`${API_BASE}/api/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
    credentials: "include",
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}) as { error?: string });
    return { ok: false, error: body.error ?? "Login failed" };
  }
  const data = (await res.json()) as { user: OrgMember };
  return { ok: true, user: data.user };
}

/**
 * Self-service password rotation for the current user
 * (`POST /api/auth/change-password`). Uses `fetchWithRefresh` like the other
 * authenticated helpers so a merely-expired access token gets silently
 * refreshed and retried rather than being mistaken for "wrong current
 * password" — both cases would otherwise be indistinguishable 401s.
 *
 * Success is `204 No Content` (no JSON body); the server has just
 * invalidated every session for this account, including the one making this
 * call, so the caller is expected to redirect to `/login` afterwards rather
 * than trying to keep using the now-dead session.
 */
export async function changePassword(
  currentPassword: string,
  newPassword: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const res = await fetchWithRefresh(() =>
    fetch(`${API_BASE}/api/auth/change-password`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ currentPassword, newPassword }),
      cache: "no-store",
      credentials: "include",
    }),
  );
  if (res.status === 204) {
    return { ok: true };
  }
  const body = await res.json().catch(() => ({}) as { error?: string });
  return { ok: false, error: body.error ?? "Could not change password" };
}

/**
 * Kicks off the forgot-password flow (`POST /api/auth/forgot-password`).
 * The endpoint always responds `204` whether or not the email belongs to an
 * account — that's deliberate anti-enumeration behavior on the server (see
 * `forgot_password` in `lorehub-api/src/handlers.rs`), so this helper
 * doesn't return anything for the caller to branch on. A network-level
 * failure is swallowed the same way: surfacing "that failed, try again" only
 * for a bad connection while silently succeeding for a nonexistent email
 * would itself leak which case happened, so both paths end at the same
 * generic "check your email" UI state.
 */
export async function forgotPassword(email: string): Promise<void> {
  try {
    await fetch(`${API_BASE}/api/auth/forgot-password`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email }),
      credentials: "include",
    });
  } catch {
    // Swallowed — see doc comment above.
  }
}

/**
 * Completes the forgot-password flow (`POST /api/auth/reset-password`).
 * Unauthenticated (the caller has no session yet), so this is a plain
 * `fetch` rather than one of the `fetchWithRefresh`-based helpers. Unlike
 * `forgotPassword`, the outcome here is meaningful to show the user — a
 * missing/expired/already-used token or an under-length password both
 * produce a descriptive error body worth surfacing directly.
 */
export async function resetPassword(
  token: string,
  newPassword: string,
): Promise<{ ok: true } | { ok: false; error: string }> {
  const res = await fetch(`${API_BASE}/api/auth/reset-password`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ token, newPassword }),
    credentials: "include",
  });
  if (res.status === 204) {
    return { ok: true };
  }
  const body = await res.json().catch(() => ({}) as { error?: string });
  return { ok: false, error: body.error ?? "Could not reset password" };
}

/**
 * Looks up who an invite token is for (`GET /api/auth/invite/{token}`) so
 * the accept-invite page can show "you've been invited as ..." context
 * before asking for a password. Public/unauthenticated endpoint; `null`
 * covers every failure mode (missing, expired, already-used, or malformed
 * token) since the accept-invite UI treats them all identically.
 */
export async function getInvitePreview(
  token: string,
): Promise<InvitePreview | null> {
  const res = await fetch(
    `${API_BASE}/api/auth/invite/${encodeURIComponent(token)}`,
    { cache: "no-store", credentials: "include" },
  );
  if (!res.ok) return null;
  return res.json() as Promise<InvitePreview>;
}

/**
 * Redeems an invite token and sets a password (`POST
 * /api/auth/accept-invite`). Mirrors `login()`'s shape exactly: on success
 * the server also sets the session/refresh cookies, so the caller ends up
 * auto-logged-in and can navigate straight into the app rather than back to
 * `/login`.
 */
export async function acceptInvite(
  token: string,
  password: string,
): Promise<{ ok: true; user: OrgMember } | { ok: false; error: string }> {
  const res = await fetch(`${API_BASE}/api/auth/accept-invite`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ token, password }),
    credentials: "include",
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}) as { error?: string });
    return { ok: false, error: body.error ?? "Could not accept invite" };
  }
  const data = (await res.json()) as { user: OrgMember };
  return { ok: true, user: data.user };
}

/**
 * Owner/Admin-only (`POST /api/org/invites`). The list endpoint
 * (`listInvites` below) never re-exposes the secret `inviteUrl` once
 * created, so this is the one moment the caller can show/copy it — worth
 * returning even though the rest of the created record isn't, since the
 * Settings UI already has the email/name/role/teams it just submitted.
 */
export async function createInvite(
  email: string,
  name: string,
  role: MemberRole,
  teams: string[],
): Promise<{ ok: true; inviteUrl: string } | { ok: false; error: string }> {
  const res = await fetchWithRefresh(() =>
    fetch(`${API_BASE}/api/org/invites`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, name, role, teams }),
      cache: "no-store",
      credentials: "include",
    }),
  );
  if (res.status === 201) {
    const data = (await res.json()) as { inviteUrl: string };
    return { ok: true, inviteUrl: data.inviteUrl };
  }
  const body = await res.json().catch(() => ({}) as { error?: string });
  return { ok: false, error: body.error ?? "Could not create invite" };
}

export function listInvites(cookie?: string): Promise<PendingInvite[]> {
  return apiGet("/api/org/invites", cookie);
}

/** Owner/Admin-only (`DELETE /api/org/invites/{email}`); always `204`, idempotent. */
export async function revokeInvite(email: string): Promise<void> {
  await apiDelete(`/api/org/invites/${encodeURIComponent(email)}`);
}

export async function logout(): Promise<void> {
  await fetch(`${API_BASE}/api/auth/logout`, {
    method: "POST",
    credentials: "include",
  });
}
