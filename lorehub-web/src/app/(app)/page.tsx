import { PageHeader } from "@/components/PageHeader";
import { RepositoryBrowser } from "@/components/RepositoryBrowser";
import { getRepositories } from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";

export default async function Home() {
  const cookie = await getSessionCookieHeader();
  const repositories = await getRepositories(cookie);

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
