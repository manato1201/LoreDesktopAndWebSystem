import { notFound } from "next/navigation";
import { PageHeader } from "@/components/PageHeader";
import { RepoTabs } from "@/components/RepoTabs";
import { mockRepositories } from "@/lib/mock-data";

export function generateStaticParams() {
  return mockRepositories.map((repo) => ({ slug: repo.slug }));
}

export default async function RepositoryLayout(
  props: LayoutProps<"/repositories/[slug]">,
) {
  const { slug } = await props.params;
  const repository = mockRepositories.find((repo) => repo.slug === slug);

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
