"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { deleteRepository, updateRepository } from "@/lib/api";
import type { Repository } from "@/lib/types";

const VISIBILITIES: Repository["visibility"][] = [
  "private",
  "internal",
  "public",
];

export function RepositorySettingsForm({
  repository,
}: {
  repository: Repository;
}) {
  const router = useRouter();
  const [name, setName] = useState(repository.name);
  const [description, setDescription] = useState(repository.description);
  const [visibility, setVisibility] = useState(repository.visibility);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [confirmText, setConfirmText] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const handleSave = async (event: React.FormEvent) => {
    event.preventDefault();
    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      const updated = await updateRepository(repository.slug, {
        name,
        description,
        visibility,
      });
      if (!updated) {
        setError("Could not save changes. Try again.");
        return;
      }
      setSaved(true);
      router.refresh();
    } catch {
      setError("Could not save changes. Try again.");
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    setDeleting(true);
    setDeleteError(null);
    try {
      const ok = await deleteRepository(repository.slug);
      if (!ok) {
        setDeleteError("Could not delete repository. Try again.");
        setDeleting(false);
        return;
      }
      router.push("/");
      router.refresh();
    } catch {
      setDeleteError("Could not delete repository. Try again.");
      setDeleting(false);
    }
  };

  return (
    <div className="flex max-w-2xl flex-col gap-8">
      <form
        onSubmit={handleSave}
        className="flex flex-col gap-4 rounded-comfortable bg-surface p-5"
      >
        <h2 className="text-sm font-semibold text-text-secondary">General</h2>

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Name
          <input
            type="text"
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
            className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-accent"
          />
        </label>

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Description
          <textarea
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            rows={2}
            className="w-full resize-y rounded-comfortable bg-surface-interactive p-3 text-sm text-text-primary placeholder:text-text-secondary focus-visible:outline-2 focus-visible:outline-accent"
          />
        </label>

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Visibility
          <select
            value={visibility}
            onChange={(event) =>
              setVisibility(event.target.value as Repository["visibility"])
            }
            className="w-fit rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary capitalize"
          >
            {VISIBILITIES.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        </label>

        {error && <p className="text-xs text-negative">{error}</p>}
        {saved && !error && <p className="text-xs text-accent">Saved.</p>}

        <button
          type="submit"
          disabled={saving}
          className="self-end rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
        >
          {saving ? "Saving…" : "Save changes"}
        </button>
      </form>

      <div className="flex flex-col gap-4 rounded-comfortable border border-negative/40 bg-surface p-5">
        <h2 className="text-sm font-semibold text-negative">Danger zone</h2>
        <p className="text-sm text-text-secondary">
          Deleting{" "}
          <span className="font-bold text-text-primary">{repository.name}</span>{" "}
          removes it and its pull requests permanently. This cannot be undone.
        </p>

        <label className="flex flex-col gap-1 text-sm text-text-secondary">
          Type <code className="text-text-primary">{repository.slug}</code> to
          confirm
          <input
            type="text"
            value={confirmText}
            onChange={(event) => setConfirmText(event.target.value)}
            className="rounded-standard bg-surface-interactive px-3 py-2 text-sm text-text-primary focus-visible:outline-2 focus-visible:outline-negative"
          />
        </label>

        {deleteError && <p className="text-xs text-negative">{deleteError}</p>}

        <button
          type="button"
          onClick={handleDelete}
          disabled={confirmText !== repository.slug || deleting}
          className="self-end rounded-pill bg-negative px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
        >
          {deleting ? "Deleting…" : "Delete this repository"}
        </button>
      </div>
    </div>
  );
}
