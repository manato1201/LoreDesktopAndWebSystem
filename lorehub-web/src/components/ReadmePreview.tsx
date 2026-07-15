"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { getFileContent } from "@/lib/api";
import { FileTextIcon } from "./icons";

const markdownComponents: Components = {
  h1: ({ children }) => (
    <h1 className="mb-4 mt-6 text-2xl font-bold text-text-primary first:mt-0">
      {children}
    </h1>
  ),
  h2: ({ children }) => (
    <h2 className="mb-3 mt-6 border-b border-border/40 pb-2 text-xl font-bold text-text-primary first:mt-0">
      {children}
    </h2>
  ),
  h3: ({ children }) => (
    <h3 className="mb-2 mt-5 text-lg font-semibold text-text-primary first:mt-0">
      {children}
    </h3>
  ),
  h4: ({ children }) => (
    <h4 className="mb-2 mt-4 text-base font-semibold text-text-primary first:mt-0">
      {children}
    </h4>
  ),
  p: ({ children }) => (
    <p className="mb-4 text-sm leading-relaxed text-text-secondary last:mb-0">
      {children}
    </p>
  ),
  a: ({ href, children }) => {
    const isInternal = href?.startsWith("/") ?? false;
    const className =
      "font-semibold text-accent underline-offset-2 hover:underline";
    if (isInternal && href) {
      return (
        <Link href={href} className={className}>
          {children}
        </Link>
      );
    }
    return (
      <a
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        className={className}
      >
        {children}
      </a>
    );
  },
  strong: ({ children }) => (
    <strong className="font-bold text-text-primary">{children}</strong>
  ),
  em: ({ children }) => <em className="italic">{children}</em>,
  ul: ({ children }) => (
    <ul className="mb-4 flex list-disc flex-col gap-1 pl-5 text-sm text-text-secondary last:mb-0">
      {children}
    </ul>
  ),
  ol: ({ children }) => (
    <ol className="mb-4 flex list-decimal flex-col gap-1 pl-5 text-sm text-text-secondary last:mb-0">
      {children}
    </ol>
  ),
  li: ({ children }) => <li className="leading-relaxed">{children}</li>,
  blockquote: ({ children }) => (
    <blockquote className="mb-4 border-l-2 border-border pl-4 text-sm italic text-text-secondary last:mb-0">
      {children}
    </blockquote>
  ),
  hr: () => <hr className="my-6 border-border/40" />,
  code: ({ className, children }) => {
    const isBlock = className?.includes("language-");
    if (isBlock) {
      return <code className={className}>{children}</code>;
    }
    return (
      <code className="rounded-subtle bg-surface-elevated px-1.5 py-0.5 font-mono text-[13px] text-text-primary">
        {children}
      </code>
    );
  },
  pre: ({ children }) => (
    <pre className="mb-4 overflow-x-auto rounded-comfortable bg-surface-elevated p-4 text-xs leading-relaxed text-text-secondary last:mb-0">
      {children}
    </pre>
  ),
  table: ({ children }) => (
    <div className="mb-4 overflow-x-auto rounded-standard border border-border/40 last:mb-0">
      <table className="w-full border-collapse text-sm">{children}</table>
    </div>
  ),
  thead: ({ children }) => (
    <thead className="border-b border-border/40 text-left text-text-secondary">
      {children}
    </thead>
  ),
  th: ({ children }) => (
    <th className="px-3 py-2 text-xs font-semibold">{children}</th>
  ),
  td: ({ children }) => (
    <td className="border-t border-border/40 px-3 py-2 text-text-secondary">
      {children}
    </td>
  ),
  img: ({ src, alt }) => (
    // eslint-disable-next-line @next/next/no-img-element -- markdown source is untrusted/arbitrary, next/image's remote-origin allowlist doesn't apply here
    <img src={src} alt={alt ?? ""} className="max-w-full rounded-standard" />
  ),
};

/**
 * Fetches and renders a repository's top-level README as formatted
 * markdown. Keyed by `path` in the parent (see TextFileContent for the
 * same remount-on-change pattern) so switching repos doesn't show stale
 * content mid-fetch.
 */
export function ReadmePreview({
  repoSlug,
  path,
}: {
  repoSlug: string;
  path: string;
}) {
  const [content, setContent] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getFileContent(repoSlug, path).then((result) => {
      if (!cancelled) {
        setContent(result);
        setLoaded(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [repoSlug, path]);

  if (!loaded) {
    return <p className="text-sm text-text-secondary">Loading README…</p>;
  }

  if (!content) {
    return (
      <p className="text-sm text-text-secondary">Preview not available.</p>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="mb-3 flex items-center gap-2 text-xs font-semibold text-text-secondary">
        <FileTextIcon className="size-4" />
        {path}
      </div>
      <div className="max-w-3xl">
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={markdownComponents}
        >
          {content}
        </ReactMarkdown>
      </div>
    </div>
  );
}
