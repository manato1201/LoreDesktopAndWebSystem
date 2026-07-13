export type Repository = {
  slug: string;
  name: string;
  organization: string;
  description: string;
  updatedAt: string;
  sizeLabel: string;
  lockedFileCount: number;
  visibility: "private" | "internal" | "public";
};
