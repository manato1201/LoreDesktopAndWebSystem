import { AuditLogList } from "@/components/AuditLogList";
import { ChangePasswordForm } from "@/components/ChangePasswordForm";
import { InviteMembersPanel } from "@/components/InviteMembersPanel";
import { MembersTable } from "@/components/MembersTable";
import { PageHeader } from "@/components/PageHeader";
import { StorageUsageCard } from "@/components/StorageUsageCard";
import {
  getAuditLog,
  getCurrentUser,
  getMembers,
  getStorage,
  listInvites,
} from "@/lib/api";
import { getSessionCookieHeader } from "@/lib/auth-server";

export const metadata = { title: "Settings · LoreHub" };

export default async function SettingsPage() {
  const cookie = await getSessionCookieHeader();
  const [members, storage, auditLog, currentUser] = await Promise.all([
    getMembers(cookie),
    getStorage(cookie),
    getAuditLog(cookie),
    getCurrentUser(cookie),
  ]);

  // Owner/Admin only — the backend 403s `GET /api/org/invites` (and every
  // invite mutation) for a plain member, so there's no point fetching or
  // rendering a form that would always fail for them.
  const canManageInvites =
    currentUser?.role === "owner" || currentUser?.role === "admin";
  const pendingInvites = canManageInvites ? await listInvites(cookie) : [];

  return (
    <>
      <PageHeader title="Organization Settings" subtitle="Nebula Studios" />

      <div className="flex flex-col gap-8">
        <section>
          <h2 className="mb-3 text-sm font-semibold text-text-secondary">
            Account
          </h2>
          <ChangePasswordForm />
        </section>

        <section>
          <h2 className="mb-3 text-sm font-semibold text-text-secondary">
            Members
          </h2>
          <MembersTable initialMembers={members} />
        </section>

        {canManageInvites && (
          <section>
            <h2 className="mb-3 text-sm font-semibold text-text-secondary">
              Invite Members
            </h2>
            <InviteMembersPanel initialInvites={pendingInvites} />
          </section>
        )}

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
