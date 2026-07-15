import { notFound } from "next/navigation";
import { RepositoryExplorer } from "@/components/RepositoryExplorer";
import { getTree } from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";

export default async function RepositoryCodePage(
  props: PageProps<"/repositories/[slug]">,
) {
  const { slug } = await props.params;
  const cookie = await getSessionCookieHeader();
  const tree = await getTree(slug, cookie);

  if (!tree) {
    notFound();
  }

  return <RepositoryExplorer repoSlug={slug} tree={tree} />;
}
