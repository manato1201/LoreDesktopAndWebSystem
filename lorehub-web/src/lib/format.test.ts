import { describe, expect, it } from "vitest";
import {
  formatAbsoluteTime,
  formatBytes,
  shortHash,
  summarizeStorage,
} from "./format";
import type { TreeNode } from "./types";

describe("shortHash", () => {
  it("keeps the first 7 characters", () => {
    expect(shortHash("0123456789abcdef")).toBe("0123456");
  });

  it("leaves a hash shorter than 7 characters alone", () => {
    expect(shortHash("abc")).toBe("abc");
  });
});

describe("formatAbsoluteTime", () => {
  it("formats unix seconds as UTC to the minute", () => {
    // 2026-10-25T09:41:30Z
    expect(formatAbsoluteTime(1792921290)).toBe("2026-10-25 09:41 UTC");
  });
});

describe("formatBytes", () => {
  it("uses bytes below 1 KB and one decimal above", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  it("moves up a unit when rounding would print 1024.0 of the smaller one", () => {
    // 1_048_575 B is 1023.999 KB; rounded to one decimal that reads "1024.0 KB".
    expect(formatBytes(1024 * 1024 - 1)).toBe("1.0 MB");
    expect(formatBytes(1024 * 1024 * 1024 - 1)).toBe("1.0 GB");
  });

  it("keeps the unit when the rounded value is still below 1024", () => {
    expect(formatBytes(1024 * 1024 - 1024)).toBe("1023.0 KB");
  });
});

const file = (
  kind: "audio" | "model3d" | "image" | "text" | "binary",
  name: string,
  sizeBytes: number,
): TreeNode => ({
  kind,
  path: name,
  name,
  sizeLabel: formatBytes(sizeBytes),
  sizeBytes,
  updatedAt: "1d ago",
  lockedBy: null,
});

describe("summarizeStorage", () => {
  it("sums by kind across nested directories and the total equals the parts", () => {
    const tree: TreeNode[] = [
      {
        kind: "directory",
        path: "Assets",
        name: "Assets",
        children: [
          file("audio", "a.wav", 1000),
          {
            kind: "directory",
            path: "Assets/3D",
            name: "3D",
            children: [file("model3d", "b.fbx", 4000), file("image", "c.png", 250)],
          },
        ],
      },
      file("text", "README.md", 50),
      file("binary", "d.bin", 700),
    ];

    const result = summarizeStorage(tree);

    expect(Object.fromEntries(result.buckets.map((b) => [b.bucket, b.bytes]))).toEqual({
      audio: 1000,
      model3d: 4000,
      image: 250,
      other: 750,
    });
    expect(result.totalBytes).toBe(6000);
    expect(result.totalBytes).toBe(result.buckets.reduce((n, b) => n + b.bytes, 0));
  });

  it("keeps every bucket (as 0) for an empty tree", () => {
    const result = summarizeStorage([]);
    expect(result.totalBytes).toBe(0);
    expect(result.buckets).toHaveLength(4);
  });
});
