"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { acceptInvite, getInvitePreview } from "@/lib/api";
import type { InvitePreview } from "@/lib/types";

/**
 * Same `useSearchParams`-needs-`<Suspense>` reasoning as
 * `src/app/reset-password/page.tsx` — see that file's comment.
 */
function AcceptInviteForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const token = searchParams.get("token");

  // No token at all means there's nothing to fetch — start straight at the
  // "invalid" state rather than "loading" so the effect below never needs to
  // set state for that case (it only runs, and only calls `setPreview`, when
  // there's an actual token to look up).
  const [preview, setPreview] = useState<InvitePreview | null | "loading">(
    token ? "loading" : null,
  );
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!token) return;
    getInvitePreview(token).then(setPreview);
  }, [token]);

  if (preview === "loading") {
    return <p className="text-sm text-text-secondary">Loading invite…</p>;
  }

  if (preview === null) {
    return (
      <div className="flex flex-col gap-4">
        <p className="text-sm text-text-primary">
          This invite is invalid or has expired.
        </p>
        <Link href="/login" className="text-xs text-accent underline">
          Back to sign in
        </Link>
      </div>
    );
  }

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);

    // Client-side sanity check only — the server independently re-validates
    // password length and the token's validity, since the client can't be
    // trusted to enforce either.
    if (password !== confirmPassword) {
      setError("Password and confirmation do not match.");
      return;
    }

    setSubmitting(true);
    // `token` is guaranteed non-null here: `preview` only resolves away from
    // "loading" once the effect above has run, and it only runs when
    // `token` is set.
    const result = await acceptInvite(token as string, password);
    if (result.ok) {
      router.replace("/");
      router.refresh();
      return;
    }
    setError(result.error);
    setSubmitting(false);
  };

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-text-primary">
        You&apos;ve been invited to join LoreHub as {preview.name} (
        {preview.email}), role: {preview.role}.
      </p>

      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Password
          <input
            type="password"
            required
            minLength={8}
            autoComplete="new-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
          />
        </label>

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Confirm password
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
          className="rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
        >
          {submitting ? "Joining…" : "Accept invite"}
        </button>
      </form>
    </div>
  );
}

export default function AcceptInvitePage() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-bg-base px-4">
      <div className="flex w-full max-w-sm flex-col gap-4 rounded-comfortable bg-surface p-8">
        <div>
          <h1 className="text-lg font-bold text-accent">LoreHub</h1>
          <p className="mt-1 text-sm text-text-secondary">Accept invite</p>
        </div>

        <Suspense
          fallback={<p className="text-sm text-text-secondary">Loading…</p>}
        >
          <AcceptInviteForm />
        </Suspense>
      </div>
    </div>
  );
}
