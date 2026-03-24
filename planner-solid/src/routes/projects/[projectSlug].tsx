import { Title } from "@solidjs/meta";
import { A, useNavigate, useParams } from "@solidjs/router";
import { For, Match, Show, Switch, createMemo, createResource, createSignal } from "solid-js";

import {
  applyProjectImportReview,
  createProjectSession,
  getProject,
  getProjectBlueprint,
  getProjectImportReview,
  getProjectImportState,
  listBlueprintExportHistory,
  getSessionEvents,
  getSessionRuns,
  getPromptBank,
  listSessions,
  updateProjectImportReviewSelection,
} from "~/lib/api";
import {
  summarizeBlueprint,
  summarizeBuildPath,
  summarizeBuildExecution,
  summarizeBuildReadiness,
  summarizeKnowledge,
  summarizeOutputArtifacts,
  summarizeProjectActivity,
  summarizeReview,
  type AdvancedPanelTab,
} from "~/lib/advanced";
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

function readinessBadgeClass(state: "ready" | "needs-review" | "in-progress" | "not-started"): string {
  switch (state) {
    case "ready":
      return "state-badge is-active";
    case "needs-review":
      return "state-badge is-attention";
    case "in-progress":
      return "state-badge is-recent";
    default:
      return "state-badge is-quiet";
  }
}

function reviewBadgeClass(state: "pending" | "quiet" | "applied"): string {
  switch (state) {
    case "pending":
      return "state-badge is-attention";
    case "applied":
      return "state-badge is-active";
    default:
      return "state-badge is-quiet";
  }
}

export default function ProjectWorkspacePage() {
  const params = useParams();
  const navigate = useNavigate();
  let advancedDetails: HTMLDetailsElement | undefined;

  const [project] = createResource(() => params.projectSlug, getProject);
  const [sessions] = createResource(listSessions);
  const [advancedOpen, setAdvancedOpen] = createSignal(false);
  const [advancedTab, setAdvancedTab] = createSignal<AdvancedPanelTab>("review");
  const [starting, setStarting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [reviewError, setReviewError] = createSignal<string | null>(null);
  const [applyPending, setApplyPending] = createSignal(false);
  const [selectionPendingNodeId, setSelectionPendingNodeId] = createSignal<string | null>(null);

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

  const primarySession = createMemo(() => summary()?.primarySession ?? null);
  const currentProjectId = createMemo(() => project()?.project.id);

  const [projectBlueprint] = createResource(
    () => (advancedOpen() ? params.projectSlug : undefined),
    async projectSlug => {
      if (!projectSlug) return null;
      return getProjectBlueprint(projectSlug, {
        includeShared: true,
        includeGlobal: false,
      });
    },
  );
  const [projectImportState, { refetch: refetchImportState }] = createResource(
    () => (advancedOpen() ? params.projectSlug : undefined),
    async projectSlug => (projectSlug ? getProjectImportState(projectSlug) : null),
  );
  const [projectImportReview, { refetch: refetchImportReview }] = createResource(
    () => (advancedOpen() ? params.projectSlug : undefined),
    async projectSlug => (projectSlug ? getProjectImportReview(projectSlug) : null),
  );
  const [promptBank, { refetch: refetchPromptBank }] = createResource(
    () => (advancedOpen() && primarySession()?.id ? primarySession()!.id : undefined),
    async sessionId => (sessionId ? getPromptBank(sessionId) : null),
  );
  const [sessionRuns] = createResource(
    () => (advancedOpen() && primarySession()?.id ? primarySession()!.id : undefined),
    async sessionId => (sessionId ? getSessionRuns(sessionId) : null),
  );
  const [sessionEvents] = createResource(
    () => (advancedOpen() && primarySession()?.id ? primarySession()!.id : undefined),
    async sessionId =>
      sessionId
        ? getSessionEvents(sessionId, {
            source: "pipeline",
            limit: 5,
          })
        : null,
  );
  const [exportHistory] = createResource(
    () => (advancedOpen() ? currentProjectId() : undefined),
    async projectId => (projectId ? listBlueprintExportHistory({ projectId, limit: 6 }) : null),
  );

  const knowledgeSummary = createMemo(() => {
    const blueprint = projectBlueprint();
    return blueprint ? summarizeKnowledge(blueprint) : null;
  });
  const blueprintSummary = createMemo(() => {
    const blueprint = projectBlueprint();
    return blueprint ? summarizeBlueprint(blueprint) : null;
  });
  const reviewSummary = createMemo(() =>
    summarizeReview({
      importReview: projectImportReview(),
      importState: projectImportState(),
      promptBank: promptBank(),
    }),
  );
  const buildReadiness = createMemo(() =>
    summarizeBuildReadiness({
      primarySession: primarySession(),
      promptBank: promptBank(),
      importState: projectImportState(),
      importReview: projectImportReview(),
      blueprintSummary: blueprintSummary(),
    }),
  );
  const buildPath = createMemo(() =>
    summarizeBuildPath({
      projectName: project()?.project.name ?? "Project",
      primarySession: primarySession(),
      readiness: buildReadiness(),
      promptBank: promptBank(),
      blueprintSummary: blueprintSummary(),
    }),
  );
  const activitySummary = createMemo(() =>
    summarizeProjectActivity({
      sessions: projectSessions(),
      importState: projectImportState(),
      promptBank: promptBank(),
      buildPath: buildPath(),
    }),
  );
  const buildExecution = createMemo(() =>
    summarizeBuildExecution({
      primarySession: primarySession(),
      runs: sessionRuns(),
      events: sessionEvents()?.events ?? [],
    }),
  );
  const outputArtifacts = createMemo(() =>
    summarizeOutputArtifacts({
      projectName: project()?.project.name ?? "Project",
      history: exportHistory(),
    }),
  );

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

  const handleSetImportNodeIncluded = async (nodeId: string, included: boolean) => {
    if (!params.projectSlug) return;
    setSelectionPendingNodeId(nodeId);
    setReviewError(null);
    try {
      await updateProjectImportReviewSelection(params.projectSlug, { nodeId, included });
      await Promise.all([refetchImportReview(), refetchImportState()]);
    } catch (err) {
      setReviewError(err instanceof Error ? err.message : "Unable to update review selection.");
    } finally {
      setSelectionPendingNodeId(null);
    }
  };

  const handleApplyImportReview = async () => {
    if (!params.projectSlug) return;
    setApplyPending(true);
    setReviewError(null);
    try {
      await applyProjectImportReview(params.projectSlug);
      await Promise.all([refetchImportReview(), refetchImportState(), refetchPromptBank()]);
    } catch (err) {
      setReviewError(err instanceof Error ? err.message : "Unable to apply the import review.");
    } finally {
      setApplyPending(false);
    }
  };

  return (
    <section class="page page-scroll">
      <Title>{project()?.project.name ?? "Project"}</Title>
      <div class="stack page-frame">
        <Show when={project()} fallback={<div class="empty-state">Loading project workspace…</div>}>
          {response => {
            const currentProject = () => response().project;
            const currentSummary = () => summary();
            const activeSession = () => primarySession();
            const currentPromptBank = () => promptBank();
            const currentReview = () => projectImportReview();
            const currentReadiness = () => buildReadiness();
            const currentBuildPath = () => buildPath();
            const currentActivity = () => activitySummary();
            const currentBuildExecution = () => buildExecution();

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
                        {activeSession()
                          ? presentSessionTitle(activeSession()!)
                          : "No active analysis yet"}
                      </h2>
                      <p class="hero-focus-copy">
                        {activeSession()?.project_description?.trim() ||
                          "Start a new Socratic analysis to shape this project's working truth."}
                      </p>
                    </div>
                    <div class="hero-actions">
                      <span class={readinessBadgeClass(currentReadiness().state)}>
                        {currentReadiness().label}
                      </span>
                      <Show
                        when={activeSession()}
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
                  ref={element => {
                    advancedDetails = element;
                  }}
                  onToggle={() => {
                    const open = advancedDetails?.open ?? false;
                    setAdvancedOpen(open);
                    if (open) {
                      setAdvancedTab(current =>
                        current === "knowledge" || current === "blueprint" ? "review" : current,
                      );
                    }
                  }}
                >
                  <summary>Project review, readiness, and advanced surfaces</summary>
                  <div class="advanced-panel-body">
                    <div class="advanced-tab-row" role="tablist" aria-label="Project attached surfaces">
                      <button
                        class={`advanced-tab${advancedTab() === "review" ? " is-active" : ""}`}
                        type="button"
                        role="tab"
                        aria-selected={advancedTab() === "review"}
                        onClick={() => setAdvancedTab("review")}
                      >
                        Review
                      </button>
                      <button
                        class={`advanced-tab${advancedTab() === "readiness" ? " is-active" : ""}`}
                        type="button"
                        role="tab"
                        aria-selected={advancedTab() === "readiness"}
                        onClick={() => setAdvancedTab("readiness")}
                      >
                        Build readiness
                      </button>
                      <button
                        class={`advanced-tab${advancedTab() === "build" ? " is-active" : ""}`}
                        type="button"
                        role="tab"
                        aria-selected={advancedTab() === "build"}
                        onClick={() => setAdvancedTab("build")}
                      >
                        Build path
                      </button>
                      <button
                        class={`advanced-tab${advancedTab() === "execution" ? " is-active" : ""}`}
                        type="button"
                        role="tab"
                        aria-selected={advancedTab() === "execution"}
                        onClick={() => setAdvancedTab("execution")}
                      >
                        Build execution
                      </button>
                      <button
                        class={`advanced-tab${advancedTab() === "outputs" ? " is-active" : ""}`}
                        type="button"
                        role="tab"
                        aria-selected={advancedTab() === "outputs"}
                        onClick={() => setAdvancedTab("outputs")}
                      >
                        Outputs
                      </button>
                      <button
                        class={`advanced-tab${advancedTab() === "activity" ? " is-active" : ""}`}
                        type="button"
                        role="tab"
                        aria-selected={advancedTab() === "activity"}
                        onClick={() => setAdvancedTab("activity")}
                      >
                        Activity
                      </button>
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

                    <Switch>
                      <Match when={advancedTab() === "review"}>
                        <Show
                          when={!projectImportReview.loading && !projectImportState.loading && !promptBank.loading}
                          fallback={<div class="advanced-loading">Loading project review state…</div>}
                        >
                          <div class="advanced-surface">
                          <div class="advanced-surface-head">
                            <div>
                              <div class="eyebrow">Project review</div>
                              <h3 class="advanced-surface-title">{reviewSummary().headline}</h3>
                              <p class="section-copy">{reviewSummary().copy}</p>
                            </div>
                            <span class={reviewBadgeClass(reviewSummary().state)}>
                              {reviewSummary().state === "pending"
                                ? "Pending review"
                                : reviewSummary().state === "applied"
                                  ? "Review applied"
                                  : "Quiet"}
                            </span>
                          </div>
                          <div class="advanced-summary-grid">
                            <div class="advanced-summary-card">
                              <div class="advanced-label">Pending</div>
                              <div class="advanced-metric">{reviewSummary().pendingCount}</div>
                            </div>
                            <div class="advanced-summary-card">
                              <div class="advanced-label">Completed</div>
                              <div class="advanced-metric">{reviewSummary().completedCount}</div>
                            </div>
                            <div class="advanced-summary-card">
                              <div class="advanced-label">Queued analysis</div>
                              <div class="advanced-metric">{currentPromptBank()?.queued_threads.length ?? 0}</div>
                            </div>
                            <div class="advanced-summary-card">
                              <div class="advanced-label">Build posture</div>
                              <div class="advanced-metric advanced-metric-text">{currentReadiness().label}</div>
                            </div>
                          </div>

                          {reviewError() ? <div class="error-copy">{reviewError()}</div> : null}

                          <Show when={currentReview()?.import_job.status === "review_pending"}>
                            <div class="advanced-action-row">
                              <Show when={currentReview()?.import_job.seed_session_id}>
                                {seedSessionId => (
                                  <A class="btn btn-subtle" href={`/sessions/${seedSessionId()}`}>
                                    Open seeded session
                                  </A>
                                )}
                              </Show>
                              <button class="btn btn-primary" type="button" disabled={applyPending()} onClick={handleApplyImportReview}>
                                {applyPending() ? "Applying…" : "Apply import review"}
                              </button>
                            </div>
                          </Show>

                          <Show
                            when={currentReview()?.import_job.status === "review_pending" && (currentReview()?.review_nodes?.length ?? 0) > 0}
                            fallback={
                              <Show
                                when={reviewSummary().rows.length > 0}
                                fallback={<div class="empty-state">No project-local review queue is open right now.</div>}
                              >
                                <div class="advanced-list">
                                  <For each={reviewSummary().rows}>
                                    {row => (
                                      <div class="advanced-list-row">
                                        <div>
                                          <div class="advanced-item-title">{row.title}</div>
                                          <div class="advanced-item-copy">{row.copy}</div>
                                        </div>
                                        <div class="advanced-item-meta">{row.meta}</div>
                                      </div>
                                    )}
                                  </For>
                                </div>
                              </Show>
                            }
                          >
                            <div class="advanced-list">
                              <For each={currentReview()?.review_nodes ?? []}>
                                {node => (
                                  <div class="advanced-list-row advanced-list-row-action">
                                    <div>
                                      <div class="advanced-item-title">{node.node_name}</div>
                                      <div class="advanced-item-copy">
                                        {node.node_type} · {node.included ? "included in apply" : "excluded from apply"}
                                      </div>
                                    </div>
                                    <button
                                      class="btn btn-subtle"
                                      type="button"
                                      disabled={selectionPendingNodeId() === node.node_id}
                                      onClick={() => void handleSetImportNodeIncluded(node.node_id, !node.included)}
                                    >
                                      {selectionPendingNodeId() === node.node_id
                                        ? node.included
                                          ? "Excluding…"
                                          : "Including…"
                                        : node.included
                                          ? "Exclude"
                                          : "Include"}
                                    </button>
                                  </div>
                                )}
                              </For>
                            </div>
                          </Show>
                          </div>
                        </Show>
                      </Match>

                      <Match when={advancedTab() === "readiness"}>
                        <Show
                          when={!projectImportReview.loading && !projectImportState.loading && !promptBank.loading}
                          fallback={<div class="advanced-loading">Loading build-readiness state…</div>}
                        >
                          <div class="advanced-surface">
                          <div class="advanced-surface-head">
                            <div>
                              <div class="eyebrow">Build readiness</div>
                              <h3 class="advanced-surface-title">{currentReadiness().headline}</h3>
                              <p class="section-copy">{currentReadiness().nextAction}</p>
                            </div>
                            <span class={readinessBadgeClass(currentReadiness().state)}>
                              {currentReadiness().label}
                            </span>
                          </div>
                          <div class="advanced-summary-grid">
                            <div class="advanced-summary-card">
                              <div class="advanced-label">Banked threads</div>
                              <div class="advanced-metric">{currentPromptBank()?.banked_threads.length ?? 0}</div>
                            </div>
                            <div class="advanced-summary-card">
                              <div class="advanced-label">Queued threads</div>
                              <div class="advanced-metric">{currentPromptBank()?.queued_threads.length ?? 0}</div>
                            </div>
                            <div class="advanced-summary-card">
                              <div class="advanced-label">Blockers</div>
                              <div class="advanced-metric">{currentReadiness().blockers.length}</div>
                            </div>
                            <div class="advanced-summary-card">
                              <div class="advanced-label">Confirmations</div>
                              <div class="advanced-metric">{currentReadiness().confirmations.length}</div>
                            </div>
                          </div>
                          <div class="advanced-grid">
                            <div class="advanced-column-panel">
                              <div class="advanced-label">Blockers</div>
                              <Show
                                when={currentReadiness().blockers.length > 0}
                                fallback={<div class="advanced-value">No explicit blockers are currently registered.</div>}
                              >
                                <ul class="advanced-bullet-list">
                                  <For each={currentReadiness().blockers}>
                                    {blocker => <li>{blocker}</li>}
                                  </For>
                                </ul>
                              </Show>
                            </div>
                            <div class="advanced-column-panel">
                              <div class="advanced-label">Confirmations</div>
                              <Show
                                when={currentReadiness().confirmations.length > 0}
                                fallback={<div class="advanced-value">Explicit confirmations will appear as analysis and review settle.</div>}
                              >
                                <ul class="advanced-bullet-list">
                                  <For each={currentReadiness().confirmations}>
                                    {confirmation => <li>{confirmation}</li>}
                                  </For>
                                </ul>
                              </Show>
                            </div>
                          </div>
                          </div>
                        </Show>
                      </Match>

                      <Match when={advancedTab() === "knowledge"}>
                        <Show when={knowledgeSummary()} fallback={<div class="advanced-loading">Loading project knowledge…</div>}>
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
                      </Match>

                      <Match when={advancedTab() === "build"}>
                        <Show
                          when={!projectImportReview.loading && !projectImportState.loading && !promptBank.loading}
                          fallback={<div class="advanced-loading">Loading build handoff state…</div>}
                        >
                          <div class="advanced-surface">
                            <div class="advanced-surface-head">
                              <div>
                                <div class="eyebrow">Build path</div>
                                <h3 class="advanced-surface-title">{currentBuildPath().headline}</h3>
                                <p class="section-copy">{currentBuildPath().nextAction}</p>
                              </div>
                              <span class={readinessBadgeClass(
                                currentBuildPath().state === "ready"
                                  ? "ready"
                                  : currentBuildPath().state === "blocked"
                                    ? "needs-review"
                                    : currentBuildPath().state === "staging"
                                      ? "in-progress"
                                      : "not-started",
                              )}>
                                {currentBuildPath().label}
                              </span>
                            </div>
                            <div class="advanced-grid">
                              <div class="advanced-column-panel">
                                <div class="advanced-label">Handoff target</div>
                                <div class="advanced-value">
                                  {currentBuildPath().handoffTarget || "Project handoff target is still being assembled."}
                                </div>
                              </div>
                              <div class="advanced-column-panel">
                                <div class="advanced-label">Next move</div>
                                <div class="advanced-value">{currentBuildPath().nextAction}</div>
                                <div class="advanced-action-row">
                                  <Show when={activeSession()}>
                                    {session => (
                                      <A class="btn btn-subtle" href={`/sessions/${session().id}`}>
                                        Continue analysis
                                      </A>
                                    )}
                                  </Show>
                                  <button class="btn btn-subtle" type="button" onClick={() => setAdvancedTab("readiness")}>
                                    Open build readiness
                                  </button>
                                </div>
                              </div>
                            </div>
                            <div class="advanced-grid">
                              <div class="advanced-column-panel">
                                <div class="advanced-label">Blockers</div>
                                <Show
                                  when={currentBuildPath().blockers.length > 0}
                                  fallback={<div class="advanced-value">No explicit build handoff blockers are currently registered.</div>}
                                >
                                  <ul class="advanced-bullet-list">
                                    <For each={currentBuildPath().blockers}>
                                      {blocker => <li>{blocker}</li>}
                                    </For>
                                  </ul>
                                </Show>
                              </div>
                              <div class="advanced-column-panel">
                                <div class="advanced-label">Confirmations</div>
                                <Show
                                  when={currentBuildPath().confirmations.length > 0}
                                  fallback={<div class="advanced-value">The handoff summary will list confirmations as the project settles.</div>}
                                >
                                  <ul class="advanced-bullet-list">
                                    <For each={currentBuildPath().confirmations}>
                                      {confirmation => <li>{confirmation}</li>}
                                    </For>
                                  </ul>
                                </Show>
                              </div>
                            </div>
                          </div>
                        </Show>
                      </Match>

                      <Match when={advancedTab() === "activity"}>
                        <Show
                          when={!projectImportReview.loading && !projectImportState.loading && !promptBank.loading}
                          fallback={<div class="advanced-loading">Loading project activity…</div>}
                        >
                          <div class="advanced-surface">
                            <div class="advanced-surface-head">
                              <div>
                                <div class="eyebrow">Project activity</div>
                                <h3 class="advanced-surface-title">{currentActivity().headline}</h3>
                                <p class="section-copy">{currentActivity().copy}</p>
                              </div>
                              <span class="state-badge is-recent">Attached timeline</span>
                            </div>
                            <Show
                              when={currentActivity().items.length > 0}
                              fallback={<div class="empty-state">Project-local activity will appear here once the project starts moving.</div>}
                            >
                              <div class="advanced-list">
                                <For each={currentActivity().items}>
                                  {item => (
                                    <div class="advanced-list-row">
                                      <div>
                                        <div class="advanced-item-title">{item.title}</div>
                                        <div class="advanced-item-copy">{item.copy}</div>
                                      </div>
                                      <div class="advanced-item-meta">{item.meta}</div>
                                    </div>
                                  )}
                                </For>
                              </div>
                            </Show>
                          </div>
                        </Show>
                      </Match>

                      <Match when={advancedTab() === "execution"}>
                        <Show
                          when={!sessionRuns.loading && !sessionEvents.loading}
                          fallback={<div class="advanced-loading">Loading build execution…</div>}
                        >
                          <div class="advanced-surface">
                            <div class="advanced-surface-head">
                              <div>
                                <div class="eyebrow">Build execution</div>
                                <h3 class="advanced-surface-title">{currentBuildExecution().headline}</h3>
                                <p class="section-copy">{currentBuildExecution().nextAction}</p>
                              </div>
                              <span
                                class={readinessBadgeClass(
                                  currentBuildExecution().state === "active"
                                    ? "in-progress"
                                    : currentBuildExecution().state === "failed"
                                      ? "needs-review"
                                      : currentBuildExecution().state === "complete"
                                        ? "ready"
                                        : "not-started",
                                )}
                              >
                                {currentBuildExecution().label}
                              </span>
                            </div>
                            <div class="advanced-summary-grid">
                              <div class="advanced-summary-card">
                                <div class="advanced-label">Runs</div>
                                <div class="advanced-metric">{currentBuildExecution().runCount}</div>
                              </div>
                              <div class="advanced-summary-card">
                                <div class="advanced-label">Latest run</div>
                                <div class="advanced-metric advanced-metric-text">
                                  {currentBuildExecution().latestRunId
                                    ? currentBuildExecution().latestRunId!.slice(0, 8)
                                    : "n/a"}
                                </div>
                              </div>
                              <div class="advanced-summary-card">
                                <div class="advanced-label">Runtime state</div>
                                <div class="advanced-metric advanced-metric-text">{currentBuildExecution().label}</div>
                              </div>
                              <div class="advanced-summary-card">
                                <div class="advanced-label">Current step</div>
                                <div class="advanced-metric advanced-metric-text">
                                  {activeSession()?.current_step ?? "n/a"}
                                </div>
                              </div>
                            </div>
                            <Show
                              when={currentBuildExecution().items.length > 0}
                              fallback={<div class="empty-state">No build-facing pipeline events have been recorded for this project yet.</div>}
                            >
                              <div class="advanced-list">
                                <For each={currentBuildExecution().items}>
                                  {item => (
                                    <div class="advanced-list-row">
                                      <div>
                                        <div class="advanced-item-title">{item.title}</div>
                                        <div class="advanced-item-copy">{item.copy}</div>
                                      </div>
                                      <div class="advanced-item-meta">{item.meta}</div>
                                    </div>
                                  )}
                                </For>
                              </div>
                            </Show>
                          </div>
                        </Show>
                      </Match>

                      <Match when={advancedTab() === "outputs"}>
                        <Show when={!exportHistory.loading} fallback={<div class="advanced-loading">Loading outputs and artifacts…</div>}>
                          <div class="advanced-surface">
                            <div class="advanced-surface-head">
                              <div>
                                <div class="eyebrow">Outputs and artifacts</div>
                                <h3 class="advanced-surface-title">{outputArtifacts().headline}</h3>
                                <p class="section-copy">{outputArtifacts().copy}</p>
                              </div>
                              <span class={readinessBadgeClass(outputArtifacts().artifactCount > 0 ? "ready" : "not-started")}>
                                {outputArtifacts().artifactCount > 0
                                  ? `${outputArtifacts().artifactCount} artifact${
                                      outputArtifacts().artifactCount === 1 ? "" : "s"
                                    }`
                                  : "No outputs yet"}
                              </span>
                            </div>
                            <div class="advanced-summary-grid">
                              <div class="advanced-summary-card">
                                <div class="advanced-label">Recorded outputs</div>
                                <div class="advanced-metric">{outputArtifacts().artifactCount}</div>
                              </div>
                              <div class="advanced-summary-card">
                                <div class="advanced-label">Latest export</div>
                                <div class="advanced-metric advanced-metric-text">
                                  {outputArtifacts().items[0]?.meta ? formatTimestamp(outputArtifacts().items[0]!.meta) : "n/a"}
                                </div>
                              </div>
                              <div class="advanced-summary-card">
                                <div class="advanced-label">Build posture</div>
                                <div class="advanced-metric advanced-metric-text">{currentBuildPath().label}</div>
                              </div>
                              <div class="advanced-summary-card">
                                <div class="advanced-label">Source</div>
                                <div class="advanced-metric advanced-metric-text">Blueprint export history</div>
                              </div>
                            </div>
                            <Show
                              when={outputArtifacts().items.length > 0}
                              fallback={<div class="empty-state">No recorded outputs are attached to this project yet.</div>}
                            >
                              <div class="advanced-list">
                                <For each={outputArtifacts().items}>
                                  {item => (
                                    <div class="advanced-list-row">
                                      <div>
                                        <div class="advanced-item-title">{item.title}</div>
                                        <div class="advanced-item-copy">{item.copy}</div>
                                      </div>
                                      <div class="advanced-item-meta">{formatTimestamp(item.meta)}</div>
                                    </div>
                                  )}
                                </For>
                              </div>
                            </Show>
                          </div>
                        </Show>
                      </Match>

                      <Match when={advancedTab() === "blueprint"}>
                        <Show when={blueprintSummary()} fallback={<div class="advanced-loading">Loading project structure…</div>}>
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
                      </Match>
                    </Switch>
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
