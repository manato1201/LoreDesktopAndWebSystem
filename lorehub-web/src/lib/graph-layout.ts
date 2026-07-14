import type { Branch, Commit } from "./types";

export type LaneStatus = "trunk" | "merged" | "open";

export type GraphNode = {
  commit: Commit;
  x: number;
  lane: number;
};

export type GraphEdge = {
  from: GraphNode;
  to: GraphNode;
};

export type CommitGraph = {
  nodes: GraphNode[];
  edges: GraphEdge[];
  laneCount: number;
  laneStatus: Record<number, LaneStatus>;
};

/**
 * Assumes `commits` is already in chronological order (oldest first) with
 * every commit's parents appearing earlier in the array — true for
 * lorehub-api's seeded history. Lane 0 is always the default branch;
 * other branches get the next free lane in first-appearance order.
 */
export function buildCommitGraph(
  commits: Commit[],
  branches: Branch[],
): CommitGraph {
  const laneByBranch = new Map<string, number>();
  const defaultBranch = branches.find((b) => b.isDefault)?.name;
  if (defaultBranch) laneByBranch.set(defaultBranch, 0);

  const nodes: GraphNode[] = commits.map((commit, x) => {
    if (!laneByBranch.has(commit.branch)) {
      laneByBranch.set(commit.branch, laneByBranch.size);
    }
    return { commit, x, lane: laneByBranch.get(commit.branch)! };
  });

  const nodeByHash = new Map(nodes.map((n) => [n.commit.hash, n]));

  const edges: GraphEdge[] = [];
  for (const node of nodes) {
    for (const parentHash of node.commit.parents) {
      const parentNode = nodeByHash.get(parentHash);
      if (parentNode) edges.push({ from: parentNode, to: node });
    }
  }

  const laneStatus: Record<number, LaneStatus> = { 0: "trunk" };
  for (let lane = 1; lane < laneByBranch.size; lane++) {
    const mergesOut = edges.some(
      (e) => e.from.lane === lane && e.to.lane !== lane,
    );
    laneStatus[lane] = mergesOut ? "merged" : "open";
  }

  return { nodes, edges, laneCount: laneByBranch.size, laneStatus };
}
