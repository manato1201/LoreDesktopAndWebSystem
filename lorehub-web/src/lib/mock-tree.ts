import type { TreeNode } from "./types";

export const mockTree: TreeNode[] = [
  {
    kind: "directory",
    path: "Assets",
    name: "Assets",
    children: [
      {
        kind: "directory",
        path: "Assets/Characters",
        name: "Characters",
        children: [
          {
            kind: "model3d",
            path: "Assets/Characters/hero_rig.fbx",
            name: "hero_rig.fbx",
            sizeLabel: "42.1 MB",
            updatedAt: "2h ago",
            lockedBy: "Aiko Tanaka",
          },
          {
            kind: "image",
            path: "Assets/Characters/hero_diffuse.png",
            name: "hero_diffuse.png",
            sizeLabel: "18.4 MB",
            updatedAt: "1d ago",
            lockedBy: null,
          },
        ],
      },
      {
        kind: "directory",
        path: "Assets/Environments",
        name: "Environments",
        children: [
          {
            kind: "binary",
            path: "Assets/Environments/hollow_keep_terrain.uasset",
            name: "hollow_keep_terrain.uasset",
            sizeLabel: "1.2 GB",
            updatedAt: "6h ago",
            lockedBy: null,
          },
          {
            kind: "image",
            path: "Assets/Environments/skybox_dusk.png",
            name: "skybox_dusk.png",
            sizeLabel: "64.0 MB",
            updatedAt: "3d ago",
            lockedBy: null,
          },
        ],
      },
      {
        kind: "directory",
        path: "Assets/Audio",
        name: "Audio",
        children: [
          {
            kind: "audio",
            path: "Assets/Audio/theme_main.wav",
            name: "theme_main.wav",
            sizeLabel: "96.3 MB",
            updatedAt: "5d ago",
            lockedBy: "Marco Silva",
          },
        ],
      },
    ],
  },
  {
    kind: "directory",
    path: "Source",
    name: "Source",
    children: [
      {
        kind: "text",
        path: "Source/Game.cpp",
        name: "Game.cpp",
        sizeLabel: "12.8 KB",
        updatedAt: "2h ago",
        lockedBy: null,
      },
      {
        kind: "text",
        path: "Source/Game.h",
        name: "Game.h",
        sizeLabel: "3.1 KB",
        updatedAt: "2h ago",
        lockedBy: null,
      },
    ],
  },
  {
    kind: "text",
    path: "README.md",
    name: "README.md",
    sizeLabel: "2.4 KB",
    updatedAt: "1w ago",
    lockedBy: null,
  },
];

export const mockFileContents: Record<string, string> = {
  "Source/Game.cpp": `#include "Game.h"

void Game::Tick(float deltaSeconds)
{
    World.Update(deltaSeconds);
    Renderer.Submit(World.GetDrawCalls());
}
`,
  "Source/Game.h": `#pragma once

class Game
{
public:
    void Tick(float deltaSeconds);
};
`,
  "README.md": `# Hollow Keep

Environment art, terrain chunks, and lighting scenarios.

Run \`lore sync Assets/Environments\` for a sparse checkout.
`,
};
