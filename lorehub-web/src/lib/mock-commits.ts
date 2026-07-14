import type { Commit } from "./types";

export const mockCommits: Commit[] = [
  {
    hash: "a1c4e7f92b3d5e6081247fa9c0d8b3e6f2a1c47",
    shortHash: "a1c4e7f",
    message: "Retarget hero rig to updated skeleton",
    author: "Aiko Tanaka",
    authorInitials: "AT",
    timestamp: "2h ago",
    changedFiles: [
      {
        path: "Assets/Characters/hero_rig.fbx",
        changeType: "modified",
        sizeDeltaLabel: "+1.2 MB",
      },
    ],
  },
  {
    hash: "7d2b9c1e4f68a0b3d5c7e9f1a2b4c6d8e0f1a2b3",
    shortHash: "7d2b9c1",
    message: "Add dusk skybox for Hollow Keep exteriors",
    author: "Marco Silva",
    authorInitials: "MS",
    timestamp: "6h ago",
    changedFiles: [
      {
        path: "Assets/Environments/skybox_dusk.png",
        changeType: "added",
        sizeDeltaLabel: "+64.0 MB",
      },
    ],
  },
  {
    hash: "e5f8a1b4c7d0e3f6a9b2c5d8e1f4a7b0c3d6e9f2",
    shortHash: "e5f8a1b",
    message: "Fix world tick order for late-joining actors",
    description:
      "Renderer.Submit was picking up stale draw calls when actors joined mid-tick.",
    author: "Priya Desai",
    authorInitials: "PD",
    timestamp: "1d ago",
    changedFiles: [
      {
        path: "Source/Game.cpp",
        changeType: "modified",
        sizeDeltaLabel: "+0.4 KB",
      },
      { path: "Source/Game.h", changeType: "modified", sizeDeltaLabel: "±0 B" },
    ],
  },
  {
    hash: "3c6d9e2f5a8b1c4d7e0f3a6b9c2d5e8f1a4b7c0d",
    shortHash: "3c6d9e2",
    message: "Remove deprecated terrain LOD tier",
    author: "Marco Silva",
    authorInitials: "MS",
    timestamp: "2d ago",
    changedFiles: [
      {
        path: "Assets/Environments/hollow_keep_terrain.uasset",
        changeType: "modified",
        sizeDeltaLabel: "-320.0 MB",
      },
    ],
  },
  {
    hash: "9b1e4f7a0c3d6e9f2a5b8c1d4e7f0a3b6c9d2e5f",
    shortHash: "9b1e4f7",
    message: "Record updated main theme mix",
    author: "Priya Desai",
    authorInitials: "PD",
    timestamp: "5d ago",
    changedFiles: [
      {
        path: "Assets/Audio/theme_main.wav",
        changeType: "modified",
        sizeDeltaLabel: "+3.1 MB",
      },
    ],
  },
  {
    hash: "0f4a7b0c3d6e9f2a5b8c1d4e7f0a3b6c9d2e5f8a",
    shortHash: "0f4a7b0",
    message: "Document sparse checkout workflow",
    author: "Aiko Tanaka",
    authorInitials: "AT",
    timestamp: "1w ago",
    changedFiles: [
      { path: "README.md", changeType: "modified", sizeDeltaLabel: "+0.6 KB" },
    ],
  },
];
