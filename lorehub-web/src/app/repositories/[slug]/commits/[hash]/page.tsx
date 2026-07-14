import { notFound } from "next/navigation";
import { AuthorAvatar } from "@/components/AuthorAvatar";
import { ChangedFileRow } from "@/components/ChangedFileRow";
import { mockCommits } from "@/lib/mock-commits";

export default async function CommitDetailPage(
  props: PageProps<"/repositories/[slug]/commits/[hash]">,
) {
  const { hash } = await props.params;
  const commit = mockCommits.find((c) => c.hash === hash);

  if (!commit) {
    notFound();
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-start gap-3 rounded-comfortable bg-surface p-5">
        <AuthorAvatar initials={commit.authorInitials} className="mt-0.5" />
        <div className="min-w-0 flex-1">
          <h2 className="text-lg font-bold text-text-primary">
            {commit.message}
          </h2>
          {commit.description && (
            <p className="mt-1 text-sm text-text-secondary">
              {commit.description}
            </p>
          )}
          <p className="mt-2 text-xs text-text-secondary">
            {commit.author} committed {commit.timestamp}
          </p>
        </div>
        <code className="shrink-0 rounded-subtle bg-surface-interactive px-2 py-1 text-xs text-text-secondary">
          {commit.hash}
        </code>
      </div>

      <div>
        <h3 className="mb-2 text-sm font-semibold text-text-secondary">
          {commit.changedFiles.length} file
          {commit.changedFiles.length === 1 ? "" : "s"} changed
        </h3>
        <div className="flex flex-col divide-y divide-border/40 rounded-comfortable bg-surface p-2">
          {commit.changedFiles.map((change) => (
            <ChangedFileRow key={change.path} change={change} />
          ))}
        </div>
      </div>
    </div>
  );
}
