export function StorageUsageCard({
  usedLabel,
  totalLabel,
  usedPercent,
}: {
  usedLabel: string;
  totalLabel: string;
  usedPercent: number;
}) {
  return (
    <div className="flex flex-col gap-3 rounded-comfortable bg-surface p-5">
      <div className="flex items-baseline justify-between">
        <p className="text-sm text-text-secondary">Storage used</p>
        <p className="text-sm text-text-primary">
          <span className="font-bold">{usedLabel}</span> of {totalLabel}
        </p>
      </div>
      <div
        role="progressbar"
        aria-valuenow={usedPercent}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label="Storage used"
        className="h-2 w-full overflow-hidden rounded-pill bg-surface-interactive"
      >
        <div
          className="h-full rounded-pill bg-accent"
          style={{ width: `${usedPercent}%` }}
        />
      </div>
    </div>
  );
}
