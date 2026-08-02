"use client";

import { useState } from "react";
import Link from "next/link";
import { forgotPassword } from "@/lib/api";

/**
 * Standalone, unauthenticated (see `src/proxy.ts`'s matcher — this route is
 * deliberately not in it, same reasoning as `/login`).
 *
 * The success state below is shown unconditionally once the request has
 * been sent, never gated on whether the email actually matched an account —
 * `forgotPassword()` always resolves the same way, mirroring the backend's
 * anti-enumeration design (see `forgot_password` in
 * `lorehub-api/src/handlers.rs`). Do not add a branch here that tries to
 * tell the two cases apart.
 */
export default function ForgotPasswordPage() {
  const [email, setEmail] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [sent, setSent] = useState(false);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    await forgotPassword(email);
    setSent(true);
    setSubmitting(false);
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg-base px-4">
      <div className="flex w-full max-w-sm flex-col gap-4 rounded-comfortable bg-surface p-8">
        <div>
          <h1 className="text-lg font-bold text-accent">LoreHub</h1>
          <p className="mt-1 text-sm text-text-secondary">
            Reset your password
          </p>
        </div>

        {sent ? (
          <p className="text-sm text-text-primary">
            If an account exists for that email, we&apos;ve sent a password
            reset link.
          </p>
        ) : (
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <label className="flex flex-col gap-1 text-sm text-text-secondary">
              Email
              <input
                type="email"
                required
                autoComplete="username"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
              />
            </label>

            <button
              type="submit"
              disabled={submitting}
              className="rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
            >
              {submitting ? "Sending…" : "Send reset link"}
            </button>
          </form>
        )}

        <Link href="/login" className="text-xs text-text-secondary underline">
          Back to sign in
        </Link>
      </div>
    </div>
  );
}
