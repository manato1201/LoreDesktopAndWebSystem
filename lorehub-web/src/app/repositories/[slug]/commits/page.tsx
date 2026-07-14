import { notFound } from "next/navigation";
import { BranchGraph } from "@/components/BranchGraph";
import { getBranches, getCommits } from "@/lib/api";

export default async function CommitsPage(
  props: PageProps<"/repositories/[slug]/commits">,
) {
  const { slug } = await props.params;
  const [commits, branches] = await Promise.all([
    getCommits(slug),
    getBranches(slug),
  ]);

  if (!commits || !branches) {
    notFound();
  }

  return <BranchGraph repoSlug={slug} commits={commits} branches={branches} />;
}
