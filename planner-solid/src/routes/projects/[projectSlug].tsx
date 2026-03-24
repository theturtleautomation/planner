import { Title } from "@solidjs/meta";
import { A, useNavigate, useParams } from "@solidjs/router";
import { For, Show, createMemo, createResource, createSignal } from "solid-js";

import { summarizeBlueprint, summarizeKnowledge, type AdvancedPanelTab } from "~/lib/advanced";
import { createProjectSession, getProject, getProjectBlueprint, listSessions } from "~/lib/api";
import { summarizeProjectWork } from "~/lib/projects";
import { presentSessionTitle } from "~/lib/workspace";

function formatTimestamp(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export default function ProjectWorkspacePage() {
  const params = useParams();
  const navigate = useNavigate();
  const [project] = createResource(() => params.projectSlug, getProject);
  const [sessions] = createResource(listSessions);
  const [advancedOpen, setAdvancedOpen] = createSignal(false);
  const [advancedTab, setAdvancedTab] = createSignal<AdvancedPanelTab>("knowledge");
  const [starting, setStarting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [projectBlueprint] = createResource(
    () => (advancedOpen() ? params.projectSlug : null),
    async projectSlug =>
      getProjectBlueprint(projectSlug, {
        includeShared: true,
        includeGlobal: false,
      }),
  );

  const projectSessions = createMemo(() => {
    const slug = params.projectSlug;
    const available = sessions()?.sessions ?? [];
    return available.filter(session => (session.project_slug ?? "") === slug && !session.archived);
  });

  const summary = createMemo(() => {
    const currentProject = project()?.project;
    if (!currentProject) return null;
    return summarizeProjectWork(currentProject, projectSessions());
  });
  const knowledgeSummary = createMemo(() => {
    const blueprint = projectBlueprint();
    return blueprint ? summarizeKnowledge(blueprint) : null;
  });
  const blueprintSummary = createMemo(() => {
    const blueprint = projectBlueprint();
    return blueprint ? summarizeBlueprint(blueprint) : null;
  });

  const handleStartAnalysis = async () => {
    const currentProject = project()?.project;
    if (!currentProject) return;
    setStarting(true);
    setError(null);

    try {
      const response = await createProjectSession(currentProject.slug, {
        title: `${currentProject.name} analysis`,
        description: currentProject.description ?? null,
      });
      navigate(`/sessions/${response.session.id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to start a new project analysis.");
      setStarting(false);
    }
  };

  return (
    <section class="page page-scroll">
      <Title>{project()?.project.name ?? "Project"}</Title>
      <div class="stack page-frame">
          <Show
          when={project()}
          fallback={<div class="empty-state">Loading project workspace…</div>}
        >
          {response => {
            const currentProject = () => response().project;
            const currentSummary = () => summary();
            const primarySession = () => currentSummary()?.primarySession ?? null;

            return (
              <>
                <section class="hero-panel workspace-hero">
                  <div class="eyebrow">Project workspace</div>
                  <h1 class="hero-title">{currentProject().name}</h1>
                  <p class="hero-copy">
                    {currentProject().description?.trim() ||
                      "Use this project as the stable container for deep Socratic analysis and the next build-shaping moves."}
                  </p>
                  <div class="hero-focus project-focus">
                    <div>
                      <div class="hero-focus-label">
                        {currentSummary()?.statusLabel ?? "Ready to start"}
                      </div>
                      <h2 class="hero-focus-title">
                        {primarySession()
                          ? presentSessionTitle(primarySession()!)
                          : "No active analysis yet"}
                      </h2>
                      <p class="hero-focus-copy">
                        {primarySession()?.project_description?.trim() ||
                          "Start a new Socratic analysis to shape this project's working truth."}
                      </p>
                    </div>
                    <div class="hero-actions">
                      <Show
                        when={primarySession()}
                        fallback={
                          <button class="btn btn-primary" type="button" disabled={starting()} onClick={handleStartAnalysis}>
                            {starting() ? "Starting…" : "Start analysis"}
                          </button>
                        }
                      >
                        {session => (
                          <>
                            <A class="btn btn-primary" href={`/sessions/${session().id}`}>
                              Continue analysis
                            </A>
                            <button class="btn btn-subtle" type="button" disabled={starting()} onClick={handleStartAnalysis}>
                              {starting() ? "Starting…" : "New analysis"}
                            </button>
                          </>
                        )}
                      </Show>
                    </div>
                  </div>
                  {error() ? <div class="error-copy">{error()}</div> : null}
                </section>

                <section class="section-panel">
                  <div class="section-head">
                    <div>
                      <div class="eyebrow">Recent project work</div>
                      <h2 class="section-title">Analysis sessions</h2>
                    </div>
                    <A class="btn btn-subtle" href="/sessions">
                      All sessions
                    </A>
                  </div>

                  <Show
                    when={projectSessions().length > 0}
                    fallback={<div class="empty-state">No sessions yet. Start the first analysis from this workspace.</div>}
                  >
                    <div class="project-list compact">
                      <For each={projectSessions().slice(0, 6)}>
                        {session => (
                          <A class="project-row" href={`/sessions/${session.id}`}>
                            <div class="project-row-main">
                              <div class="project-row-title">{presentSessionTitle(session)}</div>
                              <div class="project-row-copy">
                                {session.project_description?.trim() || "Project analysis session"}
                              </div>
                            </div>
                            <div class="project-row-facts">
                              <span>{session.intake_phase}</span>
                              <span>Updated {formatTimestamp(session.last_activity_at)}</span>
                            </div>
                          </A>
                        )}
                      </For>
                    </div>
                  </Show>
                </section>

                <details
                  class="advanced-panel"
                  ref={advancedDetails}
                  onToggle={() => setAdvancedOpen(advancedDetails?.open ?? false)}
                >
                  <summary>Advanced project surfaces</summary>
                  <div class="advanced-panel-body">
                    <div class="advanced-tab-row" role="tablist" aria-label="Project advanced surfaces">
                      <button
                        class={`advanced-tab${advancedTab() === "knowledge" ? " is-active" : ""}`}
                        type="button"
                        role="tab"
                        aria-selected={advancedTab() === "knowledge"}
                        onClick={() => setAdvancedTab("knowledge")}
                      >
                        Knowledge
                      </button>
                      <button
                        class={`advanced-tab${advancedTab() === "blueprint" ? " is-active" : ""}`}
                        type="button"
                        role="tab"
                        aria-selected={advancedTab() === "blueprint"}
                        onClick={() => setAdvancedTab("blueprint")}
                      >
                        Blueprint
                      </button>
                    </div>

                    <Show
                      when={projectBlueprint()}
                      fallback={<div class="advanced-loading">Loading project knowledge and structure…</div>}
                    >
                      <Show
                        when={advancedTab() === "knowledge"}
                        fallback={
                          <Show when={blueprintSummary()}>
                            {summary => (
                              <div class="advanced-surface">
                                <div class="advanced-summary-grid">
                                  <div class="advanced-summary-card">
                                    <div class="advanced-label">Nodes</div>
                                    <div class="advanced-metric">{summary().totalNodes}</div>
                                  </div>
                                  <div class="advanced-summary-card">
                                    <div class="advanced-label">Edges</div>
                                    <div class="advanced-metric">{summary().totalEdges}</div>
                                  </div>
                                  <div class="advanced-summary-card">
                                    <div class="advanced-label">Decisions</div>
                                    <div class="advanced-metric">{summary().decisionNodes}</div>
                                  </div>
                                  <div class="advanced-summary-card">
                                    <div class="advanced-label">Components</div>
                                    <div class="advanced-metric">{summary().componentNodes}</div>
                                  </div>
                                </div>
                                <div class="advanced-list">
                                  <For each={summary().structuralNodes}>
                                    {node => (
                                      <div class="advanced-list-row">
                                        <div>
                                          <div class="advanced-item-title">{node.name}</div>
                                          <div class="advanced-item-copy">
                                            {node.node_type} · {node.scope_visibility}
                                          </div>
                                        </div>
                                        <div class="advanced-item-meta">Updated {formatTimestamp(node.updated_at)}</div>
                                      </div>
                                    )}
                                  </For>
                                </div>
                              </div>
                            )}
                          </Show>
                        }
                      >
                        <Show when={knowledgeSummary()}>
                          {summary => (
                            <div class="advanced-surface">
                              <div class="advanced-summary-grid">
                                <div class="advanced-summary-card">
                                  <div class="advanced-label">Knowledge records</div>
                                  <div class="advanced-metric">{summary().totalNodes}</div>
                                </div>
                                <div class="advanced-summary-card">
                                  <div class="advanced-label">Documented</div>
                                  <div class="advanced-metric">{summary().documentedNodes}</div>
                                </div>
                                <div class="advanced-summary-card">
                                  <div class="advanced-label">Shared</div>
                                  <div class="advanced-metric">{summary().sharedNodes}</div>
                                </div>
                                <div class="advanced-summary-card">
                                  <div class="advanced-label">Stale</div>
                                  <div class="advanced-metric">{summary().staleNodes}</div>
                                </div>
                              </div>
                              <div class="advanced-list">
                                <For each={summary().featuredNodes}>
                                  {node => (
                                    <div class="advanced-list-row">
                                      <div>
                                        <div class="advanced-item-title">{node.name}</div>
                                        <div class="advanced-item-copy">
                                          {node.node_type} · {node.has_documentation ? "documented" : "needs docs"}
                                        </div>
                                      </div>
                                      <div class="advanced-item-meta">Updated {formatTimestamp(node.updated_at)}</div>
                                    </div>
                                  )}
                                </For>
                              </div>
                            </div>
                          )}
                        </Show>
                      </Show>
                    </Show>
                  </div>
                </details>
              </>
            );
          }}
        </Show>
      </div>
    </section>
  );
}
  let advancedDetails: HTMLDetailsElement | undefined;
