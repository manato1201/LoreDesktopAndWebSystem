import { AuditLogList } from "@/components/AuditLogList";
import { MembersTable } from "@/components/MembersTable";
import { PageHeader } from "@/components/PageHeader";
import { StorageUsageCard } from "@/components/StorageUsageCard";
import { getAuditLog, getMembers, getStorage } from "@/lib/api";

export const metadata = { title: "Settings · LoreHub" };

export default async function SettingsPage() {
  const [members, storage, auditLog] = await Promise.all([
    getMembers(),
    getStorage(),
    getAuditLog(),
  ]);

  return (
    <>
      <PageHeader title="Organization Settings" subtitle="Nebula Studios" />

      <div className="flex flex-col gap-8">
        <section>
          <h2 className="mb-3 text-sm font-semibold text-text-secondary">
            Members
          </h2>
          <MembersTable initialMembers={members} />
        </section>

        <section>
          <h2 className="mb-3 text-sm font-semibold text-text-secondary">
            Storage
          </h2>
          <StorageUsageCard
            usedLabel={storage.usedLabel}
            totalLabel={storage.totalLabel}
            usedPercent={storage.usedPercent}
          />
        </section>

        <section>
          <h2 className="mb-3 text-sm font-semibold text-text-secondary">
            Audit Log
          </h2>
          <AuditLogList entries={auditLog} />
        </section>
      </div>
    </>
  );
}
