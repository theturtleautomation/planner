import { Title } from "@solidjs/meta";
import { createEffect, createMemo, createResource, createSignal, For, Show } from "solid-js";

import { getProjectBlueprint, listProjects } from "~/lib/api";

type GraphPoint = {
  x: number;
  y: number;
};

const TYPE_ORDER = [
  "project",
  "decision",
  "component",
  "technology",
  "constraint",
  "pattern",
  "quality_requirement",
];

const TYPE_LABELS: Record<string, string> = {
  project: "Project nodes",
  decision: "Decision nodes",
  component: "Component nodes",
  technology: "Technology nodes",
  constraint: "Constraint nodes",
  pattern: "Pattern nodes",
  quality_requirement: "Quality requirement nodes",
};

function typeRank(nodeType: string): number {
  const index = TYPE_ORDER.indexOf(nodeType);
  return index === -1 ? TYPE_ORDER.length : index;
}

export default function BlueprintPage() {
  const [selectedProject, setSelectedProject] = createSignal<string | null>(null);
  const [nodeTypeFilter, setNodeTypeFilter] = createSignal("all");
  const [selectedNodeId, setSelectedNodeId] = createSignal<string | null>(null);

  const [projects] = createResource(listProjects);
  createEffect(() => {
    if (!selectedProject() && (projects()?.projects.length ?? 0) > 0) {
      setSelectedProject(projects()!.projects[0]!.slug);
    }
  });

  const [blueprint] = createResource(
    () => selectedProject(),
    async projectRef =>
      projectRef
        ? getProjectBlueprint(projectRef, {
            includeShared: true,
            includeGlobal: false,
          })
        : null,
  );

  const activeNodes = createMemo(() =>
    (blueprint()?.nodes ?? [])
      .filter(node => node.lifecycle !== "archived")
      .sort((left, right) => {
        const typeDelta = typeRank(left.node_type) - typeRank(right.node_type);
        if (typeDelta !== 0) return typeDelta;
        return left.name.localeCompare(right.name);
      }),
  );

  const nodeTypes = createMemo(() => {
    const discovered = new Set(activeNodes().map(node => node.node_type));
    return ["all", ...Array.from(discovered)];
  });

  const filteredNodes = createMemo(() => {
    const filter = nodeTypeFilter();
    return activeNodes().filter(node => (filter === "all" ? true : node.node_type === filter));
  });

  const filteredNodeIds = createMemo(() => new Set(filteredNodes().map(node => node.id)));
  const filteredEdges = createMemo(() =>
    (blueprint()?.edges ?? []).filter(
      edge => filteredNodeIds().has(edge.source) && filteredNodeIds().has(edge.target),
    ),
  );

  createEffect(() => {
    const nodes = filteredNodes();
    if (nodes.length === 0) {
      setSelectedNodeId(null);
      return;
    }
    if (!selectedNodeId() || !nodes.some(node => node.id === selectedNodeId())) {
      setSelectedNodeId(nodes[0]!.id);
    }
  });

  const selectedNode = createMemo(() => filteredNodes().find(node => node.id === selectedNodeId()) ?? null);

  const layout = createMemo(() => {
    const nodes = filteredNodes();
    const groups = new Map<string, typeof nodes>();
    for (const node of nodes) {
      const existing = groups.get(node.node_type);
      if (existing) {
        existing.push(node);
      } else {
        groups.set(node.node_type, [node]);
      }
    }

    const positions = new Map<string, GraphPoint>();
    const orderedTypes = TYPE_ORDER.filter(type => groups.has(type));
    const width = 940;
    const height = Math.max(360, orderedTypes.length * 110 + 80);

    orderedTypes.forEach((type, groupIndex) => {
      const row = groups.get(type) ?? [];
      const rowY = 70 + groupIndex * 108;
      const horizontalSpacing = row.length > 1 ? 760 / (row.length - 1) : 0;
      row.forEach((node, index) => {
        const x = row.length === 1 ? 470 : 90 + index * horizontalSpacing;
        positions.set(node.id, { x, y: rowY });
      });
    });

    return {
      width,
      height,
      positions,
      orderedTypes,
      groupedCounts: orderedTypes.map(type => ({
        type,
        label: TYPE_LABELS[type] ?? type,
        count: groups.get(type)?.length ?? 0,
      })),
    };
  });

  const selectedProjectName = createMemo(
    () => projects()?.projects.find(project => project.slug === selectedProject())?.name ?? "Loading project…",
  );

  const relatedNodes = createMemo(() => {
    const current = selectedNode();
    if (!current) return [];
    const edges = filteredEdges().filter(edge => edge.source === current.id || edge.target === current.id);
    const ids = new Set<string>();
    for (const edge of edges) {
      ids.add(edge.source === current.id ? edge.target : edge.source);
    }
    return filteredNodes()
      .filter(node => ids.has(node.id))
      .slice(0, 6);
  });

  return (
    <section class="page page-scroll">
      <Title>Blueprint</Title>
      <div class="stack page-frame">
        <section class="hero-panel workspace-hero">
          <div class="eyebrow">Blueprint graph</div>
          <h1 class="hero-title">Blueprint</h1>
          <p class="hero-copy">
            Inspect project structure as a graph first. Counts, filters, and node detail stay attached so the route reads as a structural workspace instead of a generic inventory.
          </p>
          <div class="hero-focus project-focus">
            <div>
              <div class="hero-focus-label">Current project graph</div>
              <h2 class="hero-focus-title">{selectedProjectName()}</h2>
              <p class="hero-focus-copy">
                {filteredNodes().length} visible node{filteredNodes().length === 1 ? "" : "s"} and {filteredEdges().length} visible edge{filteredEdges().length === 1 ? "" : "s"} in the current slice.
              </p>
            </div>
          </div>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Scope and graph slice</div>
              <h2 class="section-title">Blueprint controls</h2>
            </div>
          </div>
          <div class="knowledge-toolbar">
            <label class="timeline-limit-field">
              <span>Project</span>
              <select
                value={selectedProject() ?? ""}
                onInput={event => setSelectedProject(event.currentTarget.value)}
              >
                <For each={projects()?.projects ?? []}>
                  {project => <option value={project.slug}>{project.name}</option>}
                </For>
              </select>
            </label>
            <label class="timeline-limit-field">
              <span>Node type</span>
              <select value={nodeTypeFilter()} onInput={event => setNodeTypeFilter(event.currentTarget.value)}>
                <For each={nodeTypes()}>{type => <option value={type}>{type}</option>}</For>
              </select>
            </label>
          </div>
        </section>

        <section class="section-panel blueprint-layout">
          <div class="blueprint-canvas-panel">
            <div class="section-head">
              <div>
                <div class="eyebrow">Primary structure</div>
                <h2 class="section-title">Graph canvas</h2>
                <p class="section-copy">Use the graph as the main reading target, then inspect the attached node detail beside it.</p>
              </div>
            </div>
            <Show when={!blueprint.loading} fallback={<div class="advanced-loading">Loading blueprint graph…</div>}>
              <Show
                when={filteredNodes().length > 0}
                fallback={<div class="empty-state">No blueprint nodes match the current project and graph filter.</div>}
              >
                <div class="blueprint-canvas-wrap" data-testid="blueprint-graph-canvas">
                  <svg
                    class="blueprint-svg"
                    viewBox={`0 0 ${layout().width} ${layout().height}`}
                    role="img"
                    aria-label="Blueprint graph"
                  >
                    <For each={layout().orderedTypes}>
                      {type => (
                        <>
                          <line
                            x1="48"
                            x2={String(layout().width - 48)}
                            y1={String((layout().positions.get(groupsFirstNode(filteredNodes(), type)?.id ?? "")?.y ?? 40) + 28)}
                            y2={String((layout().positions.get(groupsFirstNode(filteredNodes(), type)?.id ?? "")?.y ?? 40) + 28)}
                            class="blueprint-row-rule"
                          />
                          <text
                            x="48"
                            y={String((layout().positions.get(groupsFirstNode(filteredNodes(), type)?.id ?? "")?.y ?? 40) - 18)}
                            class="blueprint-row-label"
                          >
                            {TYPE_LABELS[type] ?? type}
                          </text>
                        </>
                      )}
                    </For>
                    <For each={filteredEdges()}>
                      {edge => {
                        const source = layout().positions.get(edge.source);
                        const target = layout().positions.get(edge.target);
                        if (!source || !target) return null;
                        return (
                          <line
                            x1={String(source.x)}
                            y1={String(source.y)}
                            x2={String(target.x)}
                            y2={String(target.y)}
                            class="blueprint-edge-line"
                          />
                        );
                      }}
                    </For>
                    <For each={filteredNodes()}>
                      {node => {
                        const point = layout().positions.get(node.id);
                        if (!point) return null;
                        const active = selectedNodeId() === node.id;
                        return (
                          <g
                            class={`blueprint-node${active ? " is-active" : ""}`}
                            data-node-id={node.id}
                            onClick={() => setSelectedNodeId(node.id)}
                          >
                            <circle cx={String(point.x)} cy={String(point.y)} r="22" />
                            <text x={String(point.x)} y={String(point.y - 30)} text-anchor="middle">
                              {node.name}
                            </text>
                          </g>
                        );
                      }}
                    </For>
                  </svg>
                </div>
              </Show>
            </Show>
          </div>

          <div class="blueprint-inspector-panel">
            <div class="section-head">
              <div>
                <div class="eyebrow">Attached inspection</div>
                <h2 class="section-title">Selected node</h2>
              </div>
            </div>
            <Show
              when={selectedNode()}
              fallback={<div class="empty-state">Select a node in the graph to inspect the current structure.</div>}
            >
              {node => (
                <div class="advanced-column-panel">
                  <div>
                    <div class="advanced-item-title">{node().name}</div>
                    <div class="advanced-item-copy">
                      {node().node_type} · {node().status} · {node().scope_visibility}
                    </div>
                  </div>
                  <div class="blueprint-inspector-grid">
                    <div>
                      <div class="advanced-label">Project</div>
                      <div class="advanced-value">{node().project_name ?? "Project local"}</div>
                    </div>
                    <div>
                      <div class="advanced-label">Linked projects</div>
                      <div class="advanced-value">
                        {node().linked_project_ids.length > 0 ? node().linked_project_ids.join(", ") : "None"}
                      </div>
                    </div>
                    <div>
                      <div class="advanced-label">Tags</div>
                      <div class="advanced-value">{node().tags.length > 0 ? node().tags.join(", ") : "No tags"}</div>
                    </div>
                    <div>
                      <div class="advanced-label">Documentation</div>
                      <div class="advanced-value">{node().has_documentation ? "Attached" : "Missing"}</div>
                    </div>
                  </div>

                  <div>
                    <div class="advanced-label">Connected structure</div>
                    <Show
                      when={relatedNodes().length > 0}
                      fallback={<div class="advanced-value">No directly connected nodes in the current graph slice.</div>}
                    >
                      <div class="advanced-list compact">
                        <For each={relatedNodes()}>
                          {related => (
                            <button
                              class={`knowledge-row${selectedNodeId() === related.id ? " is-active" : ""}`}
                              type="button"
                              onClick={() => setSelectedNodeId(related.id)}
                            >
                              <div>
                                <div class="advanced-item-title">{related.name}</div>
                                <div class="advanced-item-copy">{related.node_type}</div>
                              </div>
                              <span class="pill">{related.status}</span>
                            </button>
                          )}
                        </For>
                      </div>
                    </Show>
                  </div>
                </div>
              )}
            </Show>
          </div>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Structural highlights</div>
              <h2 class="section-title">Graph summary</h2>
            </div>
          </div>
          <div class="blueprint-summary-grid">
            <div class="advanced-column-panel">
              <div class="advanced-label">Visible graph counts</div>
              <div class="project-pill-row">
                <div class="pill">{filteredNodes().length} nodes</div>
                <div class="pill">{filteredEdges().length} edges</div>
              </div>
              <div class="advanced-list compact">
                <For each={layout().groupedCounts}>
                  {group => (
                    <div class="advanced-list-row">
                      <div>
                        <div class="advanced-item-title">{group.label}</div>
                        <div class="advanced-item-copy">{group.count} structural item{group.count === 1 ? "" : "s"}</div>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </div>
            <div class="advanced-column-panel">
              <div class="advanced-label">Visible nodes</div>
              <div class="advanced-list compact">
                <For each={filteredNodes()}>
                  {node => (
                    <button
                      class={`knowledge-row${selectedNodeId() === node.id ? " is-active" : ""}`}
                      type="button"
                      onClick={() => setSelectedNodeId(node.id)}
                    >
                      <div>
                        <div class="advanced-item-title">{node.name}</div>
                        <div class="advanced-item-copy">
                          {node.node_type} · {node.scope_visibility}
                        </div>
                      </div>
                      <span class="pill">{node.status}</span>
                    </button>
                  )}
                </For>
              </div>
            </div>
          </div>
        </section>
      </div>
    </section>
  );
}

function groupsFirstNode<T extends { id: string; node_type: string }>(nodes: T[], type: string): T | null {
  return nodes.find(node => node.node_type === type) ?? null;
}
