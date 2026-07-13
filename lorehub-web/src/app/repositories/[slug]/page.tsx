import { notFound } from "next/navigation";
import { PageHeader } from "@/components/PageHeader";
import { RepositoryExplorer } from "@/components/RepositoryExplorer";
import { mockRepositories } from "@/lib/mock-data";
import { mockTree } from "@/lib/mock-tree";

export function generateStaticParams() {
  return mockRepositories.map((repo) => ({ slug: repo.slug }));
}

export default async function RepositoryDetailPage(
  props: PageProps<"/repositories/[slug]">,
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
      <RepositoryExplorer tree={mockTree} />
    </>
  );
}
