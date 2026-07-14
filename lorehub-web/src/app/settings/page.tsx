import { AuditLogList } from "@/components/AuditLogList";
import { MembersTable } from "@/components/MembersTable";
import { PageHeader } from "@/components/PageHeader";
import { StorageUsageCard } from "@/components/StorageUsageCard";
import { mockAuditLog, mockMembers, mockStorageUsage } from "@/lib/mock-org";

export const metadata = { title: "Settings · LoreHub" };

export default function SettingsPage() {
  return (
    <>
      <PageHeader title="Organization Settings" subtitle="Nebula Studios" />

      <div className="flex flex-col gap-8">
        <section>
          <h2 className="mb-3 text-sm font-semibold text-text-secondary">
            Members
          </h2>
          <MembersTable initialMembers={mockMembers} />
        </section>

        <section>
          <h2 className="mb-3 text-sm font-semibold text-text-secondary">
            Storage
          </h2>
          <StorageUsageCard
            usedLabel={mockStorageUsage.usedLabel}
            totalLabel={mockStorageUsage.totalLabel}
            usedPercent={mockStorageUsage.usedPercent}
          />
        </section>

        <section>
          <h2 className="mb-3 text-sm font-semibold text-text-secondary">
            Audit Log
          </h2>
          <AuditLogList entries={mockAuditLog} />
        </section>
      </div>
    </>
  );
}
