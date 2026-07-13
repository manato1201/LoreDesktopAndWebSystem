import type { Repository } from "./types";

export const mockRepositories: Repository[] = [
  {
    slug: "starforge-vfx",
    name: "starforge-vfx",
    organization: "Nebula Studios",
    description:
      "Particle FX library and Niagara modules for the Starforge campaign.",
    updatedAt: "2h ago",
    sizeLabel: "184 GB",
    lockedFileCount: 3,
    visibility: "private",
  },
  {
    slug: "hollow-keep-env",
    name: "hollow-keep-env",
    organization: "Nebula Studios",
    description:
      "Environment art, terrain chunks, and lighting scenarios for Hollow Keep.",
    updatedAt: "6h ago",
    sizeLabel: "512 GB",
    lockedFileCount: 0,
    visibility: "private",
  },
  {
    slug: "character-rigs",
    name: "character-rigs",
    organization: "Nebula Studios",
    description:
      "Shared character skeletons, rigs, and animation retarget presets.",
    updatedAt: "1d ago",
    sizeLabel: "76 GB",
    lockedFileCount: 1,
    visibility: "internal",
  },
  {
    slug: "audio-master",
    name: "audio-master",
    organization: "Nebula Studios",
    description: "Master audio sessions, foley captures, and mix stems.",
    updatedAt: "2d ago",
    sizeLabel: "212 GB",
    lockedFileCount: 0,
    visibility: "private",
  },
  {
    slug: "cinematics-s2",
    name: "cinematics-s2",
    organization: "Nebula Studios",
    description:
      "Season 2 cinematic sequences, previs, and camera capture data.",
    updatedAt: "3d ago",
    sizeLabel: "1.1 TB",
    lockedFileCount: 5,
    visibility: "private",
  },
  {
    slug: "shared-materials",
    name: "shared-materials",
    organization: "Nebula Studios",
    description:
      "Cross-project material library, substance graphs, and texture sets.",
    updatedAt: "5d ago",
    sizeLabel: "98 GB",
    lockedFileCount: 0,
    visibility: "public",
  },
];
