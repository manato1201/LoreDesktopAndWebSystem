import { notFound } from "next/navigation";
import { RepositoryExplorer } from "@/components/RepositoryExplorer";
import { getTree } from "@/lib/api";

export default async function RepositoryCodePage(
  props: PageProps<"/repositories/[slug]">,
) {
  const { slug } = await props.params;
  const tree = await getTree(slug);

  if (!tree) {
    notFound();
  }

  return <RepositoryExplorer repoSlug={slug} tree={tree} />;
}
