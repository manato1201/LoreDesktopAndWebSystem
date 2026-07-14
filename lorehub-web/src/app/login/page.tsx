"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { login } from "@/lib/api";

export default function LoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);

    const result = await login(email, password);
    if (result.ok) {
      router.replace("/");
      router.refresh();
    } else {
      setError(result.error);
      setSubmitting(false);
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg-base px-4">
      <form
        onSubmit={handleSubmit}
        className="flex w-full max-w-sm flex-col gap-4 rounded-comfortable bg-surface p-8"
      >
        <div>
          <h1 className="text-lg font-bold text-accent">LoreHub</h1>
          <p className="mt-1 text-sm text-text-secondary">
            Sign in to Nebula Studios
          </p>
        </div>

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

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Password
          <input
            type="password"
            required
            autoComplete="current-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
          />
        </label>

        {error && <p className="text-xs text-negative">{error}</p>}

        <button
          type="submit"
          disabled={submitting}
          className="rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
        >
          {submitting ? "Signing in…" : "Sign in"}
        </button>

        <p className="text-xs text-text-secondary">
          Demo account: aiko.tanaka@nebula.studio · password: lorehub
        </p>
      </form>
    </div>
  );
}
