import type { PRDiffFile } from "@/lib/types";
import { ChangeTypeBadge } from "./ChangeTypeBadge";
import { FILE_KIND_ICON, inferFileKind } from "./icons";
import { ImageDiffSlider } from "./ImageDiffSlider";
import { Model3DDiffToggle } from "./Model3DDiffToggle";
import { TextDiffViewer } from "./TextDiffViewer";

export function DiffFileViewer({
  repoSlug,
  file,
}: {
  repoSlug: string;
  file: PRDiffFile;
}) {
  const Icon = FILE_KIND_ICON[inferFileKind(file.path)];

  return (
    <div className="flex flex-col gap-3 rounded-comfortable bg-surface p-4">
      <div className="flex items-center gap-2">
        <Icon className="size-4 shrink-0 text-text-secondary" />
        <span className="min-w-0 flex-1 truncate text-sm font-bold text-text-primary">
          {file.path}
        </span>
        <ChangeTypeBadge type={file.changeType} />
      </div>

      {file.diffKind === "text" && <TextDiffViewer lines={file.lines} />}
      {file.diffKind === "image" && (
        <ImageDiffSlider
          repoSlug={repoSlug}
          path={file.path}
          changeType={file.changeType}
        />
      )}
      {file.diffKind === "model3d" && <Model3DDiffToggle path={file.path} />}
    </div>
  );
}
