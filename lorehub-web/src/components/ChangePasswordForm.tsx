"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { changePassword } from "@/lib/api";

/**
 * Self-service "change my password" form for the Settings page — distinct
 * from `MembersTable` (org-admin-facing member management): this only ever
 * acts on the caller's own account, mirroring
 * `POST /api/auth/change-password` (see `lorehub-api/src/handlers.rs`).
 *
 * The server invalidates every session for the account on a successful
 * change, including the one that made this request, so on success we redirect
 * to `/login` rather than pretending the user is still signed in.
 */
export function ChangePasswordForm() {
  const router = useRouter();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);

    // Client-side sanity check only — the server re-validates length and
    // "different from current" independently, since the client can't be
    // trusted to enforce either.
    if (newPassword !== confirmPassword) {
      setError("New password and confirmation do not match.");
      return;
    }

    setSubmitting(true);
    const result = await changePassword(currentPassword, newPassword);
    if (result.ok) {
      router.push("/login");
      router.refresh();
      return;
    }
    setError(result.error);
    setSubmitting(false);
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="flex max-w-md flex-col gap-4 rounded-comfortable bg-surface p-5"
    >
      <label className="flex flex-col gap-1 text-sm text-text-secondary">
        Current password
        <input
          type="password"
          required
          autoComplete="current-password"
          value={currentPassword}
          onChange={(event) => setCurrentPassword(event.target.value)}
          className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
        />
      </label>

      <label className="flex flex-col gap-1 text-sm text-text-secondary">
        New password
        <input
          type="password"
          required
          minLength={8}
          autoComplete="new-password"
          value={newPassword}
          onChange={(event) => setNewPassword(event.target.value)}
          className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
        />
      </label>

      <label className="flex flex-col gap-1 text-sm text-text-secondary">
        Confirm new password
        <input
          type="password"
          required
          minLength={8}
          autoComplete="new-password"
          value={confirmPassword}
          onChange={(event) => setConfirmPassword(event.target.value)}
          className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
        />
      </label>

      {error && <p className="text-xs text-negative">{error}</p>}

      <button
        type="submit"
        disabled={submitting}
        className="self-end rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
      >
        {submitting ? "Changing…" : "Change password"}
      </button>
    </form>
  );
}
