import Link from "next/link";
import type { Repository } from "@/lib/types";
import { LockIcon, RepositoryIcon } from "./icons";

export function RepositoryCard({ repository }: { repository: Repository }) {
  return (
    <Link
      href={`/repositories/${repository.slug}`}
      className="group flex flex-col gap-3 rounded-comfortable bg-surface p-5 transition-colors hover:bg-surface-elevated"
    >
      <div className="flex items-center gap-2">
        <RepositoryIcon className="size-4 shrink-0 text-text-secondary" />
        <span className="truncate font-bold text-text-primary group-hover:underline">
          {repository.name}
        </span>
        <span className="ml-auto shrink-0 rounded-pill border border-border-light px-2 py-0.5 text-[11px] font-semibold uppercase tracking-wide text-text-secondary">
          {repository.visibility}
        </span>
      </div>

      <p className="line-clamp-2 text-sm text-text-secondary">
        {repository.description}
      </p>

      <div className="mt-auto flex items-center gap-4 pt-2 text-xs text-text-secondary">
        <span>Updated {repository.updatedAt}</span>
        <span>{repository.sizeLabel}</span>
        {repository.lockedFileCount > 0 && (
          <span
            className="ml-auto flex items-center gap-1 text-warning"
            title={`${repository.lockedFileCount} file(s) currently locked`}
          >
            <LockIcon className="size-3.5" />
            {repository.lockedFileCount}
          </span>
        )}
      </div>
    </Link>
  );
}
