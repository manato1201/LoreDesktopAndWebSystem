"use client";

import { createContext, useContext, useEffect, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { getCurrentUser } from "@/lib/api";
import type { OrgMember } from "@/lib/types";
import { Sidebar } from "./Sidebar";

const CurrentUserContext = createContext<OrgMember | null>(null);

export function useCurrentUser(): OrgMember | null {
  return useContext(CurrentUserContext);
}

/**
 * Client-side auth boundary: every route except /login requires a valid
 * session (checked against GET /api/auth/me). This only gates the app
 * shell — it does not, on its own, protect server-rendered data fetches,
 * which still call lorehub-api's public read endpoints directly.
 */
export function AuthGate({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const router = useRouter();
  const [status, setStatus] = useState<"loading" | "authed" | "guest">(
    "loading",
  );
  const [user, setUser] = useState<OrgMember | null>(null);

  useEffect(() => {
    if (pathname === "/login") {
      return;
    }

    let cancelled = false;
    getCurrentUser().then((current) => {
      if (cancelled) return;
      if (current) {
        setUser(current);
        setStatus("authed");
      } else {
        setStatus("guest");
        router.replace("/login");
      }
    });
    return () => {
      cancelled = true;
    };
  }, [pathname, router]);

  if (pathname === "/login") {
    return <>{children}</>;
  }

  if (status !== "authed") {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <p className="text-sm text-text-secondary">Loading…</p>
      </div>
    );
  }

  return (
    <CurrentUserContext.Provider value={user}>
      <div className="flex min-h-full">
        <Sidebar />
        <main className="min-w-0 flex-1 px-8 py-6">{children}</main>
      </div>
    </CurrentUserContext.Provider>
  );
}
