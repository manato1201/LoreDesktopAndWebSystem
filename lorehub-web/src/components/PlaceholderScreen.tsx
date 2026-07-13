import type { ComponentType } from "react";

export function PlaceholderScreen({
  icon: Icon,
  title,
  description,
}: {
  icon: ComponentType<{ className?: string }>;
  title: string;
  description: string;
}) {
  return (
    <div className="flex min-h-80 flex-col items-center justify-center gap-3 rounded-comfortable bg-surface p-12 text-center">
      <Icon className="size-8 text-text-secondary" />
      <h2 className="text-lg font-semibold text-text-primary">{title}</h2>
      <p className="max-w-sm text-sm text-text-secondary">{description}</p>
    </div>
  );
}
