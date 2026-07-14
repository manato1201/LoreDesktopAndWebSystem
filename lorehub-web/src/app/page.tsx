import { PageHeader } from "@/components/PageHeader";
import { RepositoryBrowser } from "@/components/RepositoryBrowser";
import { getRepositories } from "@/lib/api";

export default async function Home() {
  const repositories = await getRepositories();

  return (
    <>
      <PageHeader
        title="Repositories"
        subtitle={`Nebula Studios · ${repositories.length} repositories`}
      />
      <RepositoryBrowser repositories={repositories} />
    </>
  );
}
