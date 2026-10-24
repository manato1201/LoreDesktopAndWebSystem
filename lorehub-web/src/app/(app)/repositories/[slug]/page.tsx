import { notFound } from "next/navigation";
import { RepositoryExplorer } from "@/components/RepositoryExplorer";
import { RepositorySummary } from "@/components/RepositorySummary";
import { getBranches, getCommits, getPullRequests, getTree } from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";

export default async function RepositoryCodePage(
  props: PageProps<"/repositories/[slug]">,
) {
  const { slug } = await props.params;
  const cookie = await getSessionCookieHeader();
  const [tree, commits, branches, openPrs, mergedPrs, closedPrs] =
    await Promise.all([
      getTree(slug, cookie),
      getCommits(slug, cookie),
      getBranches(slug, cookie),
      getPullRequests("open", cookie),
      getPullRequests("merged", cookie),
      getPullRequests("closed", cookie),
    ]);

  if (!tree) {
    notFound();
  }

  const pullRequestCount = [...openPrs, ...mergedPrs, ...closedPrs].filter(
    (pr) => pr.repoSlug === slug,
  ).length;

  return (
    <RepositoryExplorer
      repoSlug={slug}
      tree={tree}
      summary={
        <RepositorySummary
          tree={tree}
          commitCount={commits?.length ?? 0}
          branchCount={branches?.length ?? 0}
          pullRequestCount={pullRequestCount}
        />
      }
    />
  );
}
