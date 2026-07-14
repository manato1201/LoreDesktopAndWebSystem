import type { AuditLogEntry, OrgMember } from "./types";

export const mockMembers: OrgMember[] = [
  {
    name: "Aiko Tanaka",
    initials: "AT",
    email: "aiko.tanaka@nebula.studio",
    role: "owner",
    joinedAt: "Jan 2023",
  },
  {
    name: "Marco Silva",
    initials: "MS",
    email: "marco.silva@nebula.studio",
    role: "admin",
    joinedAt: "Mar 2023",
  },
  {
    name: "Priya Desai",
    initials: "PD",
    email: "priya.desai@nebula.studio",
    role: "member",
    joinedAt: "Aug 2023",
  },
  {
    name: "Diego Fernandez",
    initials: "DF",
    email: "diego.fernandez@nebula.studio",
    role: "member",
    joinedAt: "Nov 2024",
  },
];

export const mockStorageUsage = {
  usedLabel: "2.18 TB",
  totalLabel: "5 TB",
  usedPercent: 44,
};

export const mockAuditLog: AuditLogEntry[] = [
  {
    id: "a1",
    actor: "Aiko Tanaka",
    action: "locked",
    target: "Assets/Characters/hero_rig.fbx",
    timestamp: "2h ago",
  },
  {
    id: "a2",
    actor: "Marco Silva",
    action: "merged pull request #39 into",
    target: "hollow-keep-env",
    timestamp: "1d ago",
  },
  {
    id: "a3",
    actor: "Priya Desai",
    action: "updated permissions on",
    target: "Source",
    timestamp: "3d ago",
  },
  {
    id: "a4",
    actor: "Aiko Tanaka",
    action: "invited",
    target: "diego.fernandez@nebula.studio",
    timestamp: "8mo ago",
  },
];
