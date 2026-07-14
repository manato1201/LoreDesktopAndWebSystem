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

async function apiGet<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    cache: "no-store",
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`GET ${path} failed: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

async function apiGetOrNull<T>(path: string): Promise<T | null> {
  const res = await fetch(`${API_BASE}${path}`, {
    cache: "no-store",
    credentials: "include",
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

export function getRepositories(): Promise<Repository[]> {
  return apiGet("/api/repositories");
}

export function getRepository(slug: string): Promise<Repository | null> {
  return apiGetOrNull(`/api/repositories/${slug}`);
}

export function getTree(slug: string): Promise<TreeNode[] | null> {
  return apiGetOrNull(`/api/repositories/${slug}/tree`);
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

export function getCommits(slug: string): Promise<Commit[] | null> {
  return apiGetOrNull(`/api/repositories/${slug}/commits`);
}

export function getCommit(slug: string, hash: string): Promise<Commit | null> {
  return apiGetOrNull(`/api/repositories/${slug}/commits/${hash}`);
}

export function getBranches(slug: string): Promise<Branch[] | null> {
  return apiGetOrNull(`/api/repositories/${slug}/branches`);
}

export function getPullRequests(status: PRStatus): Promise<PullRequest[]> {
  return apiGet(`/api/pulls?status=${status}`);
}

export function getPullRequest(id: string): Promise<PullRequest | null> {
  return apiGetOrNull(`/api/pulls/${id}`);
}

export function addComment(
  id: string,
  body: string,
): Promise<PullRequest | null> {
  return apiSend("POST", `/api/pulls/${id}/comments`, { body });
}

export function getAccessEntries(): Promise<Record<string, AccessEntry[]>> {
  return apiGet("/api/access-control/entries");
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

export function getMembers(): Promise<OrgMember[]> {
  return apiGet("/api/org/members");
}

export function updateMemberRole(
  email: string,
  role: MemberRole,
): Promise<OrgMember | null> {
  return apiSend("PATCH", `/api/org/members/${encodeURIComponent(email)}`, {
    role,
  });
}

export function getStorage(): Promise<StorageUsage> {
  return apiGet("/api/org/storage");
}

export function getAuditLog(): Promise<AuditLogEntry[]> {
  return apiGet("/api/org/audit-log");
}

export async function getCurrentUser(): Promise<OrgMember | null> {
  const res = await fetch(`${API_BASE}/api/auth/me`, {
    cache: "no-store",
    credentials: "include",
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
