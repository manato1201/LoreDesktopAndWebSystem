import { notFound } from "next/navigation";
import { RepositorySettingsForm } from "@/components/RepositorySettingsForm";
import { getRepository } from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";

export default async function RepositorySettingsPage(
  props: PageProps<"/repositories/[slug]">,
) {
  const { slug } = await props.params;
  const cookie = await getSessionCookieHeader();
  const repository = await getRepository(slug, cookie);

  if (!repository) {
    notFound();
  }

  return <RepositorySettingsForm repository={repository} />;
}
