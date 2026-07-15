import Link from "next/link";
import { notFound } from "next/navigation";
import { AccessControlExplorer } from "@/components/AccessControlExplorer";
import { PageHeader } from "@/components/PageHeader";
import { getAccessEntries, getRepositories, getTree } from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";
import { directoriesOnly } from "@/lib/tree-utils";

export const metadata = { title: "Access Control · LoreHub" };

export default async function AccessControlPage(
  props: PageProps<"/access-control">,
) {
  const params = await props.searchParams;
  const rawRepo = Array.isArray(params.repo) ? params.repo[0] : params.repo;

  const cookie = await getSessionCookieHeader();
  const repositories = await getRepositories(cookie);
  const repository =
    repositories.find((repo) => repo.slug === rawRepo) ?? repositories[0];

  if (!repository) {
    notFound();
  }

  const [tree, entries] = await Promise.all([
    getTree(repository.slug, cookie),
    getAccessEntries(cookie),
  ]);
  if (!tree) {
    notFound();
  }
  const directories = directoriesOnly(tree);

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
        {repositories.map((repo) => {
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
        initialEntries={entries}
      />
    </>
  );
}
