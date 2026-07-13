"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  LockIcon,
  PullRequestIcon,
  RepositoryIcon,
  SettingsIcon,
} from "./icons";

const NAV_ITEMS = [
  { href: "/", label: "Repositories", icon: RepositoryIcon },
  { href: "/pulls", label: "Pull Requests", icon: PullRequestIcon },
  { href: "/access-control", label: "Access Control", icon: LockIcon },
  { href: "/settings", label: "Settings", icon: SettingsIcon },
] as const;

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="flex w-60 shrink-0 flex-col gap-6 bg-bg-base px-3 py-5">
      <Link href="/" className="flex items-center gap-2 px-2">
        <span className="text-lg font-bold tracking-tight text-accent">
          LoreHub
        </span>
      </Link>

      <nav aria-label="Primary" className="flex flex-col gap-1">
        {NAV_ITEMS.map(({ href, label, icon: Icon }) => {
          const isActive =
            href === "/" ? pathname === "/" : pathname.startsWith(href);

          return (
            <Link
              key={href}
              href={href}
              aria-current={isActive ? "page" : undefined}
              className={`flex items-center gap-3 rounded-standard px-3 py-2 text-sm transition-colors ${
                isActive
                  ? "bg-surface-interactive font-bold text-text-primary"
                  : "font-normal text-text-secondary hover:bg-surface hover:text-text-primary"
              }`}
            >
              <Icon className="size-5" />
              {label}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
