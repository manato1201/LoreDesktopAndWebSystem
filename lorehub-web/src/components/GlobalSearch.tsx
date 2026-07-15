"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { getPullRequests, getRepositories } from "@/lib/api";
import type { PullRequest, Repository } from "@/lib/types";
import { PRStatusPill } from "./PRStatusPill";
import { PullRequestIcon, RepositoryIcon, SearchIcon } from "./icons";

type SearchResult = {
  key: string;
  href: string;
  title: string;
  subtitle: string;
  group: "Repositories" | "Pull Requests";
  pr?: PullRequest;
};

const DEBOUNCE_MS = 200;
const MAX_PER_GROUP = 5;

export function GlobalSearch() {
  const router = useRouter();
  const containerRef = useRef<HTMLDivElement>(null);

  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const [repositories, setRepositories] = useState<Repository[] | null>(null);
  const [pullRequests, setPullRequests] = useState<PullRequest[] | null>(null);
  const [dataRequested, setDataRequested] = useState(false);

  useEffect(() => {
    const timer = setTimeout(
      () => setDebouncedQuery(query.trim().toLowerCase()),
      DEBOUNCE_MS,
    );
    return () => clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    function handleOutsideClick(event: MouseEvent) {
      if (!containerRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleOutsideClick);
    return () => document.removeEventListener("mousedown", handleOutsideClick);
  }, []);

  const ensureData = () => {
    if (dataRequested) return;
    setDataRequested(true);
    getRepositories().then(setRepositories);
    Promise.all([
      getPullRequests("open"),
      getPullRequests("merged"),
      getPullRequests("closed"),
    ]).then(([openPrs, merged, closed]) =>
      setPullRequests([...openPrs, ...merged, ...closed]),
    );
  };

  const results = useMemo<SearchResult[]>(() => {
    if (!debouncedQuery) return [];

    const repoResults: SearchResult[] = (repositories ?? [])
      .filter(
        (repo) =>
          repo.name.toLowerCase().includes(debouncedQuery) ||
          repo.description.toLowerCase().includes(debouncedQuery),
      )
      .slice(0, MAX_PER_GROUP)
      .map((repo) => ({
        key: `repo-${repo.slug}`,
        href: `/repositories/${repo.slug}`,
        title: repo.name,
        subtitle: repo.description,
        group: "Repositories" as const,
      }));

    const prResults: SearchResult[] = (pullRequests ?? [])
      .filter(
        (pr) =>
          pr.title.toLowerCase().includes(debouncedQuery) ||
          pr.description.toLowerCase().includes(debouncedQuery),
      )
      .slice(0, MAX_PER_GROUP)
      .map((pr) => ({
        key: `pr-${pr.id}`,
        href: `/pulls/${pr.id}`,
        title: pr.title,
        subtitle: `${pr.repoName} #${pr.id}`,
        group: "Pull Requests" as const,
        pr,
      }));

    return [...repoResults, ...prResults];
  }, [debouncedQuery, repositories, pullRequests]);

  // Derived rather than synced via an effect — `results` can change out
  // from under a stale `activeIndex` (new keystroke, async data arriving),
  // so clamp at render time instead of chasing it with setState.
  const activeIndexInBounds =
    results.length === 0 ? -1 : Math.min(activeIndex, results.length - 1);

  const navigateTo = (result: SearchResult) => {
    setOpen(false);
    setQuery("");
    router.push(result.href);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      setOpen(false);
      return;
    }
    if (!open || results.length === 0) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((activeIndexInBounds + 1) % results.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex(
        (activeIndexInBounds - 1 + results.length) % results.length,
      );
    } else if (event.key === "Enter") {
      event.preventDefault();
      const result = results[activeIndexInBounds];
      if (result) navigateTo(result);
    }
  };

  let groupCursor = -1;
  let lastGroup: SearchResult["group"] | null = null;

  return (
    <div ref={containerRef} className="relative">
      <label className="relative block">
        <span className="sr-only">Search repositories and pull requests</span>
        <SearchIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-secondary" />
        <input
          type="search"
          role="combobox"
          aria-expanded={open}
          aria-controls="global-search-listbox"
          aria-autocomplete="list"
          aria-activedescendant={
            open && results[activeIndexInBounds]
              ? `global-search-option-${results[activeIndexInBounds].key}`
              : undefined
          }
          value={query}
          onFocus={() => {
            ensureData();
            if (query.trim()) setOpen(true);
          }}
          onChange={(event) => {
            const value = event.target.value;
            setQuery(value);
            setActiveIndex(0);
            ensureData();
            setOpen(value.trim().length > 0);
          }}
          onKeyDown={handleKeyDown}
          placeholder="Search LoreHub..."
          className="w-full rounded-pill bg-surface-interactive py-2 pl-9 pr-3 text-sm text-text-primary placeholder:text-text-secondary focus-visible:outline-2 focus-visible:outline-accent"
        />
      </label>

      {open && (
        <div
          id="global-search-listbox"
          role="listbox"
          aria-label="Search results"
          className="absolute left-0 top-full z-[200] mt-2 w-80 max-w-[calc(100vw-2rem)] overflow-hidden rounded-comfortable bg-surface-elevated py-2 shadow-heavy"
        >
          {results.length === 0 ? (
            <p className="px-4 py-3 text-sm text-text-secondary">
              No results for &ldquo;{debouncedQuery}&rdquo;.
            </p>
          ) : (
            results.map((result) => {
              const showGroupLabel = result.group !== lastGroup;
              lastGroup = result.group;
              groupCursor += 1;
              const optionIndex = groupCursor;

              return (
                <div key={result.key}>
                  {showGroupLabel && (
                    <p className="px-4 pb-1 pt-2 text-[11px] font-bold uppercase tracking-wide text-text-secondary first:pt-0">
                      {result.group}
                    </p>
                  )}
                  <button
                    id={`global-search-option-${result.key}`}
                    type="button"
                    role="option"
                    aria-selected={optionIndex === activeIndexInBounds}
                    onMouseEnter={() => setActiveIndex(optionIndex)}
                    onClick={() => navigateTo(result)}
                    className={`flex w-full items-center gap-3 px-4 py-2 text-left transition-colors ${
                      optionIndex === activeIndexInBounds
                        ? "bg-surface-interactive"
                        : "hover:bg-surface-interactive"
                    }`}
                  >
                    {result.group === "Repositories" ? (
                      <RepositoryIcon className="size-4 shrink-0 text-text-secondary" />
                    ) : (
                      <PullRequestIcon className="size-4 shrink-0 text-text-secondary" />
                    )}
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-bold text-text-primary">
                        {result.title}
                      </span>
                      <span className="block truncate text-xs text-text-secondary">
                        {result.subtitle}
                      </span>
                    </span>
                    {result.pr && <PRStatusPill status={result.pr.status} />}
                  </button>
                </div>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}
