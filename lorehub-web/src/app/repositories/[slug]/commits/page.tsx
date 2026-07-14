import { notFound } from "next/navigation";
import { CommitRow } from "@/components/CommitRow";
import { getCommits } from "@/lib/api";

export default async function CommitsPage(
  props: PageProps<"/repositories/[slug]/commits">,
) {
  const { slug } = await props.params;
  const commits = await getCommits(slug);

  if (!commits) {
    notFound();
  }

  return (
    <div className="flex flex-col gap-2">
      {commits.map((commit) => (
        <CommitRow key={commit.hash} repoSlug={slug} commit={commit} />
      ))}
    </div>
  );
}
