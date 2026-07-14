"use client";

import { useState } from "react";
import type { PRComment } from "@/lib/types";
import { AuthorAvatar } from "./AuthorAvatar";

export function CommentThread({
  initialComments,
}: {
  initialComments: PRComment[];
}) {
  const [comments, setComments] = useState(initialComments);
  const [draft, setDraft] = useState("");

  const handleSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    const body = draft.trim();
    if (!body) return;

    setComments((prev) => [
      ...prev,
      {
        id: `local-${prev.length}-${Date.now()}`,
        author: "You",
        authorInitials: "Y",
        timestamp: "just now",
        body,
      },
    ]);
    setDraft("");
  };

  return (
    <div className="flex flex-col gap-4">
      <h3 className="text-sm font-semibold text-text-secondary">
        {comments.length} comment{comments.length === 1 ? "" : "s"}
      </h3>

      <ul className="flex flex-col gap-3">
        {comments.map((comment) => (
          <li
            key={comment.id}
            className="flex items-start gap-3 rounded-comfortable bg-surface p-4"
          >
            <AuthorAvatar initials={comment.authorInitials} />
            <div className="min-w-0 flex-1">
              <p className="text-xs text-text-secondary">
                <span className="font-bold text-text-primary">
                  {comment.author}
                </span>{" "}
                · {comment.timestamp}
              </p>
              <p className="mt-1 text-sm text-text-primary">{comment.body}</p>
            </div>
          </li>
        ))}
      </ul>

      <form onSubmit={handleSubmit} className="flex flex-col gap-2">
        <label htmlFor="comment-draft" className="sr-only">
          Add a comment
        </label>
        <textarea
          id="comment-draft"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Leave a review comment…"
          rows={3}
          className="w-full resize-y rounded-comfortable bg-surface-interactive p-3 text-sm text-text-primary placeholder:text-text-secondary focus-visible:outline-2 focus-visible:outline-accent"
        />
        <button
          type="submit"
          disabled={!draft.trim()}
          className="self-end rounded-pill bg-accent px-5 py-2 text-xs font-bold uppercase tracking-wide text-bg-base transition-opacity disabled:opacity-40"
        >
          Comment
        </button>
      </form>
    </div>
  );
}
