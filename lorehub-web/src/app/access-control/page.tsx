import { PageHeader } from "@/components/PageHeader";
import { PlaceholderScreen } from "@/components/PlaceholderScreen";
import { LockIcon } from "@/components/icons";

export const metadata = { title: "Access Control · LoreHub" };

export default function AccessControlPage() {
  return (
    <>
      <PageHeader title="Access Control" />
      <PlaceholderScreen
        icon={LockIcon}
        title="Directory-level permissions are coming next"
        description="A tree + permission-matrix panel will land here. See LOREHUB_UI_SPEC.md §1 (screen 7) for the planned layout."
      />
    </>
  );
}
