import type {
  AccessEntry,
  AuditLogEntry,
  Branch,
  Commit,
  MemberRole,
  OrgMember,
  PermissionLevel,
  PRStatus,
  PullRequest,
  Repository,
  StorageUsage,
  TreeNode,
} from "./types";

const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000";

/**
 * `cookie` is only needed when calling from a Server Component (see
 * src/lib/auth-server.ts) — Node has no ambient cookie jar. Client
 * Components omit it; the browser attaches the session cookie itself
 * because every call already sets `credentials: "include"`.
 */
async function apiGet<T>(path: string, cookie?: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    cache: "no-store",
    credentials: "include",
    headers: cookie ? { Cookie: cookie } : undefined,
  });
  if (!res.ok) {
    throw new Error(`GET ${path} failed: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

async function apiGetOrNull<T>(
  path: string,
  cookie?: string,
): Promise<T | null> {
  const res = await fetch(`${API_BASE}${path}`, {
    cache: "no-store",
    credentials: "include",
    headers: cookie ? { Cookie: cookie } : undefined,
  });
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
  const res = await fetch(`${API_BASE}${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    cache: "no-store",
    credentials: "include",
  });
  if (res.status === 404 || res.status === 401) return null;
  if (!res.ok) {
    throw new Error(`${method} ${path} failed: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

async function apiDelete(path: string): Promise<boolean> {
  const res = await fetch(`${API_BASE}${path}`, {
    method: "DELETE",
    cache: "no-store",
    credentials: "include",
  });
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

export async function getFileContent(
  slug: string,
  path: string,
): Promise<string | null> {
  const result = await apiGetOrNull<{ content: string }>(
    `/api/repositories/${slug}/content/${path}`,
  );
  return result?.content ?? null;
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

export async function logout(): Promise<void> {
  await fetch(`${API_BASE}/api/auth/logout`, {
    method: "POST",
    credentials: "include",
  });
}
