import Link from "next/link";
import { AccessControlExplorer } from "@/components/AccessControlExplorer";
import { PageHeader } from "@/components/PageHeader";
import { mockAccessEntries } from "@/lib/mock-access-control";
import { mockRepositories } from "@/lib/mock-data";
import { mockTree } from "@/lib/mock-tree";
import { directoriesOnly } from "@/lib/tree-utils";

export const metadata = { title: "Access Control · LoreHub" };

export default async function AccessControlPage(
  props: PageProps<"/access-control">,
) {
  const params = await props.searchParams;
  const rawRepo = Array.isArray(params.repo) ? params.repo[0] : params.repo;
  const repository =
    mockRepositories.find((repo) => repo.slug === rawRepo) ??
    mockRepositories[0];

  const directories = directoriesOnly(mockTree);

  return (
    <>
      <PageHeader
        title="Access Control"
        subtitle="Directory-level permissions, per repository."
      />

      <div
        role="tablist"
        aria-label="Select repository"
        className="mb-6 flex flex-wrap gap-1 border-b border-border/40"
      >
        {mockRepositories.map((repo) => {
          const isActive = repo.slug === repository.slug;
          return (
            <Link
              key={repo.slug}
              href={`/access-control?repo=${repo.slug}`}
              role="tab"
              aria-selected={isActive}
              className={`border-b-2 px-3 py-2 text-sm transition-colors ${
                isActive
                  ? "border-accent font-bold text-text-primary"
                  : "border-transparent font-normal text-text-secondary hover:text-text-primary"
              }`}
            >
              {repo.name}
            </Link>
          );
        })}
      </div>

      <AccessControlExplorer
        key={repository.slug}
        directories={directories}
        initialEntries={mockAccessEntries}
      />
    </>
  );
}
