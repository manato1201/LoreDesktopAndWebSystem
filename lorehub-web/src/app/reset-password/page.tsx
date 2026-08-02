"use client";

import { Suspense, useState } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { resetPassword } from "@/lib/api";

/**
 * Reads `token` via `useSearchParams`, which requires a `<Suspense>`
 * ancestor for static prerendering to succeed in this Next.js version (see
 * `node_modules/next/dist/docs/01-app/03-api-reference/04-functions/use-search-params.md`
 * — a static page calling it without one fails the production build). The
 * inner component below does the actual reading; the default export just
 * wraps it in the boundary.
 */
function ResetPasswordForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const token = searchParams.get("token");

  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  if (!token) {
    return (
      <div className="flex flex-col gap-4">
        <p className="text-sm text-text-primary">
          This link is missing its reset token. Request a new one below.
        </p>
        <Link href="/forgot-password" className="text-xs text-accent underline">
          Request a new reset link
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
    if (newPassword !== confirmPassword) {
      setError("New password and confirmation do not match.");
      return;
    }

    setSubmitting(true);
    const result = await resetPassword(token, newPassword);
    if (result.ok) {
      router.push("/login");
      router.refresh();
      return;
    }
    setError(result.error);
    setSubmitting(false);
  };

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
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
        className="rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
      >
        {submitting ? "Resetting…" : "Reset password"}
      </button>
    </form>
  );
}

export default function ResetPasswordPage() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-bg-base px-4">
      <div className="flex w-full max-w-sm flex-col gap-4 rounded-comfortable bg-surface p-8">
        <div>
          <h1 className="text-lg font-bold text-accent">LoreHub</h1>
          <p className="mt-1 text-sm text-text-secondary">
            Choose a new password
          </p>
        </div>

        <Suspense
          fallback={<p className="text-sm text-text-secondary">Loading…</p>}
        >
          <ResetPasswordForm />
        </Suspense>
      </div>
    </div>
  );
}
