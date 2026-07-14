import { notFound } from "next/navigation";
import { PageHeader } from "@/components/PageHeader";
import { RepoTabs } from "@/components/RepoTabs";
import { getRepository } from "@/lib/api";

export default async function RepositoryLayout(
  props: LayoutProps<"/repositories/[slug]">,
) {
  const { slug } = await props.params;
  const repository = await getRepository(slug);

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
