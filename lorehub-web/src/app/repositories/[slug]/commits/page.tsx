import { CommitRow } from "@/components/CommitRow";
import { mockCommits } from "@/lib/mock-commits";

export default async function CommitsPage(
  props: PageProps<"/repositories/[slug]/commits">,
) {
  const { slug } = await props.params;

  return (
    <div className="flex flex-col gap-2">
      {mockCommits.map((commit) => (
        <CommitRow key={commit.hash} repoSlug={slug} commit={commit} />
      ))}
    </div>
  );
}
