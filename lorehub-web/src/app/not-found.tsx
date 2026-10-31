import Link from "next/link";
import { AsciiTree } from "@/components/AsciiTree";

export default function NotFound() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-6 px-6 text-center">
      <AsciiTree />
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-bold text-text-primary">
          This page doesn&apos;t exist
        </h1>
        <p className="max-w-sm text-sm text-text-secondary">
          The repository or page may have been deleted or moved, or you may not
          have access to it.
        </p>
      </div>
      <Link
        href="/"
        className="rounded-pill bg-accent px-5 py-2 text-sm font-bold text-black transition-opacity hover:opacity-90"
      >
        Back to repositories
      </Link>
    </main>
  );
}
