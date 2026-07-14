import { notFound } from "next/navigation";
import Link from "next/link";
import { AuthorAvatar } from "@/components/AuthorAvatar";
import { CommentThread } from "@/components/CommentThread";
import { DiffFileViewer } from "@/components/DiffFileViewer";
import { PageHeader } from "@/components/PageHeader";
import { PRStatusPill } from "@/components/PRStatusPill";
import { getPullRequest } from "@/lib/api";

export default async function PullRequestDetailPage(
  props: PageProps<"/pulls/[id]">,
) {
  const { id } = await props.params;
  const pullRequest = await getPullRequest(id);

  if (!pullRequest) {
    notFound();
  }

  return (
    <>
      <PageHeader
        title={`${pullRequest.title} #${pullRequest.id}`}
        subtitle={
          <>
            <Link
              href={`/repositories/${pullRequest.repoSlug}`}
              className="text-text-secondary hover:text-text-primary hover:underline"
            >
              {pullRequest.repoName}
            </Link>{" "}
            · opened {pullRequest.createdAt} · updated {pullRequest.updatedAt}
          </>
        }
      />

      <div className="flex flex-col gap-6">
        <div className="flex items-start gap-3 rounded-comfortable bg-surface p-5">
          <PRStatusPill status={pullRequest.status} />
          <AuthorAvatar
            initials={pullRequest.authorInitials}
            className="mt-0.5"
          />
          <div className="min-w-0 flex-1">
            <p className="text-sm text-text-secondary">
              <span className="font-bold text-text-primary">
                {pullRequest.author}
              </span>{" "}
              wants to merge {pullRequest.changedFiles.length} file
              {pullRequest.changedFiles.length === 1 ? "" : "s"}
            </p>
            <p className="mt-2 text-sm text-text-primary">
              {pullRequest.description}
            </p>
          </div>
        </div>

        <div>
          <h3 className="mb-2 text-sm font-semibold text-text-secondary">
            {pullRequest.changedFiles.length} file
            {pullRequest.changedFiles.length === 1 ? "" : "s"} changed
          </h3>
          <div className="flex flex-col gap-4">
            {pullRequest.changedFiles.map((file) => (
              <DiffFileViewer key={file.path} file={file} />
            ))}
          </div>
        </div>

        <CommentThread
          pullRequestId={pullRequest.id}
          initialComments={pullRequest.comments}
        />
      </div>
    </>
  );
}
