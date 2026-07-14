import { RepositoryExplorer } from "@/components/RepositoryExplorer";
import { mockTree } from "@/lib/mock-tree";

export default function RepositoryCodePage() {
  return <RepositoryExplorer tree={mockTree} />;
}
