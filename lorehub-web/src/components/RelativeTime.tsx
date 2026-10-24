import { formatAbsoluteTime } from "@/lib/format";

/**
 * "3h ago" on screen. The exact time is in the hover tooltip and is also read
 * out by screen readers, since a tooltip alone is not exposed reliably.
 */
export function RelativeTime({
  label,
  unixSeconds,
}: {
  label: string;
  unixSeconds?: number;
}) {
  if (unixSeconds === undefined) {
    return <span>{label}</span>;
  }
  const absolute = formatAbsoluteTime(unixSeconds);
  return (
    <time
      dateTime={new Date(unixSeconds * 1000).toISOString()}
      title={absolute}
    >
      {label}
      <span className="sr-only"> ({absolute})</span>
    </time>
  );
}
