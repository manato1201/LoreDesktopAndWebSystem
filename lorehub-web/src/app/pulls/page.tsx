import { PageHeader } from "@/components/PageHeader";
import { PlaceholderScreen } from "@/components/PlaceholderScreen";
import { PullRequestIcon } from "@/components/icons";

export const metadata = { title: "Pull Requests · LoreHub" };

export default function PullsPage() {
  return (
    <>
      <PageHeader title="Pull Requests" />
      <PlaceholderScreen
        icon={PullRequestIcon}
        title="Diff review is coming next"
        description="Text, image, and 3D-model diff review will land here. See LOREHUB_UI_SPEC.md §1 (screen 6) for the planned layout."
      />
    </>
  );
}
