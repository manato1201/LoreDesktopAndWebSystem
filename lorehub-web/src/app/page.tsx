import { PageHeader } from "@/components/PageHeader";
import { RepositoryBrowser } from "@/components/RepositoryBrowser";
import { mockRepositories } from "@/lib/mock-data";

export default function Home() {
  return (
    <>
      <PageHeader
        title="Repositories"
        subtitle="Nebula Studios · 6 repositories"
      />
      <RepositoryBrowser repositories={mockRepositories} />
    </>
  );
}
