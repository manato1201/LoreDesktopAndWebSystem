"use client";

import { useEffect, useState } from "react";
import { audioUrl, imageUrl } from "@/lib/api";
import type { FileKind } from "@/lib/types";
import { AudioPlayer } from "./AudioPlayer";
import { ModelViewer } from "./ModelViewer";

const PREVIEW_LABEL: Partial<Record<FileKind, string>> = {
  binary: "No preview available for this file type",
};

type StreamState =
  | { status: "loading"; progress: number }
  | { status: "ready"; objectUrl: string }
  | { status: "error" };

/**
 * Real chunk-based streaming per ARCHITECTURE.md §4.3, for the `image` and
 * `audio` kinds (the ones lorehub-api actually has bytes for): `fetch(url,
 * { credentials: "include" })`, then `response.body.getReader()` to
 * accumulate chunks and report `progress` as real bytes-received-over-the-
 * wire (`bytesReceived / Content-Length`) — not a timer. Once fully read,
 * the bytes are assembled into a `Blob` and exposed via
 * `URL.createObjectURL`, revoked on unmount/path-change so switching files
 * doesn't leak object URLs. The parent passes `key={path}` so a new file
 * remounts this component (and therefore re-runs this effect from scratch)
 * instead of reusing state across files.
 *
 * lorehub-api's `/image`, `/image-before`, and `/audio` endpoints also
 * support real HTTP Range requests (`206 Partial Content` with
 * `Content-Range`/`Accept-Ranges`) — this component still reads the whole
 * body in one pass because the demo assets here are only a few KB (SVG) or
 * ~130KB (WAV), but the transport underneath genuinely supports partial
 * reads for a real large file substituted in.
 *
 * `model3d` renders the existing `ModelViewer` placeholder immediately —
 * there is no real model asset being streamed here, per this project's own
 * established simplification (don't invent one). `binary` falls back to
 * the existing static "no preview" message immediately. Neither kind goes
 * through the fetch/progress path since there's no real byte stream backing
 * them, so faking a progress bar for them would just be the same
 * simulation this component used to do for everything.
 */
export function StreamingPreview({
  kind,
  repoSlug,
  path,
}: {
  kind: FileKind;
  repoSlug: string;
  path: string;
}) {
  const [state, setState] = useState<StreamState>({
    status: "loading",
    progress: 0,
  });

  useEffect(() => {
    if (kind !== "image" && kind !== "audio") return;

    let cancelled = false;
    let objectUrl: string | null = null;
    const controller = new AbortController();

    // No explicit "reset to loading" setState here: the initial `useState`
    // value already is `{ status: "loading", progress: 0 }`, and the parent
    // keys this component on `path` (see doc comment above) so a new file
    // always means a fresh mount with that initial state, not a re-run of
    // this same effect instance.
    const url =
      kind === "image" ? imageUrl(repoSlug, path) : audioUrl(repoSlug, path);
    const contentType = kind === "image" ? "image/svg+xml" : "audio/wav";

    (async () => {
      try {
        const res = await fetch(url, {
          credentials: "include",
          signal: controller.signal,
        });
        if (!res.ok || !res.body) {
          throw new Error(`GET ${url} failed: ${res.status}`);
        }

        // Real total from the wire, not an assumption — drives the actual
        // percentage below. If a server ever omits it, we fall back to 0%
        // until the stream completes rather than pretending we know a total.
        const total = Number.parseInt(
          res.headers.get("content-length") ?? "",
          10,
        );

        const reader = res.body.getReader();
        const chunks: Uint8Array[] = [];
        let received = 0;

        for (;;) {
          const { done, value } = await reader.read();
          if (done) break;
          if (!value) continue;

          chunks.push(value);
          received += value.byteLength;

          if (!cancelled) {
            const progress =
              Number.isFinite(total) && total > 0
                ? Math.min(100, Math.round((received / total) * 100))
                : 0;
            setState({ status: "loading", progress });
          }
        }

        if (cancelled) return;

        const blob = new Blob(chunks as BlobPart[], { type: contentType });
        objectUrl = URL.createObjectURL(blob);
        setState({ status: "ready", objectUrl });
      } catch (error) {
        if (cancelled) return;
        console.error(
          "StreamingPreview: streaming fetch failed for",
          path,
          error,
        );
        setState({ status: "error" });
      }
    })();

    return () => {
      cancelled = true;
      controller.abort();
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, [kind, repoSlug, path]);

  if (kind === "model3d") {
    return <ModelViewer className="aspect-video" />;
  }

  if (kind === "binary") {
    return (
      <div className="flex aspect-video flex-col items-center justify-center gap-3 rounded-comfortable bg-surface-elevated">
        <p className="max-w-xs text-center text-xs text-text-secondary">
          {PREVIEW_LABEL.binary}
        </p>
      </div>
    );
  }

  if (state.status === "ready") {
    if (kind === "image") {
      return (
        // eslint-disable-next-line @next/next/no-img-element -- dynamic cross-origin SVG assembled from streamed bytes into a blob: URL, not a static asset next/image can optimize
        <img
          src={state.objectUrl}
          alt=""
          className="aspect-video w-full rounded-comfortable object-cover"
        />
      );
    }
    return <AudioPlayer src={state.objectUrl} />;
  }

  if (state.status === "error") {
    return (
      <div className="flex aspect-video flex-col items-center justify-center gap-3 rounded-comfortable bg-surface-elevated">
        <p className="max-w-xs text-center text-xs text-text-secondary">
          Failed to load preview.
        </p>
      </div>
    );
  }

  return (
    <div className="flex aspect-video flex-col items-center justify-center gap-3 rounded-comfortable bg-surface-elevated">
      <div className="flex w-48 flex-col items-center gap-2">
        <p className="text-xs text-text-secondary">Streaming chunks…</p>
        <div
          role="progressbar"
          aria-valuenow={state.progress}
          aria-valuemin={0}
          aria-valuemax={100}
          className="h-1.5 w-full overflow-hidden rounded-pill bg-surface"
        >
          <div
            className="h-full rounded-pill bg-accent"
            style={{ width: `${state.progress}%` }}
          />
        </div>
      </div>
    </div>
  );
}
