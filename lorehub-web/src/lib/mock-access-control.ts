import type { AccessEntry } from "./types";

export const mockAccessEntries: Record<string, AccessEntry[]> = {
  Assets: [
    {
      principal: "Environment Artists",
      principalType: "team",
      permissions: ["read", "write"],
    },
    {
      principal: "Character Artists",
      principalType: "team",
      permissions: ["read", "write"],
    },
    {
      principal: "QA Contractors",
      principalType: "team",
      permissions: ["read"],
    },
  ],
  "Assets/Characters": [
    {
      principal: "Character Artists",
      principalType: "team",
      permissions: ["read", "write", "lock"],
    },
    {
      principal: "Aiko Tanaka",
      principalType: "user",
      permissions: ["read", "write", "lock"],
    },
  ],
  "Assets/Environments": [
    {
      principal: "Environment Artists",
      principalType: "team",
      permissions: ["read", "write", "lock"],
    },
  ],
  Source: [
    {
      principal: "Engineering",
      principalType: "team",
      permissions: ["read", "write", "lock"],
    },
    {
      principal: "QA Contractors",
      principalType: "team",
      permissions: ["read"],
    },
  ],
};
