import { Title } from "@solidjs/meta";
import { A, useParams } from "@solidjs/router";
import { For, Show, createMemo, createResource, createSignal } from "solid-js";

import {
  applyProjectImportReview,
  getProject,
  getProjectImportHistoryComparison,
  getProjectImportHistoryOptional,
  reimportProject,
  getProjectImportHistoryPairComparison,
  getProjectImportReview,
  getProjectImportState,
  restoreProjectImportHistoryEntry,
  restoreProjectImportHistoryEntryForReview,
  restoreProjectImportReviewDraft,
  updateProjectImportReviewSelection,
} from "~/lib/api";
import {
  buildCurrentComparisonNotes,
  buildPairComparisonNotes,
  formatEntrySelection,
  formatImportStatusLabel,
  hasSelectionExclusions,
  summarizeDiffHeadline,
} from "~/lib/import-history";
import { withFrontendMockSearch } from "~/lib/mock/runtime";
import type {
  ProjectImportDiffSummary,
  ProjectImportHistoryComparisonResponse,
  ProjectImportHistoryEntry,
  ProjectImportHistoryPairComparisonResponse,
  ProjectImportResponse,
} from "~/lib/types";

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

function renderNodeTypeFacts(diff: ProjectImportDiffSummary, side: "added" | "removed"): string | null {
  const entries = side === "added" ? diff.added_node_types : diff.removed_node_types;
  if (entries.length === 0) return null;
  return entries.map(entry => `${entry.node_type} (${entry.count})`).join(", ");
}

function renderNodeFacts(diff: ProjectImportDiffSummary, side: "added" | "removed"): string | null {
  const entries = side === "added" ? diff.added_nodes : diff.removed_nodes;
  if (entries.length === 0) return null;
  return entries.map(entry => entry.node_name).join(", ");
}

function currentStateNotice(state: ProjectImportResponse | null): string {
  if (!state) return "No current import posture is attached to this project right now.";
  if (state.import_job.status === "review_pending") {
    return "A pending import review is open now. Historical restore actions stay blocked until you resolve it.";
  }
  return state.import_job.analysis_summary ?? "The current import posture is stable.";
}

export default function ProjectImportReviewPage() {
  const params = useParams<{ projectSlug: string }>();
  const projectSlug = () => params.projectSlug;
  const [selectionPendingNodeId, setSelectionPendingNodeId] = createSignal<string | null>(null);
  const [applyPending, setApplyPending] = createSignal(false);
  const [routeError, setRouteError] = createSignal<string | null>(null);
  const [historyError, setHistoryError] = createSignal<string | null>(null);
  const [historyNotice, setHistoryNotice] = createSignal<string | null>(null);
  const [pendingHistoryJobId, setPendingHistoryJobId] = createSignal<string | null>(null);
  const [reimportPending, setReimportPending] = createSignal(false);
  const [baselineJobId, setBaselineJobId] = createSignal<string | null>(null);
  const [currentComparison, setCurrentComparison] =
    createSignal<ProjectImportHistoryComparisonResponse | null>(null);
  const [pairComparison, setPairComparison] =
    createSignal<ProjectImportHistoryPairComparisonResponse | null>(null);

  const [project] = createResource(projectSlug, getProject);
  const [importReview, { refetch: refetchImportReview }] = createResource(projectSlug, getProjectImportReview);
  const [importState, { refetch: refetchImportState }] = createResource(projectSlug, getProjectImportState);
  const [importHistory, { refetch: refetchImportHistory }] = createResource(
    projectSlug,
    getProjectImportHistoryOptional,
  );

  const current = createMemo(() => importReview() ?? importState() ?? null);
  const reviewNodes = createMemo(() => importReview()?.review_nodes ?? []);
  const includedCount = createMemo(
    () =>
      importReview()?.import_review_selection?.included_node_count ??
      reviewNodes().filter(node => node.included).length,
  );
  const excludedCount = createMemo(
    () =>
      importReview()?.import_review_selection?.excluded_node_count ??
      reviewNodes().filter(node => !node.included).length,
  );
  const pendingReview = createMemo(() => current()?.import_job.status === "review_pending");
  const historyEntries = createMemo(() => importHistory()?.history ?? []);
  const currentImportJobId = createMemo(() => current()?.import_job.id ?? null);
  const restoreBlockedByPendingReview = createMemo(() => pendingReview());
  const currentComparisonNotes = createMemo(() =>
    currentComparison() ? buildCurrentComparisonNotes(currentComparison()!) : [],
  );
  const pairComparisonNotes = createMemo(() =>
    pairComparison() ? buildPairComparisonNotes(pairComparison()!) : [],
  );

  async function refreshImportRoute() {
    await Promise.all([refetchImportReview(), refetchImportState(), refetchImportHistory()]);
  }

  function clearHistorySelections() {
    setBaselineJobId(null);
    setCurrentComparison(null);
    setPairComparison(null);
  }

  async function handleSetIncluded(nodeId: string, included: boolean) {
    setSelectionPendingNodeId(nodeId);
    setRouteError(null);
    try {
      await updateProjectImportReviewSelection(projectSlug(), {
        nodeId,
        included,
      });
      clearHistorySelections();
      await refreshImportRoute();
    } catch (error) {
      setRouteError(error instanceof Error ? error.message : "Unable to update the import review selection.");
    } finally {
      setSelectionPendingNodeId(null);
    }
  }

  async function handleApply() {
    setApplyPending(true);
    setRouteError(null);
    try {
      await applyProjectImportReview(projectSlug());
      clearHistorySelections();
      await refreshImportRoute();
    } catch (error) {
      setRouteError(error instanceof Error ? error.message : "Unable to apply the import review.");
    } finally {
      setApplyPending(false);
    }
  }

  async function handleCompareToCurrent(jobId: string) {
    setHistoryError(null);
    setHistoryNotice(null);
    setPairComparison(null);
    try {
      const response = await getProjectImportHistoryComparison(projectSlug(), jobId);
      setCurrentComparison(response);
    } catch (error) {
      setHistoryError(error instanceof Error ? error.message : "Unable to compare the selected history entry.");
    }
  }

  function handleSelectBaseline(jobId: string) {
    setHistoryError(null);
    setHistoryNotice(null);
    setCurrentComparison(null);
    if (baselineJobId() === jobId) {
      setBaselineJobId(null);
      setPairComparison(null);
      return;
    }
    setBaselineJobId(jobId);
    setPairComparison(null);
  }

  async function handleCompareToBaseline(jobId: string) {
    if (!baselineJobId()) return;
    setHistoryError(null);
    setHistoryNotice(null);
    setCurrentComparison(null);
    try {
      const response = await getProjectImportHistoryPairComparison(projectSlug(), baselineJobId()!, jobId);
      setPairComparison(response);
    } catch (error) {
      setHistoryError(error instanceof Error ? error.message : "Unable to compare the selected history entries.");
    }
  }

  async function handleHistoryRestore(
    jobId: string,
    action: (projectRef: string, selectedJobId: string) => Promise<ProjectImportResponse>,
    fallbackNotice: string,
  ) {
    setPendingHistoryJobId(jobId);
    setHistoryError(null);
    setHistoryNotice(null);
    try {
      const response = await action(projectSlug(), jobId);
      clearHistorySelections();
      setHistoryNotice(response.import_job.progress_message ?? response.import_job.analysis_summary ?? fallbackNotice);
      await refreshImportRoute();
    } catch (error) {
      setHistoryError(error instanceof Error ? error.message : fallbackNotice);
    } finally {
      setPendingHistoryJobId(null);
    }
  }

  async function handleReimport() {
    setReimportPending(true);
    setRouteError(null);
    setHistoryError(null);
    setHistoryNotice(null);
    try {
      const response = await reimportProject(projectSlug());
      clearHistorySelections();
      setHistoryNotice(
        response.import_job.progress_message ??
          response.import_job.analysis_summary ??
          "Re-import request queued.",
      );
      await refreshImportRoute();
    } catch (error) {
      setRouteError(error instanceof Error ? error.message : "Unable to start a re-import.");
    } finally {
      setReimportPending(false);
    }
  }

  return (
    <section class="page page-scroll">
      <Title>Project Import Review</Title>
      <div class="stack page-frame">
        <section class="hero-panel workspace-hero">
          <div class="eyebrow">Project import review</div>
          <h1 class="hero-title">Import review</h1>
          <p class="hero-copy">
            Keep import decisions attached to the project. Pending review work stays primary, and attached history below handles comparison, restore, and draft recovery without leaving project context.
          </p>
          <div class="hero-focus project-focus">
            <div>
              <div class="hero-focus-label">Current project</div>
              <h2 class="hero-focus-title">{project()?.project.name ?? "Loading project…"}</h2>
              <p class="hero-focus-copy">
                {pendingReview()
                  ? `${reviewNodes().length} imported node${reviewNodes().length === 1 ? "" : "s"} still need final inclusion decisions.`
                  : current()
                    ? current()!.import_job.analysis_summary ?? "Import state is attached and stable."
                    : "No active import draft is attached to this project right now."}
              </p>
            </div>
            <div class="hero-actions">
              <A class="btn btn-subtle" href={withFrontendMockSearch(`/projects/${projectSlug()}`)}>
                Back to project
              </A>
              <A
                class="btn btn-subtle"
                href={withFrontendMockSearch(`/projects/${projectSlug()}/import#import-history`)}
              >
                Jump to import history
              </A>
              <Show when={current()}>
                <button class="btn btn-subtle" type="button" disabled={reimportPending()} onClick={() => void handleReimport()}>
                  {reimportPending() ? "Re-importing…" : "Start re-import"}
                </button>
              </Show>
              <Show when={pendingReview() && current()?.import_job.seed_session_id}>
                {seedSessionId => (
                  <A class="btn btn-subtle" href={withFrontendMockSearch(`/sessions/${seedSessionId()}`)}>
                    Open seeded session
                  </A>
                )}
              </Show>
              <Show when={pendingReview()}>
                <button class="btn btn-primary" type="button" disabled={applyPending()} onClick={() => void handleApply()}>
                  {applyPending() ? "Applying…" : "Apply import review"}
                </button>
              </Show>
            </div>
          </div>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Import posture</div>
              <h2 class="section-title">Current import state</h2>
            </div>
          </div>
          <Show
            when={current()}
            fallback={<div class="empty-state">No import review or applied import state is attached to this project yet.</div>}
          >
            {state => (
              <div class="advanced-grid">
                <div class="advanced-summary-card">
                  <div class="advanced-label">Status</div>
                  <div class="advanced-metric advanced-metric-text">{formatImportStatusLabel(state().import_job.status)}</div>
                </div>
                <div class="advanced-summary-card">
                  <div class="advanced-label">Included</div>
                  <div class="advanced-metric">{includedCount()}</div>
                </div>
                <div class="advanced-summary-card">
                  <div class="advanced-label">Excluded</div>
                  <div class="advanced-metric">{excludedCount()}</div>
                </div>
                <div class="advanced-summary-card">
                  <div class="advanced-label">Source</div>
                  <div class="advanced-metric advanced-metric-text">{state().source_binding.canonical_ref}</div>
                </div>
              </div>
            )}
          </Show>
          <Show when={routeError()}>{message => <div class="error-copy">{message()}</div>}</Show>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Decision desk</div>
              <h2 class="section-title">Imported nodes</h2>
            </div>
          </div>
          <Show
            when={pendingReview() && reviewNodes().length > 0}
            fallback={
              <div class="empty-state">
                {current()
                  ? "No pending import node decisions remain. The route now shifts into history, restore, and comparison."
                  : "No imported nodes are waiting for review."}
              </div>
            }
          >
            <div class="advanced-list">
              <For each={reviewNodes()}>
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
                      onClick={() => void handleSetIncluded(node.node_id, !node.included)}
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
        </section>

        <section class="section-panel" id="import-history">
          <div class="section-head">
            <div>
              <div class="eyebrow">Import history</div>
              <h2 class="section-title">Historical restore and comparison</h2>
              <p class="section-copy">{currentStateNotice(current())}</p>
            </div>
          </div>

          <div class="advanced-summary-grid">
            <div class="advanced-summary-card">
              <div class="advanced-label">Current status</div>
              <div class="advanced-metric advanced-metric-text">
                {current() ? formatImportStatusLabel(current()!.import_job.status) : "No current state"}
              </div>
            </div>
            <div class="advanced-summary-card">
              <div class="advanced-label">History entries</div>
              <div class="advanced-metric">{historyEntries().length}</div>
            </div>
            <div class="advanced-summary-card">
              <div class="advanced-label">Restore blocker</div>
              <div class="advanced-metric advanced-metric-text">
                {restoreBlockedByPendingReview() ? "Pending review" : "Clear"}
              </div>
            </div>
            <div class="advanced-summary-card">
              <div class="advanced-label">Source binding</div>
              <div class="advanced-metric advanced-metric-text">
                {importHistory()?.source_binding.canonical_ref ?? current()?.source_binding.canonical_ref ?? "No source binding"}
              </div>
            </div>
          </div>

          <Show when={historyNotice()}>{message => <div class="success-copy">{message()}</div>}</Show>
          <Show when={historyError()}>{message => <div class="error-copy">{message()}</div>}</Show>

          <Show when={importHistory()?.diff_summary}>
            {diff => (
              <div class="advanced-grid">
                <div class="advanced-column-panel">
                  <div class="advanced-label">Changes since last applied import</div>
                  <div class="advanced-value">{summarizeDiffHeadline(diff())}</div>
                  <Show when={diff().current_head_revision}>
                    {value => <div class="advanced-item-copy">Current revision: {value().slice(0, 8)}</div>}
                  </Show>
                  <Show when={diff().compared_head_revision}>
                    {value => <div class="advanced-item-copy">Previous revision: {value().slice(0, 8)}</div>}
                  </Show>
                </div>
                <div class="advanced-column-panel">
                  <div class="advanced-label">Type deltas</div>
                  <Show when={renderNodeTypeFacts(diff(), "added")}>
                    {text => <div class="advanced-item-copy">Added types: {text()}</div>}
                  </Show>
                  <Show when={renderNodeTypeFacts(diff(), "removed")}>
                    {text => <div class="advanced-item-copy">Removed types: {text()}</div>}
                  </Show>
                </div>
              </div>
            )}
          </Show>

          <Show when={currentComparison()}>
            {comparison => (
              <div class="advanced-grid">
                <div class="advanced-column-panel">
                  <div class="advanced-label">Selected history entry vs current import</div>
                  <div class="advanced-value">
                    Comparing import {comparison().selected_entry.import_job.id.slice(0, 8)} to current import {comparison().current_import_job.id.slice(0, 8)}.
                  </div>
                  <Show
                    when={currentComparisonNotes().length > 0}
                    fallback={<div class="advanced-item-copy">No saved selection filters affected this comparison.</div>}
                  >
                    <ul class="advanced-bullet-list">
                      <For each={currentComparisonNotes()}>{note => <li>{note}</li>}</For>
                    </ul>
                  </Show>
                </div>
                <div class="advanced-column-panel">
                  <div class="advanced-label">Diff summary</div>
                  <div class="advanced-value">{summarizeDiffHeadline(comparison().diff_summary)}</div>
                  <Show when={renderNodeFacts(comparison().diff_summary, "added")}>
                    {text => <div class="advanced-item-copy">Added nodes: {text()}</div>}
                  </Show>
                  <Show when={renderNodeFacts(comparison().diff_summary, "removed")}>
                    {text => <div class="advanced-item-copy">Removed nodes: {text()}</div>}
                  </Show>
                </div>
              </div>
            )}
          </Show>

          <Show when={pairComparison()}>
            {comparison => (
              <div class="advanced-grid">
                <div class="advanced-column-panel">
                  <div class="advanced-label">Selected history entries compared</div>
                  <div class="advanced-value">
                    Comparing baseline import {comparison().baseline_entry.import_job.id.slice(0, 8)} to import {comparison().compared_entry.import_job.id.slice(0, 8)}.
                  </div>
                  <Show
                    when={pairComparisonNotes().length > 0}
                    fallback={<div class="advanced-item-copy">No saved selection filters affected this pair comparison.</div>}
                  >
                    <ul class="advanced-bullet-list">
                      <For each={pairComparisonNotes()}>{note => <li>{note}</li>}</For>
                    </ul>
                  </Show>
                </div>
                <div class="advanced-column-panel">
                  <div class="advanced-label">Diff summary</div>
                  <div class="advanced-value">{summarizeDiffHeadline(comparison().diff_summary)}</div>
                  <Show when={renderNodeFacts(comparison().diff_summary, "added")}>
                    {text => <div class="advanced-item-copy">Added nodes: {text()}</div>}
                  </Show>
                  <Show when={renderNodeFacts(comparison().diff_summary, "removed")}>
                    {text => <div class="advanced-item-copy">Removed nodes: {text()}</div>}
                  </Show>
                </div>
              </div>
            )}
          </Show>

          <Show
            when={!importHistory.loading}
            fallback={<div class="advanced-loading">Loading project import history…</div>}
          >
            <Show
              when={historyEntries().length > 0}
              fallback={<div class="empty-state">No project-local import history is attached to this project yet.</div>}
            >
              <div class="history-entry-list">
                <For each={historyEntries()}>
                  {entry => (
                    <article class="history-entry-card">
                      <div class="history-entry-head">
                        <div>
                          <div class="history-entry-title">
                            {entry.import_job.provider.toUpperCase()} · {formatImportStatusLabel(entry.import_job.status)}
                          </div>
                          <div class="history-entry-copy">
                            {entry.import_job.analysis_summary ?? "Historical import entry"}
                          </div>
                        </div>
                        <div class="history-entry-meta">Updated {formatTimestamp(entry.import_job.updated_at)}</div>
                      </div>

                      <div class="history-entry-facts">
                        <span>Source: {entry.import_job.requested_ref}</span>
                        <Show when={entry.import_job.restored_from_job_id}>
                          {jobId => <span>Restored from {jobId().slice(0, 8)}</span>}
                        </Show>
                        <Show when={entry.source_metadata?.head_revision}>
                          {revision => <span>Revision: {revision().slice(0, 8)}</span>}
                        </Show>
                        <Show when={entry.discovered_node_count != null}>
                          <span>Draft nodes: {entry.discovered_node_count}</span>
                        </Show>
                        <Show when={formatEntrySelection(entry)}>
                          {selection => <span>{selection()}</span>}
                        </Show>
                      </div>

                      <Show when={hasSelectionExclusions(entry)}>
                        <div class="history-entry-note">
                          Saved exclusions affect this job&apos;s effective apply footprint.
                        </div>
                      </Show>

                      <div class="history-entry-actions">
                        <Show when={entry.discovered_node_count != null}>
                          <button
                            class="btn btn-subtle"
                            type="button"
                            onClick={() => handleSelectBaseline(entry.import_job.id)}
                          >
                            {baselineJobId() === entry.import_job.id ? "Baseline Selected" : "Use as baseline"}
                          </button>
                        </Show>

                        <Show when={baselineJobId() && baselineJobId() !== entry.import_job.id}>
                          <button
                            class="btn btn-subtle"
                            type="button"
                            onClick={() => void handleCompareToBaseline(entry.import_job.id)}
                          >
                            Compare to selected
                          </button>
                        </Show>

                        <Show when={entry.discovered_node_count != null && entry.import_job.id !== currentImportJobId()}>
                          <button
                            class="btn btn-subtle"
                            type="button"
                            onClick={() => void handleCompareToCurrent(entry.import_job.id)}
                          >
                            Compare to current
                          </button>
                        </Show>

                        <Show
                          when={
                            entry.import_job.status === "review_pending" &&
                            !restoreBlockedByPendingReview() &&
                            entry.import_job.id !== currentImportJobId()
                          }
                        >
                          <button
                            class="btn btn-subtle"
                            type="button"
                            disabled={pendingHistoryJobId() === entry.import_job.id}
                            onClick={() =>
                              void handleHistoryRestore(
                                entry.import_job.id,
                                restoreProjectImportReviewDraft,
                                "Historical review draft restored.",
                              )}
                          >
                            {pendingHistoryJobId() === entry.import_job.id ? "Restoring draft…" : "Restore draft for review"}
                          </button>
                        </Show>

                        <Show
                          when={
                            entry.import_job.status === "applied" &&
                            !restoreBlockedByPendingReview() &&
                            entry.import_job.id !== currentImportJobId()
                          }
                        >
                          <>
                            <button
                              class="btn btn-subtle"
                              type="button"
                              disabled={pendingHistoryJobId() === entry.import_job.id}
                              onClick={() =>
                                void handleHistoryRestore(
                                  entry.import_job.id,
                                  restoreProjectImportHistoryEntryForReview,
                                  "Historical import restored into review.",
                                )}
                            >
                              {pendingHistoryJobId() === entry.import_job.id ? "Restoring for review…" : "Restore for review"}
                            </button>
                            <button
                              class="btn btn-subtle"
                              type="button"
                              disabled={pendingHistoryJobId() === entry.import_job.id}
                              onClick={() =>
                                void handleHistoryRestore(
                                  entry.import_job.id,
                                  restoreProjectImportHistoryEntry,
                                  "Historical import restored to the project blueprint.",
                                )}
                            >
                              {pendingHistoryJobId() === entry.import_job.id ? "Restoring…" : "Restore this import"}
                            </button>
                          </>
                        </Show>
                      </div>
                    </article>
                  )}
                </For>
              </div>
            </Show>
          </Show>
        </section>
      </div>
    </section>
  );
}
