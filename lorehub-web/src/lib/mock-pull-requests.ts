import type { PullRequest } from "./types";

export const mockPullRequests: PullRequest[] = [
  {
    id: "42",
    title: "Retarget hero rig to updated skeleton",
    description:
      "Reworks the hero rig's bone hierarchy to match the new mocap skeleton. Also swaps the diffuse texture for the higher-resolution pass.",
    repoSlug: "hollow-keep-env",
    repoName: "hollow-keep-env",
    status: "open",
    author: "Aiko Tanaka",
    authorInitials: "AT",
    createdAt: "3h ago",
    updatedAt: "1h ago",
    changedFiles: [
      {
        diffKind: "text",
        path: "Source/Game.cpp",
        changeType: "modified",
        lines: [
          { type: "context", text: "void Game::Tick(float deltaSeconds)" },
          { type: "context", text: "{" },
          { type: "remove", text: "    World.Update(deltaSeconds);" },
          { type: "add", text: "    World.Update(deltaSeconds * TimeScale);" },
          { type: "add", text: "    World.FlushPendingLocks();" },
          {
            type: "context",
            text: "    Renderer.Submit(World.GetDrawCalls());",
          },
          { type: "context", text: "}" },
        ],
      },
      {
        diffKind: "model3d",
        path: "Assets/Characters/hero_rig.fbx",
        changeType: "modified",
      },
      {
        diffKind: "image",
        path: "Assets/Characters/hero_diffuse.png",
        changeType: "modified",
      },
    ],
    comments: [
      {
        id: "c1",
        author: "Marco Silva",
        authorInitials: "MS",
        timestamp: "50m ago",
        body: "Rig deformation on the left shoulder looks correct now. Diffuse pass is a nice upgrade.",
      },
    ],
  },
  {
    id: "39",
    title: "Add dusk skybox for Hollow Keep exteriors",
    description:
      "New skybox pass for the exterior courtyard scenes. Replaces the placeholder gradient sky.",
    repoSlug: "hollow-keep-env",
    repoName: "hollow-keep-env",
    status: "merged",
    author: "Marco Silva",
    authorInitials: "MS",
    createdAt: "2d ago",
    updatedAt: "1d ago",
    changedFiles: [
      {
        diffKind: "image",
        path: "Assets/Environments/skybox_dusk.png",
        changeType: "added",
      },
    ],
    comments: [
      {
        id: "c2",
        author: "Priya Desai",
        authorInitials: "PD",
        timestamp: "1d ago",
        body: "Color grading matches the reference board. Merging.",
      },
    ],
  },
  {
    id: "35",
    title: "Revert oversized terrain LOD experiment",
    description:
      "The experimental LOD tier regressed streaming performance on the courtyard level. Reverting until the chunking strategy is revisited.",
    repoSlug: "hollow-keep-env",
    repoName: "hollow-keep-env",
    status: "closed",
    author: "Priya Desai",
    authorInitials: "PD",
    createdAt: "5d ago",
    updatedAt: "4d ago",
    changedFiles: [
      {
        diffKind: "text",
        path: "README.md",
        changeType: "modified",
        lines: [
          { type: "context", text: "## Terrain LOD" },
          {
            type: "remove",
            text: "Experimental 5-tier LOD is enabled by default.",
          },
          {
            type: "add",
            text: "LOD stays at the standard 3-tier setup for now.",
          },
        ],
      },
    ],
    comments: [],
  },
];
