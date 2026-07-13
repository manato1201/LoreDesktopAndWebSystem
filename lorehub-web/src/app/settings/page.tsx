import { PageHeader } from "@/components/PageHeader";
import { PlaceholderScreen } from "@/components/PlaceholderScreen";
import { SettingsIcon } from "@/components/icons";

export const metadata = { title: "Settings · LoreHub" };

export default function SettingsPage() {
  return (
    <>
      <PageHeader title="Organization Settings" />
      <PlaceholderScreen
        icon={SettingsIcon}
        title="Member management is coming next"
        description="Member roles, storage usage, and audit log will land here. See LOREHUB_UI_SPEC.md §1 (screen 8) for the planned layout."
      />
    </>
  );
}
