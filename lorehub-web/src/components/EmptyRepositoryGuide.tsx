import { AsciiTree } from "./AsciiTree";

/** Shown when a repository has no files yet: say what is happening and what to do next. */
export function EmptyRepositoryGuide() {
  return (
    <div className="flex min-h-80 flex-col items-center justify-center gap-5 p-6 text-center">
      <AsciiTree />
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold text-text-primary">
          Waiting for the first commit
        </h2>
        <p className="max-w-sm text-sm text-text-secondary">
          This repository has no files yet. Add some from LoreForge Client and
          they will appear here.
        </p>
      </div>
      <ol className="flex max-w-sm list-decimal flex-col gap-1.5 pl-5 text-left text-sm text-text-secondary">
        <li>Open LoreForge Client and sign in to this server.</li>
        <li>Open this repository and add your files.</li>
        <li>Commit. The file tree and README show up here.</li>
      </ol>
    </div>
  );
}
