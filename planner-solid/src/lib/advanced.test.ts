import { summarizeBlueprint, summarizeKnowledge } from "./advanced";
import type { BlueprintResponse } from "./types";

const blueprint = (overrides: Partial<BlueprintResponse> = {}): BlueprintResponse => ({
  total_nodes: 5,
  total_edges: 4,
  counts: {
    project: 1,
    decision: 1,
    component: 1,
    pattern: 1,
    technology: 1,
  },
  edges: [
    { source: "project-1", target: "decision-1", edge_type: "contains" },
    { source: "decision-1", target: "component-1", edge_type: "decided_by" },
  ],
  nodes: [
    {
      id: "project-1",
      name: "Personal Calendar",
      node_type: "project",
      status: "active",
      scope_class: "project",
      scope_visibility: "project_local",
      is_shared: false,
      lifecycle: "active",
      project_id: "project-1",
      project_name: "Personal Calendar",
      secondary_scope: {},
      linked_project_ids: [],
      tags: [],
      has_documentation: true,
      updated_at: "2026-03-24T04:00:00Z",
    },
    {
      id: "decision-1",
      name: "Web first",
      node_type: "decision",
      status: "accepted",
      scope_class: "project",
      scope_visibility: "project_local",
      is_shared: false,
      lifecycle: "active",
      project_id: "project-1",
      project_name: "Personal Calendar",
      secondary_scope: {},
      linked_project_ids: [],
      tags: [],
      has_documentation: true,
      updated_at: "2026-03-24T05:00:00Z",
    },
    {
      id: "component-1",
      name: "Task Service",
      node_type: "component",
      status: "planned",
      scope_class: "project_contextual",
      scope_visibility: "shared",
      is_shared: true,
      lifecycle: "active",
      project_id: "project-1",
      project_name: "Personal Calendar",
      secondary_scope: { component: "Task Service" },
      linked_project_ids: [],
      tags: [],
      has_documentation: false,
      updated_at: "2026-02-20T05:00:00Z",
    },
  ],
  ...overrides,
});

describe("advanced project helpers", () => {
  it("builds a compact knowledge summary from the project blueprint", () => {
    const summary = summarizeKnowledge(blueprint());
    expect(summary.totalNodes).toBe(3);
    expect(summary.documentedNodes).toBe(2);
    expect(summary.sharedNodes).toBe(1);
    expect(summary.featuredNodes[0]?.node_type).toBe("decision");
  });

  it("builds a structural blueprint summary for the attached advanced surface", () => {
    const summary = summarizeBlueprint(blueprint());
    expect(summary.totalNodes).toBe(3);
    expect(summary.totalEdges).toBe(4);
    expect(summary.decisionNodes).toBe(1);
    expect(summary.componentNodes).toBe(1);
    expect(summary.structuralNodes.length).toBeGreaterThan(0);
  });
});
