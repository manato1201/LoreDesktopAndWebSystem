export function AuthorAvatar({
  initials,
  className,
}: {
  initials: string;
  className?: string;
}) {
  return (
    <span
      aria-hidden="true"
      className={`flex size-6 shrink-0 items-center justify-center rounded-full bg-surface-interactive text-[10.5px] font-bold text-text-primary ${className ?? ""}`}
    >
      {initials}
    </span>
  );
}
