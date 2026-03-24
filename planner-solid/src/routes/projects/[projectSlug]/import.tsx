import { Title } from "@solidjs/meta";
import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, For, Show } from "solid-js";

import {
  applyProjectImportReview,
  getProject,
  getProjectImportReview,
  getProjectImportState,
  updateProjectImportReviewSelection,
} from "~/lib/api";

export default function ProjectImportReviewPage() {
  const params = useParams<{ projectSlug: string }>();
  const projectSlug = () => params.projectSlug;
  const [selectionPendingNodeId, setSelectionPendingNodeId] = createSignal<string | null>(null);
  const [applyPending, setApplyPending] = createSignal(false);
  const [routeError, setRouteError] = createSignal<string | null>(null);

  const [project] = createResource(projectSlug, getProject);
  const [importReview, { refetch: refetchImportReview }] = createResource(projectSlug, getProjectImportReview);
  const [importState, { refetch: refetchImportState }] = createResource(projectSlug, getProjectImportState);

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
  const pendingReview = createMemo(() => importReview()?.import_job.status === "review_pending");

  async function handleSetIncluded(nodeId: string, included: boolean) {
    setSelectionPendingNodeId(nodeId);
    setRouteError(null);
    try {
      await updateProjectImportReviewSelection(projectSlug(), {
        nodeId,
        included,
      });
      await refetchImportReview();
      await refetchImportState();
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
      await refetchImportReview();
      await refetchImportState();
    } catch (error) {
      setRouteError(error instanceof Error ? error.message : "Unable to apply the import review.");
    } finally {
      setApplyPending(false);
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
            Keep import decisions attached to the project. Pending review work stays primary, and applied history remains available without overwhelming the decision path.
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
              <A class="btn btn-subtle" href={`/projects/${projectSlug()}`}>
                Back to project
              </A>
              <Show when={pendingReview() && current()?.import_job.seed_session_id}>
                {seedSessionId => (
                  <A class="btn btn-subtle" href={`/sessions/${seedSessionId()}`}>
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
                  <div class="advanced-metric advanced-metric-text">{state().import_job.status}</div>
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
                  ? "No pending import node decisions remain. The route is now serving as attached history and posture."
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
      </div>
    </section>
  );
}
