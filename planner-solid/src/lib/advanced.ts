import type { BlueprintResponse, NodeSummary } from "./types";

export type AdvancedPanelTab = "knowledge" | "blueprint";

export interface KnowledgeSummary {
  totalNodes: number;
  documentedNodes: number;
  sharedNodes: number;
  staleNodes: number;
  featuredNodes: NodeSummary[];
}

export interface BlueprintSummary {
  totalNodes: number;
  totalEdges: number;
  projectNodes: number;
  decisionNodes: number;
  componentNodes: number;
  structuralNodes: NodeSummary[];
}

const KNOWLEDGE_NODE_PRIORITY = [
  "decision",
  "component",
  "constraint",
  "pattern",
  "technology",
  "quality_requirement",
  "project",
];

function timestampValue(value: string | undefined | null): number {
  if (!value) return 0;
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? 0 : parsed;
}

function recentNodes(nodes: NodeSummary[]): NodeSummary[] {
  return [...nodes].sort((left, right) => timestampValue(right.updated_at) - timestampValue(left.updated_at));
}

export function summarizeKnowledge(blueprint: BlueprintResponse): KnowledgeSummary {
  const activeNodes = blueprint.nodes.filter(node => node.lifecycle !== "archived");
  const featuredNodes = recentNodes(activeNodes)
    .sort((left, right) => {
      const leftRank = KNOWLEDGE_NODE_PRIORITY.indexOf(left.node_type);
      const rightRank = KNOWLEDGE_NODE_PRIORITY.indexOf(right.node_type);
      return (leftRank === -1 ? 999 : leftRank) - (rightRank === -1 ? 999 : rightRank);
    })
    .slice(0, 5);

  const staleThreshold = Date.now() - 1000 * 60 * 60 * 24 * 30;

  return {
    totalNodes: activeNodes.length,
    documentedNodes: activeNodes.filter(node => node.has_documentation).length,
    sharedNodes: activeNodes.filter(node => node.scope_visibility === "shared").length,
    staleNodes: activeNodes.filter(node => timestampValue(node.updated_at) < staleThreshold).length,
    featuredNodes,
  };
}

export function summarizeBlueprint(blueprint: BlueprintResponse): BlueprintSummary {
  const activeNodes = blueprint.nodes.filter(node => node.lifecycle !== "archived");
  const structuralNodes = recentNodes(activeNodes)
    .filter(node => ["project", "decision", "component", "constraint"].includes(node.node_type))
    .slice(0, 5);

  return {
    totalNodes: activeNodes.length,
    totalEdges: blueprint.total_edges,
    projectNodes: activeNodes.filter(node => node.node_type === "project").length,
    decisionNodes: activeNodes.filter(node => node.node_type === "decision").length,
    componentNodes: activeNodes.filter(node => node.node_type === "component").length,
    structuralNodes,
  };
}
