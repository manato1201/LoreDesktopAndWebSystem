"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

export function RepoTabs({ slug }: { slug: string }) {
  const pathname = usePathname();
  const base = `/repositories/${slug}`;
  const commitsHref = `${base}/commits`;
  const isCommits = pathname.startsWith(commitsHref);

  const tabs = [
    { href: base, label: "Code", active: !isCommits },
    { href: commitsHref, label: "Commits", active: isCommits },
  ];

  return (
    <div
      role="tablist"
      aria-label="Repository sections"
      className="mb-6 flex gap-1 border-b border-border/40"
    >
      {tabs.map((tab) => (
        <Link
          key={tab.href}
          href={tab.href}
          role="tab"
          aria-selected={tab.active}
          className={`border-b-2 px-3 py-2 text-sm transition-colors ${
            tab.active
              ? "border-accent font-bold text-text-primary"
              : "border-transparent font-normal text-text-secondary hover:text-text-primary"
          }`}
        >
          {tab.label}
        </Link>
      ))}
    </div>
  );
}
