import { A } from "@solidjs/router";
import { Show } from "solid-js";

import { AttachedTabs } from "~/components/ui/AttachedTabs";
import { MetricCard } from "~/components/ui/MetricCard";
import { StatusBadge } from "~/components/ui/StatusBadge";
import { SurfaceList, type SurfaceListItem } from "~/components/ui/SurfaceList";
import type {
  BlueprintSummary,
  BuildExecutionSummary,
  BuildPathSummary,
  BuildReadinessSummary,
  KnowledgeSummary,
  OutputArtifactSummary,
  ProjectActivitySummary,
  ReviewSummary,
} from "~/lib/advanced";
import {
  PROJECT_SURFACE_DISCLOSURE_LABEL,
  buildExecutionToneForState,
  buildPathToneForState,
  formatProjectSurfaceTimestamp,
  readinessToneForState,
  reviewToneForState,
  type ProjectSurfaceTab,
} from "~/lib/project-surface";
import { withFrontendMockSearch } from "~/lib/mock/runtime";
import type { ProjectImportResponse, PromptBankResponse } from "~/lib/types";

import styles from "./ProjectAdvancedPanel.module.css";

interface ProjectAdvancedPanelProps {
  projectSlug: string;
  activeSessionId?: string | null;
  activeSessionStep?: string | null;
  open: boolean;
  tab: ProjectSurfaceTab;
  reviewLoading: boolean;
  readinessLoading: boolean;
  buildLoading: boolean;
  activityLoading: boolean;
  executionLoading: boolean;
  outputsLoading: boolean;
  knowledgeSummary: KnowledgeSummary | null;
  blueprintSummary: BlueprintSummary | null;
  promptBank?: PromptBankResponse | null;
  reviewSummary: ReviewSummary;
  buildReadiness: BuildReadinessSummary;
  buildPath: BuildPathSummary;
  activitySummary: ProjectActivitySummary;
  buildExecution: BuildExecutionSummary;
  outputArtifacts: OutputArtifactSummary;
  importReview?: ProjectImportResponse | null;
  importState?: ProjectImportResponse | null;
  reviewError?: string | null;
  applyPending: boolean;
  reimportPending: boolean;
  selectionPendingNodeId?: string | null;
  onOpen: (tab?: ProjectSurfaceTab) => void;
  onClose: () => void;
  onTabChange: (tab: ProjectSurfaceTab) => void;
  onApplyImportReview: () => void;
  onReimport: () => void;
  onSetImportNodeIncluded: (nodeId: string, included: boolean) => void;
}

function copyAsListItems(items: Array<{ title: string; copy: string; meta: string }>): SurfaceListItem[] {
  return items.map(item => ({
    title: item.title,
    copy: item.copy,
    meta: item.meta,
  }));
}

export function ProjectAdvancedPanel(props: ProjectAdvancedPanelProps) {
  const reviewItems = () => {
    const reviewNodes = props.importReview?.review_nodes ?? [];
    if (props.importReview?.import_job.status === "review_pending" && reviewNodes.length > 0) {
      return reviewNodes.map(node => ({
        title: node.node_name,
        copy: `${node.node_type} · ${node.included ? "included in apply" : "excluded from apply"}`,
        action: (
          <button
            class="btn btn-subtle"
            disabled={props.selectionPendingNodeId === node.node_id}
            type="button"
            onClick={() => props.onSetImportNodeIncluded(node.node_id, !node.included)}
          >
            {props.selectionPendingNodeId === node.node_id
              ? node.included
                ? "Excluding…"
                : "Including…"
              : node.included
                ? "Exclude"
                : "Include"}
          </button>
        ),
      }));
    }

    return copyAsListItems(props.reviewSummary.rows);
  };

  const buildPathItems = () =>
    props.buildPath.blockers.map(blocker => ({ title: blocker, copy: "Blocker", meta: "Build handoff" }));

  const buildPathConfirmationItems = () =>
    props.buildPath.confirmations.map(confirmation => ({
      title: confirmation,
      copy: "Confirmation",
      meta: "Build handoff",
    }));

  const readinessBlockerItems = () =>
    props.buildReadiness.blockers.map(blocker => ({ title: blocker, copy: "Blocker", meta: "Build readiness" }));

  const readinessConfirmationItems = () =>
    props.buildReadiness.confirmations.map(confirmation => ({
      title: confirmation,
      copy: "Confirmation",
      meta: "Build readiness",
    }));

  const tabItems = [
    {
      value: "review" as const,
      label: "Review",
      content: (
        <Show when={!props.reviewLoading} fallback={<div class="empty-state">Loading project review state…</div>}>
          <div class={styles.surface}>
            <div class={styles.surfaceHead}>
              <div>
                <div class="eyebrow">Project review</div>
                <h3 class={styles.surfaceTitle}>{props.reviewSummary.headline}</h3>
                <p class="section-copy">{props.reviewSummary.copy}</p>
              </div>
              <StatusBadge tone={reviewToneForState(props.reviewSummary.state)}>
                {props.reviewSummary.state === "pending"
                  ? "Pending review"
                  : props.reviewSummary.state === "applied"
                    ? "Review applied"
                    : "Quiet"}
              </StatusBadge>
            </div>

            <div class={styles.metrics}>
              <MetricCard label="Pending" value={props.reviewSummary.pendingCount} />
              <MetricCard label="Completed" value={props.reviewSummary.completedCount} />
              <MetricCard
                label="Queued analysis"
                value={props.promptBank?.queued_threads.length ?? 0}
              />
              <MetricCard
                label="Build posture"
                text
                value={props.buildReadiness.label}
              />
            </div>

            {props.reviewError ? <div class="error-copy">{props.reviewError}</div> : null}

            <Show when={props.importReview || props.importState || props.reviewSummary.state !== "quiet"}>
              <div class="button-row">
                <A class="btn btn-subtle" href={withFrontendMockSearch(`/projects/${props.projectSlug}/import`)}>
                  Open import review
                </A>
                <A
                  class="btn btn-subtle"
                  href={withFrontendMockSearch(`/projects/${props.projectSlug}/import#import-history`)}
                >
                  Open import history
                </A>
                <button
                  class="btn btn-subtle"
                  disabled={props.reimportPending}
                  type="button"
                  onClick={props.onReimport}
                >
                  {props.reimportPending ? "Re-importing…" : "Start re-import"}
                </button>
                <Show when={props.importReview?.import_job.seed_session_id}>
                  {seedSessionId => (
                    <A class="btn btn-subtle" href={withFrontendMockSearch(`/sessions/${seedSessionId()}`)}>
                      Open seeded session
                    </A>
                  )}
                </Show>
                <Show when={props.importReview?.import_job.status === "review_pending"}>
                  <button
                    class="btn btn-primary"
                    disabled={props.applyPending}
                    type="button"
                    onClick={props.onApplyImportReview}
                  >
                    {props.applyPending ? "Applying…" : "Apply import review"}
                  </button>
                </Show>
              </div>
            </Show>

            <Show
              when={reviewItems().length > 0}
              fallback={<div class="empty-state">No project-local review queue is open right now.</div>}
            >
              <SurfaceList items={reviewItems()} />
            </Show>
          </div>
        </Show>
      ),
    },
    {
      value: "readiness" as const,
      label: "Build readiness",
      content: (
        <Show when={!props.readinessLoading} fallback={<div class="empty-state">Loading build-readiness state…</div>}>
          <div class={styles.surface}>
            <div class={styles.surfaceHead}>
              <div>
                <div class="eyebrow">Build readiness</div>
                <h3 class={styles.surfaceTitle}>{props.buildReadiness.headline}</h3>
                <p class="section-copy">{props.buildReadiness.nextAction}</p>
              </div>
              <StatusBadge tone={readinessToneForState(props.buildReadiness.state)}>
                {props.buildReadiness.label}
              </StatusBadge>
            </div>
            <div class={styles.metrics}>
              <MetricCard label="Banked threads" value={props.promptBank?.banked_threads.length ?? 0} />
              <MetricCard label="Queued threads" value={props.promptBank?.queued_threads.length ?? 0} />
              <MetricCard label="Blockers" value={props.buildReadiness.blockers.length} />
              <MetricCard label="Confirmations" value={props.buildReadiness.confirmations.length} />
            </div>
            <div class={styles.columns}>
              <div class={styles.column}>
                <div class={styles.columnLabel}>Blockers</div>
                <Show
                  when={props.buildReadiness.blockers.length > 0}
                  fallback={<div class={styles.columnCopy}>No explicit blockers are currently registered.</div>}
                >
                  <SurfaceList items={readinessBlockerItems()} />
                </Show>
              </div>
              <div class={styles.column}>
                <div class={styles.columnLabel}>Confirmations</div>
                <Show
                  when={props.buildReadiness.confirmations.length > 0}
                  fallback={<div class={styles.columnCopy}>Explicit confirmations will appear as analysis and review settle.</div>}
                >
                  <SurfaceList items={readinessConfirmationItems()} />
                </Show>
              </div>
            </div>
          </div>
        </Show>
      ),
    },
    {
      value: "build" as const,
      label: "Build path",
      content: (
        <Show when={!props.buildLoading} fallback={<div class="empty-state">Loading build handoff state…</div>}>
          <div class={styles.surface}>
            <div class={styles.surfaceHead}>
              <div>
                <div class="eyebrow">Build path</div>
                <h3 class={styles.surfaceTitle}>{props.buildPath.headline}</h3>
                <p class="section-copy">{props.buildPath.nextAction}</p>
              </div>
              <StatusBadge tone={buildPathToneForState(props.buildPath.state)}>
                {props.buildPath.label}
              </StatusBadge>
            </div>
            <div class={styles.columns}>
              <div class={styles.column}>
                <div class={styles.columnLabel}>Handoff target</div>
                <div class={styles.columnCopy}>
                  {props.buildPath.handoffTarget || "Project handoff target is still being assembled."}
                </div>
              </div>
              <div class={styles.column}>
                <div class={styles.columnLabel}>Next move</div>
                <div class={styles.columnCopy}>{props.buildPath.nextAction}</div>
                <div class="button-row">
                  <Show when={props.activeSessionId}>
                    {sessionId => (
                      <A class="btn btn-subtle" href={withFrontendMockSearch(`/sessions/${sessionId()}`)}>
                        Continue analysis
                      </A>
                    )}
                  </Show>
                  <button class="btn btn-subtle" type="button" onClick={() => props.onTabChange("readiness")}>
                    Open build readiness
                  </button>
                </div>
              </div>
            </div>
            <div class={styles.columns}>
              <div class={styles.column}>
                <div class={styles.columnLabel}>Blockers</div>
                <Show
                  when={props.buildPath.blockers.length > 0}
                  fallback={<div class={styles.columnCopy}>No explicit build handoff blockers are currently registered.</div>}
                >
                  <SurfaceList items={buildPathItems()} />
                </Show>
              </div>
              <div class={styles.column}>
                <div class={styles.columnLabel}>Confirmations</div>
                <Show
                  when={props.buildPath.confirmations.length > 0}
                  fallback={<div class={styles.columnCopy}>The handoff summary will list confirmations as the project settles.</div>}
                >
                  <SurfaceList items={buildPathConfirmationItems()} />
                </Show>
              </div>
            </div>
          </div>
        </Show>
      ),
    },
    {
      value: "execution" as const,
      label: "Build execution",
      content: (
        <Show when={!props.executionLoading} fallback={<div class="empty-state">Loading build execution…</div>}>
          <div class={styles.surface}>
            <div class={styles.surfaceHead}>
              <div>
                <div class="eyebrow">Build execution</div>
                <h3 class={styles.surfaceTitle}>{props.buildExecution.headline}</h3>
                <p class="section-copy">{props.buildExecution.nextAction}</p>
              </div>
              <StatusBadge tone={buildExecutionToneForState(props.buildExecution.state)}>
                {props.buildExecution.label}
              </StatusBadge>
            </div>
            <div class={styles.metrics}>
              <MetricCard label="Runs" value={props.buildExecution.runCount} />
              <MetricCard
                label="Latest run"
                text
                value={props.buildExecution.latestRunId ? props.buildExecution.latestRunId.slice(0, 8) : "n/a"}
              />
              <MetricCard label="Runtime state" text value={props.buildExecution.label} />
              <MetricCard label="Current step" text value={props.activeSessionStep ?? "n/a"} />
            </div>
            <Show
              when={props.buildExecution.items.length > 0}
              fallback={<div class="empty-state">No build-facing pipeline events have been recorded for this project yet.</div>}
            >
              <SurfaceList items={copyAsListItems(props.buildExecution.items)} />
            </Show>
          </div>
        </Show>
      ),
    },
    {
      value: "outputs" as const,
      label: "Outputs",
      content: (
        <Show when={!props.outputsLoading} fallback={<div class="empty-state">Loading outputs and artifacts…</div>}>
          <div class={styles.surface}>
            <div class={styles.surfaceHead}>
              <div>
                <div class="eyebrow">Outputs and artifacts</div>
                <h3 class={styles.surfaceTitle}>{props.outputArtifacts.headline}</h3>
                <p class="section-copy">{props.outputArtifacts.copy}</p>
              </div>
              <StatusBadge tone={props.outputArtifacts.artifactCount > 0 ? "active" : "quiet"}>
                {props.outputArtifacts.artifactCount > 0
                  ? `${props.outputArtifacts.artifactCount} artifact${props.outputArtifacts.artifactCount === 1 ? "" : "s"}`
                  : "No outputs yet"}
              </StatusBadge>
            </div>
            <div class={styles.metrics}>
              <MetricCard label="Recorded outputs" value={props.outputArtifacts.artifactCount} />
              <MetricCard
                label="Latest export"
                text
                value={
                  props.outputArtifacts.items[0]?.meta
                    ? formatProjectSurfaceTimestamp(props.outputArtifacts.items[0]!.meta)
                    : "n/a"
                }
              />
              <MetricCard label="Build posture" text value={props.buildPath.label} />
              <MetricCard label="Source" text value="Blueprint export history" />
            </div>
            <Show
              when={props.outputArtifacts.items.length > 0}
              fallback={<div class="empty-state">No recorded outputs are attached to this project yet.</div>}
            >
              <SurfaceList
                items={props.outputArtifacts.items.map(item => ({
                  title: item.title,
                  copy: item.copy,
                  meta: formatProjectSurfaceTimestamp(item.meta),
                }))}
              />
            </Show>
          </div>
        </Show>
      ),
    },
    {
      value: "activity" as const,
      label: "Activity",
      content: (
        <Show when={!props.activityLoading} fallback={<div class="empty-state">Loading project activity…</div>}>
          <div class={styles.surface}>
            <div class={styles.surfaceHead}>
              <div>
                <div class="eyebrow">Project activity</div>
                <h3 class={styles.surfaceTitle}>{props.activitySummary.headline}</h3>
                <p class="section-copy">{props.activitySummary.copy}</p>
              </div>
              <StatusBadge tone="recent">Attached timeline</StatusBadge>
            </div>
            <Show
              when={props.activitySummary.items.length > 0}
              fallback={<div class="empty-state">Project-local activity will appear here once the project starts moving.</div>}
            >
              <SurfaceList items={copyAsListItems(props.activitySummary.items)} />
            </Show>
          </div>
        </Show>
      ),
    },
    {
      value: "knowledge" as const,
      label: "Knowledge",
      content: (
        <Show when={props.knowledgeSummary} fallback={<div class="empty-state">Loading project knowledge…</div>}>
          {summary => (
            <div class={styles.surface}>
              <div class={styles.metrics}>
                <MetricCard label="Knowledge records" value={summary().totalNodes} />
                <MetricCard label="Documented" value={summary().documentedNodes} />
                <MetricCard label="Shared" value={summary().sharedNodes} />
                <MetricCard label="Stale" value={summary().staleNodes} />
              </div>
              <SurfaceList
                items={summary().featuredNodes.map(node => ({
                  title: node.name,
                  copy: `${node.node_type} · ${node.has_documentation ? "documented" : "needs docs"}`,
                  meta: `Updated ${formatProjectSurfaceTimestamp(node.updated_at)}`,
                }))}
              />
            </div>
          )}
        </Show>
      ),
    },
    {
      value: "blueprint" as const,
      label: "Blueprint",
      content: (
        <Show when={props.blueprintSummary} fallback={<div class="empty-state">Loading project structure…</div>}>
          {summary => (
            <div class={styles.surface}>
              <div class={styles.metrics}>
                <MetricCard label="Nodes" value={summary().totalNodes} />
                <MetricCard label="Edges" value={summary().totalEdges} />
                <MetricCard label="Decisions" value={summary().decisionNodes} />
                <MetricCard label="Components" value={summary().componentNodes} />
              </div>
              <SurfaceList
                items={summary().structuralNodes.map(node => ({
                  title: node.name,
                  copy: `${node.node_type} · ${node.scope_visibility}`,
                  meta: `Updated ${formatProjectSurfaceTimestamp(node.updated_at)}`,
                }))}
              />
            </div>
          )}
        </Show>
      ),
    },
  ];

  return (
    <section class={styles.root}>
      <div class={styles.header}>
        <div>
          <div class="eyebrow">Attached surfaces</div>
          <h2 class={styles.headerTitle}>{PROJECT_SURFACE_DISCLOSURE_LABEL}</h2>
          <p class="section-copy">
            Keep review, readiness, and project-local context inside the same workspace instead
            of leaving the route.
          </p>
        </div>
        <div class="button-row">
          <Show
            when={props.open}
            fallback={
              <button class="btn btn-subtle" type="button" onClick={() => props.onOpen("review")}>
                {PROJECT_SURFACE_DISCLOSURE_LABEL}
              </button>
            }
          >
            <button class="btn btn-subtle" type="button" onClick={props.onClose}>
              Hide attached surfaces
            </button>
          </Show>
        </div>
      </div>

      <Show when={props.open}>
        <div class={styles.panel}>
          <AttachedTabs
            items={tabItems}
            label="Project attached surfaces"
            value={props.tab}
            onChange={props.onTabChange}
          />
        </div>
      </Show>
    </section>
  );
}
