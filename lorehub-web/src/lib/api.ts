import type {
  AccessEntry,
  AuditLogEntry,
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
  const res = await fetch(`${API_BASE}${path}`, { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`GET ${path} failed: ${res.status}`);
  }
  return res.json() as Promise<T>;
}

async function apiGetOrNull<T>(path: string): Promise<T | null> {
  const res = await fetch(`${API_BASE}${path}`, { cache: "no-store" });
  if (res.status === 404) return null;
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
  });
  if (res.status === 404) return null;
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

export function getCommits(slug: string): Promise<Commit[] | null> {
  return apiGetOrNull(`/api/repositories/${slug}/commits`);
}

export function getCommit(slug: string, hash: string): Promise<Commit | null> {
  return apiGetOrNull(`/api/repositories/${slug}/commits/${hash}`);
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
