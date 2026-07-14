"use client";

import { useEffect, useRef, useState } from "react";
import { PauseIcon, PlayIcon } from "./icons";

const BAR_COUNT = 80;
const SEEK_STEP_SECONDS = 2;

function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds)) return "0:00";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/**
 * Decodes the real WAV bytes via the Web Audio API to draw an actual
 * waveform (min/max peaks per bar), then plays it back through a plain
 * <audio> element so native play/pause/seek behavior stays simple.
 */
export function AudioPlayer({ src }: { src: string }) {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [progress, setProgress] = useState(0);
  const [duration, setDuration] = useState(0);
  const [peaks, setPeaks] = useState<number[] | null>(null);

  useEffect(() => {
    let cancelled = false;

    type WebAudioWindow = typeof window & {
      webkitAudioContext?: typeof AudioContext;
    };

    Promise.resolve()
      .then(() => {
        const AudioContextCtor =
          window.AudioContext || (window as WebAudioWindow).webkitAudioContext;
        if (!AudioContextCtor) {
          throw new Error("Web Audio API not supported");
        }
        const ctx = new AudioContextCtor();
        return fetch(src)
          .then((res) => res.arrayBuffer())
          .then((buffer) => ctx.decodeAudioData(buffer))
          .finally(() => ctx.close());
      })
      .then((audioBuffer) => {
        if (cancelled) return;
        const channel = audioBuffer.getChannelData(0);
        const blockSize = Math.max(1, Math.floor(channel.length / BAR_COUNT));
        const bars: number[] = [];
        for (let i = 0; i < BAR_COUNT; i++) {
          let max = 0;
          const start = i * blockSize;
          for (let j = 0; j < blockSize; j++) {
            const v = Math.abs(channel[start + j] ?? 0);
            if (v > max) max = v;
          }
          bars.push(max);
        }
        setPeaks(bars);
      })
      .catch(() => {
        if (!cancelled) setPeaks([]);
      });

    return () => {
      cancelled = true;
    };
  }, [src]);

  const togglePlay = () => {
    const el = audioRef.current;
    if (!el) return;
    if (el.paused) {
      el.play();
    } else {
      el.pause();
    }
  };

  const seekToClientX = (clientX: number, rect: DOMRect) => {
    const el = audioRef.current;
    if (!el || !el.duration) return;
    const pct = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
    el.currentTime = pct * el.duration;
  };

  const handleSeekKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const el = audioRef.current;
    if (!el || !el.duration) return;
    if (event.key === "ArrowLeft") {
      el.currentTime = Math.max(0, el.currentTime - SEEK_STEP_SECONDS);
    } else if (event.key === "ArrowRight") {
      el.currentTime = Math.min(
        el.duration,
        el.currentTime + SEEK_STEP_SECONDS,
      );
    } else {
      return;
    }
    event.preventDefault();
  };

  return (
    <div className="flex flex-col gap-3 rounded-comfortable bg-surface-elevated p-4">
      <audio
        ref={audioRef}
        src={src}
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onEnded={() => setIsPlaying(false)}
        onTimeUpdate={(event) => {
          const el = event.currentTarget;
          if (el.duration) setProgress(el.currentTime / el.duration);
        }}
        onLoadedMetadata={(event) => setDuration(event.currentTarget.duration)}
      />

      <div className="flex items-center gap-3">
        <button
          type="button"
          onClick={togglePlay}
          aria-label={isPlaying ? "Pause" : "Play"}
          className="flex size-9 shrink-0 items-center justify-center rounded-full bg-accent text-bg-base"
        >
          {isPlaying ? (
            <PauseIcon className="size-4" />
          ) : (
            <PlayIcon className="size-4" />
          )}
        </button>

        <div
          role="slider"
          tabIndex={0}
          aria-label="Playback position"
          aria-valuenow={Math.round(progress * 100)}
          aria-valuemin={0}
          aria-valuemax={100}
          onClick={(event) =>
            seekToClientX(
              event.clientX,
              event.currentTarget.getBoundingClientRect(),
            )
          }
          onKeyDown={handleSeekKeyDown}
          className="relative h-10 flex-1 cursor-pointer outline-none"
        >
          {peaks ? (
            <svg
              viewBox={`0 0 ${BAR_COUNT} 40`}
              preserveAspectRatio="none"
              className="size-full"
              aria-hidden="true"
            >
              {peaks.map((amplitude, index) => {
                const height = Math.max(2, amplitude * 40);
                const played = index / BAR_COUNT < progress;
                return (
                  <rect
                    key={index}
                    x={index}
                    y={(40 - height) / 2}
                    width={0.7}
                    height={height}
                    fill={
                      played
                        ? "var(--color-accent)"
                        : "var(--color-border-light)"
                    }
                  />
                );
              })}
            </svg>
          ) : (
            <div className="h-full animate-pulse rounded-standard bg-surface" />
          )}
        </div>

        <span className="w-10 shrink-0 text-right text-xs text-text-secondary">
          {formatTime(duration)}
        </span>
      </div>
    </div>
  );
}
