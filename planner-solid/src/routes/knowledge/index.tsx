import { Title } from "@solidjs/meta";
import { createEffect, createMemo, createResource, createSignal, For, Show } from "solid-js";

import { getProjectBlueprint, listProjects } from "~/lib/api";

export default function KnowledgePage() {
  const [selectedProject, setSelectedProject] = createSignal<string | null>(null);
  const [search, setSearch] = createSignal("");
  const [nodeType, setNodeType] = createSignal("all");
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

  const nodeTypes = createMemo(() => {
    const types = new Set((blueprint()?.nodes ?? []).map(node => node.node_type));
    return ["all", ...Array.from(types).sort()];
  });

  const filteredNodes = createMemo(() => {
    const term = search().trim().toLowerCase();
    return (blueprint()?.nodes ?? []).filter(node => {
      if (nodeType() !== "all" && node.node_type !== nodeType()) return false;
      if (!term) return true;
      return (
        node.name.toLowerCase().includes(term) ||
        node.node_type.toLowerCase().includes(term) ||
        (node.project_name ?? "").toLowerCase().includes(term)
      );
    });
  });

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

  return (
    <section class="page page-scroll">
      <Title>Knowledge</Title>
      <div class="stack page-frame">
        <section class="hero-panel workspace-hero">
          <div class="eyebrow">Knowledge inventory</div>
          <h1 class="hero-title">Knowledge</h1>
          <p class="hero-copy">
            Browse the captured project truth as an inventory first. Filters stay visible, and selected-node detail remains attached instead of competing with the list.
          </p>
          <div class="hero-focus project-focus">
            <div>
              <div class="hero-focus-label">Active project scope</div>
              <h2 class="hero-focus-title">
                {projects()?.projects.find(project => project.slug === selectedProject())?.name ?? "Loading project…"}
              </h2>
              <p class="hero-focus-copy">
                {filteredNodes().length} visible node{filteredNodes().length === 1 ? "" : "s"} in the current inventory slice.
              </p>
            </div>
          </div>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Scope and filters</div>
              <h2 class="section-title">Inventory controls</h2>
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
            <label class="timeline-limit-field knowledge-search">
              <span>Search</span>
              <input
                type="text"
                value={search()}
                placeholder="Find by name or type"
                onInput={event => setSearch(event.currentTarget.value)}
              />
            </label>
            <label class="timeline-limit-field">
              <span>Type</span>
              <select value={nodeType()} onInput={event => setNodeType(event.currentTarget.value)}>
                <For each={nodeTypes()}>{type => <option value={type}>{type}</option>}</For>
              </select>
            </label>
          </div>
        </section>

        <section class="section-panel knowledge-layout">
          <div class="knowledge-list-panel">
            <div class="section-head">
              <div>
                <div class="eyebrow">Inventory</div>
                <h2 class="section-title">Nodes</h2>
              </div>
            </div>
            <Show when={!blueprint.loading} fallback={<div class="advanced-loading">Loading knowledge inventory…</div>}>
              <Show
                when={filteredNodes().length > 0}
                fallback={<div class="empty-state">No nodes match the current project and filter slice.</div>}
              >
                <div class="advanced-list">
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
              </Show>
            </Show>
          </div>

          <div class="knowledge-detail-panel">
            <div class="section-head">
              <div>
                <div class="eyebrow">Attached detail</div>
                <h2 class="section-title">Selected node</h2>
              </div>
            </div>
            <Show
              when={selectedNode()}
              fallback={<div class="empty-state">Select a node to inspect its attached detail.</div>}
            >
              {node => (
                <div class="advanced-column-panel">
                  <div>
                    <div class="advanced-item-title">{node().name}</div>
                    <div class="advanced-item-copy">
                      {node().node_type} · {node().status} · {node().scope_visibility}
                    </div>
                  </div>
                  <div>
                    <div class="advanced-label">Project</div>
                    <div class="advanced-value">{node().project_name ?? "Project local"}</div>
                  </div>
                  <div>
                    <div class="advanced-label">Tags</div>
                    <div class="advanced-value">{node().tags.length > 0 ? node().tags.join(", ") : "No tags"}</div>
                  </div>
                  <div>
                    <div class="advanced-label">Linked projects</div>
                    <div class="advanced-value">
                      {node().linked_project_ids.length > 0 ? node().linked_project_ids.join(", ") : "None"}
                    </div>
                  </div>
                </div>
              )}
            </Show>
          </div>
        </section>
      </div>
    </section>
  );
}
