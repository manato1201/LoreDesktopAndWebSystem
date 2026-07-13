"use client";

import { useMemo, useState } from "react";
import type { Repository } from "@/lib/types";
import { RepositoryCard } from "./RepositoryCard";
import { SearchIcon } from "./icons";

export function RepositoryBrowser({
  repositories,
}: {
  repositories: Repository[];
}) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return repositories;
    return repositories.filter(
      (repo) =>
        repo.name.toLowerCase().includes(q) ||
        repo.description.toLowerCase().includes(q),
    );
  }, [repositories, query]);

  return (
    <div className="flex flex-col gap-6">
      <label className="relative block max-w-md">
        <span className="sr-only">Search repositories</span>
        <SearchIcon className="pointer-events-none absolute left-4 top-1/2 size-4 -translate-y-1/2 text-text-secondary" />
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Find a repository..."
          className="w-full rounded-pill bg-surface-interactive py-3 pl-11 pr-4 text-sm text-text-primary placeholder:text-text-secondary focus-visible:outline-2 focus-visible:outline-accent"
        />
      </label>

      {filtered.length > 0 ? (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {filtered.map((repository) => (
            <RepositoryCard key={repository.slug} repository={repository} />
          ))}
        </div>
      ) : (
        <p className="text-sm text-text-secondary">
          No repositories match &ldquo;{query}&rdquo;.
        </p>
      )}
    </div>
  );
}
