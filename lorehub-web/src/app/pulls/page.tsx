import Link from "next/link";
import { PageHeader } from "@/components/PageHeader";
import { PullRequestRow } from "@/components/PullRequestRow";
import { mockPullRequests } from "@/lib/mock-pull-requests";
import type { PRStatus } from "@/lib/types";

export const metadata = { title: "Pull Requests · LoreHub" };

const STATUS_TABS: { value: PRStatus; label: string }[] = [
  { value: "open", label: "Open" },
  { value: "merged", label: "Merged" },
  { value: "closed", label: "Closed" },
];

function isPRStatus(value: string | undefined): value is PRStatus {
  return value === "open" || value === "merged" || value === "closed";
}

export default async function PullsPage(props: PageProps<"/pulls">) {
  const params = await props.searchParams;
  const rawStatus = Array.isArray(params.status)
    ? params.status[0]
    : params.status;
  const status: PRStatus = isPRStatus(rawStatus) ? rawStatus : "open";

  const filtered = mockPullRequests.filter((pr) => pr.status === status);

  return (
    <>
      <PageHeader title="Pull Requests" />

      <div
        role="tablist"
        aria-label="Filter by status"
        className="mb-6 flex gap-1 border-b border-border/40"
      >
        {STATUS_TABS.map((tab) => {
          const isActive = tab.value === status;
          return (
            <Link
              key={tab.value}
              href={`/pulls?status=${tab.value}`}
              role="tab"
              aria-selected={isActive}
              className={`border-b-2 px-3 py-2 text-sm transition-colors ${
                isActive
                  ? "border-accent font-bold text-text-primary"
                  : "border-transparent font-normal text-text-secondary hover:text-text-primary"
              }`}
            >
              {tab.label}
            </Link>
          );
        })}
      </div>

      {filtered.length > 0 ? (
        <div className="flex flex-col gap-2">
          {filtered.map((pr) => (
            <PullRequestRow key={pr.id} pullRequest={pr} />
          ))}
        </div>
      ) : (
        <p className="text-sm text-text-secondary">
          No {status} pull requests.
        </p>
      )}
    </>
  );
}
