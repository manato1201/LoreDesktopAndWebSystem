import { notFound } from "next/navigation";
import { PageHeader } from "@/components/PageHeader";
import { RepoTabs } from "@/components/RepoTabs";
import { getRepository } from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";

export default async function RepositoryLayout(
  props: LayoutProps<"/repositories/[slug]">,
) {
  const { slug } = await props.params;
  const cookie = await getSessionCookieHeader();
  const repository = await getRepository(slug, cookie);

  if (!repository) {
    notFound();
  }

  return (
    <>
      <PageHeader
        title={repository.name}
        subtitle={`${repository.organization} · ${repository.sizeLabel} · Updated ${repository.updatedAt}`}
      />
      <RepoTabs slug={repository.slug} />
      {props.children}
    </>
  );
}
