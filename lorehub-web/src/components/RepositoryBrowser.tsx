"use client";

import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { createRepository } from "@/lib/api";
import type { Repository } from "@/lib/types";
import { RepositoryCard } from "./RepositoryCard";
import { SearchIcon } from "./icons";

const VISIBILITIES: Repository["visibility"][] = [
  "private",
  "internal",
  "public",
];

export function RepositoryBrowser({
  repositories,
}: {
  repositories: Repository[];
}) {
  const router = useRouter();
  const [query, setQuery] = useState("");
  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [visibility, setVisibility] =
    useState<Repository["visibility"]>("private");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return repositories;
    return repositories.filter(
      (repo) =>
        repo.name.toLowerCase().includes(q) ||
        repo.description.toLowerCase().includes(q),
    );
  }, [repositories, query]);

  const handleCreate = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!name.trim()) {
      setError("Name is required.");
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      const created = await createRepository({ name, description, visibility });
      if (!created) {
        setError("Could not create repository. Try again.");
        setSubmitting(false);
        return;
      }
      router.push(`/repositories/${created.slug}`);
    } catch {
      setError("Could not create repository. Try again.");
      setSubmitting(false);
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center gap-3">
        <label className="relative block w-full max-w-md">
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

        <button
          type="button"
          onClick={() => {
            setShowForm((prev) => !prev);
            setError(null);
          }}
          aria-expanded={showForm}
          aria-controls="new-repository-form"
          className="ml-auto shrink-0 rounded-pill bg-accent px-5 py-2.5 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity hover:opacity-90"
        >
          {showForm ? "Cancel" : "New repository"}
        </button>
      </div>

      {showForm && (
        <form
          id="new-repository-form"
          onSubmit={handleCreate}
          className="flex flex-col gap-4 rounded-comfortable bg-surface p-5"
        >
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <label className="flex flex-col gap-1 text-sm text-text-secondary">
              Name
              <input
                type="text"
                required
                autoFocus
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="my-new-repo"
                className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
              />
            </label>

            <label className="flex flex-col gap-1 text-sm text-text-secondary">
              Visibility
              <select
                value={visibility}
                onChange={(event) =>
                  setVisibility(event.target.value as Repository["visibility"])
                }
                className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary capitalize"
              >
                {VISIBILITIES.map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            Description
            <textarea
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              rows={2}
              placeholder="What lives in this repository?"
              className="w-full resize-y rounded-comfortable bg-surface-interactive p-3 text-sm text-text-primary placeholder:text-text-secondary focus-visible:outline-2 focus-visible:outline-accent"
            />
          </label>

          {error && <p className="text-xs text-negative">{error}</p>}

          <button
            type="submit"
            disabled={submitting}
            className="self-end rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
          >
            {submitting ? "Creating…" : "Create repository"}
          </button>
        </form>
      )}

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
