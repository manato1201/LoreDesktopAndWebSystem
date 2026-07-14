import Link from "next/link";
import type { Commit } from "@/lib/types";
import { AuthorAvatar } from "./AuthorAvatar";

export function CommitRow({
  repoSlug,
  commit,
}: {
  repoSlug: string;
  commit: Commit;
}) {
  const fileCount = commit.changedFiles.length;

  return (
    <Link
      href={`/repositories/${repoSlug}/commits/${commit.hash}`}
      className="flex items-center gap-4 rounded-comfortable bg-surface p-4 transition-colors hover:bg-surface-elevated"
    >
      <AuthorAvatar initials={commit.authorInitials} />

      <div className="min-w-0 flex-1">
        <p className="truncate font-bold text-text-primary">{commit.message}</p>
        <p className="text-xs text-text-secondary">
          {commit.author} · {commit.timestamp} · {fileCount}{" "}
          {fileCount === 1 ? "file" : "files"} changed
        </p>
      </div>

      <code className="shrink-0 rounded-subtle bg-surface-interactive px-2 py-1 text-xs text-text-secondary">
        {commit.shortHash}
      </code>
    </Link>
  );
}
