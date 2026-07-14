import Link from "next/link";
import { buildCommitGraph, type LaneStatus } from "@/lib/graph-layout";
import type { Branch, Commit } from "@/lib/types";

const NODE_SPACING_X = 210;
const LANE_HEIGHT = 76;
const NODE_W = 190;
const NODE_H = 48;
const PADDING_X = 16;
const PADDING_Y = 16;
const LABEL_COLUMN_W = 176;

const LANE_COLOR: Record<LaneStatus, string> = {
  trunk: "var(--color-text-secondary)",
  merged: "var(--color-accent)",
  open: "var(--color-announcement)",
};

const LANE_LABEL: Record<LaneStatus, string> = {
  trunk: "Trunk",
  merged: "Merged",
  open: "Open",
};

function edgePath(x1: number, y1: number, x2: number, y2: number): string {
  const midX = (x1 + x2) / 2;
  return `M ${x1} ${y1} C ${midX} ${y1}, ${midX} ${y2}, ${x2} ${y2}`;
}

export function BranchGraph({
  repoSlug,
  commits,
  branches,
}: {
  repoSlug: string;
  commits: Commit[];
  branches: Branch[];
}) {
  const graph = buildCommitGraph(commits, branches);

  const laneNameByIndex = new Map<number, string>();
  for (const node of graph.nodes) {
    if (!laneNameByIndex.has(node.lane)) {
      laneNameByIndex.set(node.lane, node.commit.branch);
    }
  }

  const graphWidth =
    PADDING_X * 2 +
    Math.max(0, graph.nodes.length - 1) * NODE_SPACING_X +
    NODE_W;
  const graphHeight =
    PADDING_Y * 2 + Math.max(0, graph.laneCount - 1) * LANE_HEIGHT + NODE_H;

  const pixelX = (x: number) => PADDING_X + x * NODE_SPACING_X;
  const pixelY = (lane: number) => PADDING_Y + lane * LANE_HEIGHT;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-4 text-xs text-text-secondary">
        {(Object.keys(LANE_LABEL) as LaneStatus[]).map((status) => (
          <span key={status} className="flex items-center gap-1.5">
            <span
              className="size-2 rounded-full"
              style={{ backgroundColor: LANE_COLOR[status] }}
            />
            {LANE_LABEL[status]}
          </span>
        ))}
      </div>

      <div className="flex gap-2">
        <div
          className="flex shrink-0 flex-col"
          style={{ width: LABEL_COLUMN_W, paddingTop: PADDING_Y }}
        >
          {Array.from({ length: graph.laneCount }, (_, lane) => (
            <div
              key={lane}
              style={{ height: LANE_HEIGHT }}
              className="flex items-center"
            >
              <span
                className="truncate rounded-standard bg-surface px-2 py-1.5 text-xs font-semibold"
                style={{ color: LANE_COLOR[graph.laneStatus[lane]] }}
                title={laneNameByIndex.get(lane)}
              >
                {laneNameByIndex.get(lane)}
              </span>
            </div>
          ))}
        </div>

        <div className="min-w-0 flex-1 overflow-x-auto rounded-comfortable bg-surface">
          <div
            className="relative"
            style={{ width: graphWidth, height: graphHeight }}
          >
            <svg
              className="absolute inset-0"
              width={graphWidth}
              height={graphHeight}
              aria-hidden="true"
            >
              {graph.edges.map((edge, index) => {
                const x1 = pixelX(edge.from.x) + NODE_W;
                const y1 = pixelY(edge.from.lane) + NODE_H / 2;
                const x2 = pixelX(edge.to.x);
                const y2 = pixelY(edge.to.lane) + NODE_H / 2;
                const color =
                  LANE_COLOR[
                    graph.laneStatus[
                      edge.from.lane !== 0 ? edge.from.lane : edge.to.lane
                    ]
                  ];
                return (
                  <path
                    key={index}
                    d={edgePath(x1, y1, x2, y2)}
                    fill="none"
                    stroke={color}
                    strokeWidth={2}
                    opacity={0.6}
                  />
                );
              })}
            </svg>

            {graph.nodes.map((node) => (
              <Link
                key={node.commit.hash}
                href={`/repositories/${repoSlug}/commits/${node.commit.hash}`}
                style={{
                  left: pixelX(node.x),
                  top: pixelY(node.lane),
                  width: NODE_W,
                  height: NODE_H,
                  borderColor: LANE_COLOR[graph.laneStatus[node.lane]],
                }}
                className="absolute flex flex-col justify-center gap-0.5 rounded-comfortable border-l-4 bg-surface-interactive px-3 py-1 transition-colors hover:bg-surface-elevated"
              >
                <span className="truncate text-xs font-bold text-text-primary">
                  {node.commit.message}
                </span>
                <span className="truncate text-[10.5px] text-text-secondary">
                  {node.commit.shortHash} · {node.commit.author}
                </span>
              </Link>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
