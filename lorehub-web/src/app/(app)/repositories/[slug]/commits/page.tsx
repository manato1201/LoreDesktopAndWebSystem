import { notFound } from "next/navigation";
import { BranchGraph } from "@/components/BranchGraph";
import { getBranches, getCommits } from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";

export default async function CommitsPage(
  props: PageProps<"/repositories/[slug]/commits">,
) {
  const { slug } = await props.params;
  const cookie = await getSessionCookieHeader();
  const [commits, branches] = await Promise.all([
    getCommits(slug, cookie),
    getBranches(slug, cookie),
  ]);

  if (!commits || !branches) {
    notFound();
  }

  return <BranchGraph repoSlug={slug} commits={commits} branches={branches} />;
}
