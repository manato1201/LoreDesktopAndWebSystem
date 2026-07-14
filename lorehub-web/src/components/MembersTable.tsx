"use client";

import { useState } from "react";
import type { MemberRole, OrgMember } from "@/lib/types";
import { AuthorAvatar } from "./AuthorAvatar";

const ROLES: MemberRole[] = ["owner", "admin", "member"];

export function MembersTable({
  initialMembers,
}: {
  initialMembers: OrgMember[];
}) {
  const [members, setMembers] = useState(initialMembers);

  const setRole = (email: string, role: MemberRole) => {
    setMembers((prev) =>
      prev.map((member) =>
        member.email === email ? { ...member, role } : member,
      ),
    );
  };

  return (
    <div className="overflow-hidden rounded-comfortable bg-surface">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr className="border-b border-border/40 text-left text-xs text-text-secondary">
            <th className="px-4 py-3 font-semibold">Member</th>
            <th className="px-4 py-3 font-semibold">Role</th>
            <th className="px-4 py-3 font-semibold">Joined</th>
          </tr>
        </thead>
        <tbody>
          {members.map((member) => (
            <tr
              key={member.email}
              className="border-b border-border/40 last:border-0"
            >
              <td className="px-4 py-3">
                <div className="flex items-center gap-3">
                  <AuthorAvatar initials={member.initials} />
                  <div className="min-w-0">
                    <p className="truncate font-bold text-text-primary">
                      {member.name}
                    </p>
                    <p className="truncate text-xs text-text-secondary">
                      {member.email}
                    </p>
                  </div>
                </div>
              </td>
              <td className="px-4 py-3">
                <label className="sr-only" htmlFor={`role-${member.email}`}>
                  Role for {member.name}
                </label>
                <select
                  id={`role-${member.email}`}
                  value={member.role}
                  onChange={(event) =>
                    setRole(member.email, event.target.value as MemberRole)
                  }
                  disabled={member.role === "owner"}
                  className="rounded-standard bg-surface-interactive px-2 py-1.5 text-sm text-text-primary capitalize disabled:opacity-60"
                >
                  {ROLES.map((role) => (
                    <option key={role} value={role}>
                      {role}
                    </option>
                  ))}
                </select>
              </td>
              <td className="px-4 py-3 text-text-secondary">
                {member.joinedAt}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
