import Link from "next/link";
import type { PullRequest } from "@/lib/types";
import { AuthorAvatar } from "./AuthorAvatar";
import { PRStatusPill } from "./PRStatusPill";

export function PullRequestRow({ pullRequest }: { pullRequest: PullRequest }) {
  return (
    <Link
      href={`/pulls/${pullRequest.id}`}
      className="flex items-center gap-4 rounded-comfortable bg-surface p-4 transition-colors hover:bg-surface-elevated"
    >
      <PRStatusPill status={pullRequest.status} />

      <div className="min-w-0 flex-1">
        <p className="truncate font-bold text-text-primary">
          {pullRequest.title}
        </p>
        <p className="text-xs text-text-secondary">
          {pullRequest.repoName} #{pullRequest.id} · opened{" "}
          {pullRequest.createdAt} by {pullRequest.author}
        </p>
      </div>

      <AuthorAvatar initials={pullRequest.authorInitials} />
    </Link>
  );
}
