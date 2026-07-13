import { notFound } from "next/navigation";
import { PageHeader } from "@/components/PageHeader";
import { PlaceholderScreen } from "@/components/PlaceholderScreen";
import { RepositoryIcon } from "@/components/icons";
import { mockRepositories } from "@/lib/mock-data";

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
      <PlaceholderScreen
        icon={RepositoryIcon}
        title="File tree & chunk-streaming preview are coming next"
        description="Sparse workspace browsing and the lock-aware file tree will land here. See LOREHUB_UI_SPEC.md §1 (screen 2-3) for the planned layout."
      />
    </>
  );
}
