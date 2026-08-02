"use client";

import { useState } from "react";
import { createInvite, revokeInvite } from "@/lib/api";
import type { MemberRole, PendingInvite } from "@/lib/types";

const ROLES: MemberRole[] = ["owner", "admin", "member"];

const SEVEN_DAYS_SECONDS = 7 * 24 * 60 * 60;

/**
 * Owner/Admin-only section of Settings — the caller (`(app)/settings/page.tsx`)
 * is responsible for only rendering this for a viewer whose role is
 * `"owner"` or `"admin"`, since the backend 403s everything here otherwise.
 *
 * Combines invite creation and the pending-invites list in one component
 * because they share state: a successful create appends straight to the
 * visible list instead of refetching `GET /api/org/invites`. `createInvite`
 * only returns `inviteUrl` (see `src/lib/api.ts` — the list endpoint never
 * re-exposes that secret, so it's only available at creation time), so the
 * rest of the `PendingInvite` shape is constructed optimistically from the
 * form fields plus a locally-computed `expiresAt`/`invitedBy` — close enough
 * for display purposes, and any drift self-heals on the next real page load.
 */
export function InviteMembersPanel({
  initialInvites,
}: {
  initialInvites: PendingInvite[];
}) {
  const [invites, setInvites] = useState(initialInvites);

  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [role, setRole] = useState<MemberRole>("member");
  const [teamsInput, setTeamsInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [lastInviteUrl, setLastInviteUrl] = useState<string | null>(null);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);

    const teams = teamsInput
      .split(",")
      .map((team) => team.trim())
      .filter((team) => team.length > 0);

    setSubmitting(true);
    const result = await createInvite(email, name, role, teams);
    if (result.ok) {
      // The server overwrites any prior pending invite for the same email
      // (see `create_invite` in `lorehub-api/src/handlers.rs`) — drop any
      // existing local row for it first so re-inviting doesn't show two
      // entries for the same address until the next real page load.
      setInvites((prev) => [
        ...prev.filter((invite) => invite.email !== email),
        {
          email,
          name,
          role,
          teams,
          invitedBy: "you",
          expiresAt: Date.now() / 1000 + SEVEN_DAYS_SECONDS,
        },
      ]);
      setLastInviteUrl(result.inviteUrl);
      setEmail("");
      setName("");
      setRole("member");
      setTeamsInput("");
    } else {
      setError(result.error);
    }
    setSubmitting(false);
  };

  const handleRevoke = async (revokeEmail: string) => {
    await revokeInvite(revokeEmail);
    setInvites((prev) => prev.filter((invite) => invite.email !== revokeEmail));
  };

  return (
    <div className="flex flex-col gap-4">
      <form
        onSubmit={handleSubmit}
        className="flex max-w-md flex-col gap-4 rounded-comfortable bg-surface p-5"
      >
        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Email
          <input
            type="email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
          />
        </label>

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Name
          <input
            type="text"
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
            className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
          />
        </label>

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Role
          <select
            value={role}
            onChange={(event) => setRole(event.target.value as MemberRole)}
            className="rounded-standard bg-surface-interactive px-2 py-1.5 text-sm text-text-primary capitalize"
          >
            {ROLES.map((r) => (
              <option key={r} value={r}>
                {r}
              </option>
            ))}
          </select>
        </label>

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Teams (comma-separated)
          <input
            type="text"
            value={teamsInput}
            onChange={(event) => setTeamsInput(event.target.value)}
            placeholder="art, narrative"
            className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
          />
        </label>

        {error && <p className="text-xs text-negative">{error}</p>}

        <button
          type="submit"
          disabled={submitting}
          className="self-end rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
        >
          {submitting ? "Inviting…" : "Send invite"}
        </button>
      </form>

      {lastInviteUrl && (
        <div className="flex max-w-md items-start justify-between gap-3 rounded-comfortable bg-surface p-4 text-xs text-text-secondary">
          <p>
            Invite created — link:{" "}
            <span className="break-all text-text-primary">
              {lastInviteUrl}
            </span>
          </p>
          <button
            type="button"
            onClick={() => setLastInviteUrl(null)}
            className="shrink-0 text-text-secondary underline"
          >
            Dismiss
          </button>
        </div>
      )}

      <div className="overflow-hidden rounded-comfortable bg-surface">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="border-b border-border/40 text-left text-xs text-text-secondary">
              <th className="px-4 py-3 font-semibold">Invitee</th>
              <th className="px-4 py-3 font-semibold">Role</th>
              <th className="px-4 py-3 font-semibold">Teams</th>
              <th className="px-4 py-3 font-semibold">Invited by</th>
              <th className="px-4 py-3 font-semibold">Expires</th>
              <th className="px-4 py-3" />
            </tr>
          </thead>
          <tbody>
            {invites.map((invite) => (
              <tr
                key={invite.email}
                className="border-b border-border/40 last:border-0"
              >
                <td className="px-4 py-3">
                  <div className="min-w-0">
                    <p className="truncate font-bold text-text-primary">
                      {invite.name}
                    </p>
                    <p className="truncate text-xs text-text-secondary">
                      {invite.email}
                    </p>
                  </div>
                </td>
                <td className="px-4 py-3 capitalize text-text-primary">
                  {invite.role}
                </td>
                <td className="px-4 py-3 text-text-secondary">
                  {invite.teams.join(", ") || "—"}
                </td>
                <td className="px-4 py-3 text-text-secondary">
                  {invite.invitedBy}
                </td>
                <td className="px-4 py-3 text-text-secondary">
                  {new Date(invite.expiresAt * 1000).toLocaleDateString()}
                </td>
                <td className="px-4 py-3 text-right">
                  <button
                    type="button"
                    onClick={() => handleRevoke(invite.email)}
                    className="rounded-standard px-2 py-1 text-xs text-negative hover:bg-surface-interactive"
                  >
                    Revoke
                  </button>
                </td>
              </tr>
            ))}
            {invites.length === 0 && (
              <tr>
                <td
                  colSpan={6}
                  className="px-4 py-3 text-center text-text-secondary"
                >
                  No pending invites.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
