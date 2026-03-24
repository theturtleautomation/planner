import type {
  BlueprintResponse,
  NodeSummary,
  PlannerEvent,
  ProjectImportResponse,
  PromptBankResponse,
  RunListResponse,
  SessionSummary,
} from "./types";

export type AdvancedPanelTab =
  | "knowledge"
  | "blueprint"
  | "review"
  | "readiness"
  | "build"
  | "activity"
  | "execution";

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

export interface ReviewSummaryRow {
  title: string;
  copy: string;
  meta: string;
}

export interface ReviewSummary {
  state: "pending" | "quiet" | "applied";
  headline: string;
  copy: string;
  pendingCount: number;
  completedCount: number;
  rows: ReviewSummaryRow[];
}

export interface BuildReadinessSummary {
  state: "ready" | "needs-review" | "in-progress" | "not-started";
  label: string;
  headline: string;
  nextAction: string;
  blockers: string[];
  confirmations: string[];
}

export interface BuildPathSummary {
  state: "ready" | "blocked" | "staging" | "not-started";
  label: string;
  headline: string;
  nextAction: string;
  handoffTarget: string;
  blockers: string[];
  confirmations: string[];
}

export interface ProjectActivityItem {
  title: string;
  copy: string;
  meta: string;
}

export interface ProjectActivitySummary {
  headline: string;
  copy: string;
  items: ProjectActivityItem[];
}

export interface BuildExecutionSummary {
  state: "active" | "failed" | "idle" | "complete";
  label: string;
  headline: string;
  nextAction: string;
  runCount: number;
  latestRunId?: string | null;
  items: ProjectActivityItem[];
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

export function summarizeReview(params: {
  importReview?: ProjectImportResponse | null;
  importState?: ProjectImportResponse | null;
  promptBank?: PromptBankResponse | null;
}): ReviewSummary {
  const importReview = params.importReview ?? null;
  const importState = params.importState ?? null;
  const promptBank = params.promptBank ?? null;
  const reviewNodes = importReview?.review_nodes ?? [];
  const includedCount =
    importReview?.import_review_selection?.included_node_count ??
    reviewNodes.filter(node => node.included).length;
  const excludedCount = importReview?.import_review_selection?.excluded_node_count ?? 0;
  const queuedThreads = promptBank?.queued_threads ?? [];

  if (importReview?.import_job.status === "review_pending") {
    return {
      state: "pending",
      headline: "Import draft needs review",
      copy:
        importReview.import_job.analysis_summary ??
        "A project import draft is waiting for merge decisions before it should affect the build path.",
      pendingCount: includedCount,
      completedCount: excludedCount,
      rows: reviewNodes.slice(0, 5).map(node => ({
        title: node.node_name,
        copy: `${node.node_type} · ${node.included ? "included in apply" : "excluded from apply"}`,
        meta: `Draft node ${node.node_id}`,
      })),
    };
  }

  if (queuedThreads.length > 0) {
    return {
      state: "pending",
      headline: "Analysis follow-ups are still queued",
      copy: "The project still has unresolved Socratic threads before it can read as fully reviewed.",
      pendingCount: queuedThreads.length,
      completedCount: promptBank?.banked_threads.length ?? 0,
      rows: queuedThreads.slice(0, 5).map(thread => ({
        title: thread.title,
        copy: thread.summary,
        meta: `${thread.status} · ${thread.question_count} question${thread.question_count === 1 ? "" : "s"}`,
      })),
    };
  }

  if (importState?.import_job.status === "applied") {
    return {
      state: "applied",
      headline: "Latest import review is already applied",
      copy:
        importState.import_job.analysis_summary ??
        "The most recent import reconciliation is stable and no project-level review queue is open.",
      pendingCount: 0,
      completedCount: 1,
      rows: [
        {
          title: importState.source_binding.canonical_ref,
          copy: `${importState.source_binding.provider.toUpperCase()} source remains attached to the project`,
          meta: `Applied ${importState.import_job.updated_at}`,
        },
      ],
    };
  }

  return {
    state: "quiet",
    headline: "No active review queue",
    copy: "Import reconciliation is quiet and no additional review surface is blocking the project right now.",
    pendingCount: 0,
    completedCount: 0,
    rows: [],
  };
}

export function summarizeBuildReadiness(params: {
  primarySession?: SessionSummary | null;
  promptBank?: PromptBankResponse | null;
  importState?: ProjectImportResponse | null;
  importReview?: ProjectImportResponse | null;
  blueprintSummary?: BlueprintSummary | null;
}): BuildReadinessSummary {
  const primarySession = params.primarySession ?? null;
  const promptBank = params.promptBank ?? null;
  const importState = params.importState ?? null;
  const importReview = params.importReview ?? null;
  const blueprintSummary = params.blueprintSummary ?? null;
  const blockers: string[] = [];
  const confirmations: string[] = [];

  if (importReview?.import_job.status === "review_pending") {
    blockers.push("Import draft still needs merge review before the project should be treated as build-ready.");
  }
  if (promptBank && promptBank.queued_threads.length > 0) {
    blockers.push(
      `${promptBank.queued_threads.length} Socratic thread${
        promptBank.queued_threads.length === 1 ? " is" : "s are"
      } still queued for question generation or follow-up.`,
    );
  }
  if (primarySession && primarySession.intake_phase !== "complete" && !promptBank?.build_ready) {
    blockers.push("Active analysis is still in progress and has not yet reported build readiness.");
  }
  if (blueprintSummary && blueprintSummary.totalNodes > 0) {
    confirmations.push(`${blueprintSummary.totalNodes} blueprint nodes are already attached to the project.`);
  }
  if (promptBank && promptBank.banked_threads.length > 0) {
    confirmations.push(
      `${promptBank.banked_threads.length} banked thread${
        promptBank.banked_threads.length === 1 ? "" : "s"
      } are locally available in the active analysis.`,
    );
  }
  if (importState?.import_job.status === "applied") {
    confirmations.push("Latest import review has already been reconciled into the project state.");
  }

  if (!primarySession && !importState && !promptBank) {
    return {
      state: "not-started",
      label: "Not started",
      headline: "Build readiness has not started yet",
      nextAction: "Start the first Socratic analysis for this project.",
      blockers: ["No active analysis session is attached to this project yet."],
      confirmations,
    };
  }

  if (blockers.length === 0 && promptBank?.build_ready) {
    if (promptBank.build_readiness_message) {
      confirmations.unshift(promptBank.build_readiness_message);
    }
    return {
      state: "ready",
      label: "Ready to build",
      headline: "The project is currently ready for the next build step",
      nextAction: "Review the project scope once, then move into the build path.",
      blockers: [],
      confirmations,
    };
  }

  if (importReview?.import_job.status === "review_pending") {
    return {
      state: "needs-review",
      label: "Needs review",
      headline: "A review gate is still blocking build readiness",
      nextAction: "Resolve the project review queue before treating this project as build-ready.",
      blockers,
      confirmations,
    };
  }

  return {
    state: "in-progress",
    label: "In progress",
    headline: "Build readiness is still being shaped",
    nextAction: "Continue analysis until the remaining queued threads are resolved.",
    blockers,
    confirmations,
  };
}

export function summarizeBuildPath(params: {
  primarySession?: SessionSummary | null;
  readiness: BuildReadinessSummary;
  promptBank?: PromptBankResponse | null;
  blueprintSummary?: BlueprintSummary | null;
  projectName: string;
}): BuildPathSummary {
  const primarySession = params.primarySession ?? null;
  const promptBank = params.promptBank ?? null;
  const blueprintSummary = params.blueprintSummary ?? null;
  const readiness = params.readiness;

  const handoffTarget = [
    params.projectName,
    blueprintSummary ? `${blueprintSummary.totalNodes} blueprint nodes` : null,
    promptBank ? `${promptBank.banked_threads.length} banked thread${promptBank.banked_threads.length === 1 ? "" : "s"}` : null,
  ]
    .filter(Boolean)
    .join(" · ");

  if (readiness.state === "ready") {
    return {
      state: "ready",
      label: "Handoff ready",
      headline: "Automation handoff is ready",
      nextAction: "Review the handoff snapshot once, then move this project into the build path.",
      handoffTarget,
      blockers: [],
      confirmations: readiness.confirmations,
    };
  }

  if (readiness.state === "needs-review") {
    return {
      state: "blocked",
      label: "Blocked by review",
      headline: "Build handoff is blocked by unresolved review work",
      nextAction: "Resolve the project review queue before handing this project to the automated build path.",
      handoffTarget,
      blockers: readiness.blockers,
      confirmations: readiness.confirmations,
    };
  }

  if (readiness.state === "not-started") {
    return {
      state: "not-started",
      label: "Not started",
      headline: "Build handoff cannot start without project analysis",
      nextAction: "Start the first project analysis to establish the build-facing truth.",
      handoffTarget,
      blockers: readiness.blockers,
      confirmations: readiness.confirmations,
    };
  }

  return {
    state: "staging",
    label: "Still staging",
    headline: primarySession
      ? "Build handoff is still being assembled from active analysis"
      : "Build handoff still needs a stable project state",
    nextAction: readiness.nextAction,
    handoffTarget,
    blockers: readiness.blockers,
    confirmations: readiness.confirmations,
  };
}

export function summarizeProjectActivity(params: {
  sessions: SessionSummary[];
  importState?: ProjectImportResponse | null;
  promptBank?: PromptBankResponse | null;
  buildPath: BuildPathSummary;
}): ProjectActivitySummary {
  const items: ProjectActivityItem[] = [];
  const sessions = [...params.sessions].sort(
    (left, right) => Date.parse(right.last_activity_at) - Date.parse(left.last_activity_at),
  );

  if (sessions[0]) {
    items.push({
      title: sessions[0].title ?? "Active analysis session",
      copy: `${sessions[0].intake_phase} · ${sessions[0].project_description ?? "Project analysis session"}`,
      meta: sessions[0].last_activity_at,
    });
  }

  if (params.importState?.import_job.status) {
    items.push({
      title: "Import state",
      copy:
        params.importState.import_job.analysis_summary ??
        params.importState.import_job.progress_message ??
        `Project import is ${params.importState.import_job.status}.`,
      meta: params.importState.import_job.updated_at,
    });
  }

  if ((params.promptBank?.queued_threads.length ?? 0) > 0) {
    items.push({
      title: "Queued Socratic follow-ups",
      copy: `${params.promptBank!.queued_threads.length} thread${
        params.promptBank!.queued_threads.length === 1 ? "" : "s"
      } still need question generation or follow-up.`,
      meta: "Current session prompt bank",
    });
  }

  items.push({
    title: "Build path",
    copy: params.buildPath.headline,
    meta: params.buildPath.label,
  });

  return {
    headline: "Recent project activity",
    copy: "Use this attached surface to scan what changed recently without leaving the project workspace.",
    items: items.slice(0, 6),
  };
}

export function summarizeBuildExecution(params: {
  primarySession?: SessionSummary | null;
  runs?: RunListResponse | null;
  events?: PlannerEvent[];
}): BuildExecutionSummary {
  const primarySession = params.primarySession ?? null;
  const runs = params.runs?.runs ?? [];
  const latestRunId = runs[0] ?? null;
  const items = (params.events ?? [])
    .filter(event => event.source === "pipeline")
    .sort((left, right) => Date.parse(right.timestamp) - Date.parse(left.timestamp))
    .slice(0, 5)
    .map(event => ({
      title: event.step ?? event.message,
      copy: event.message,
      meta: event.timestamp,
    }));

  if (!primarySession) {
    return {
      state: "idle",
      label: "No build run",
      headline: "No project-local build run is available yet",
      nextAction: "Start analysis and reach a stable build handoff before expecting build execution detail.",
      runCount: 0,
      latestRunId,
      items,
    };
  }

  if (primarySession.error_message) {
    return {
      state: "failed",
      label: "Run failed",
      headline: "The latest build-facing run ended in an error state",
      nextAction: "Inspect the latest pipeline error, then retry or return to analysis.",
      runCount: runs.length,
      latestRunId,
      items,
    };
  }

  if (primarySession.pipeline_running) {
    return {
      state: "active",
      label: "Run active",
      headline: "A build-facing pipeline run is currently active",
      nextAction: "Watch the latest pipeline events here while the run advances.",
      runCount: runs.length,
      latestRunId,
      items,
    };
  }

  if (runs.length > 0 || primarySession.intake_phase === "complete") {
    return {
      state: "complete",
      label: "Latest run complete",
      headline: "The latest build-facing run is no longer active",
      nextAction: "Use the latest run and event trail to decide whether to continue, retry, or return to shaping work.",
      runCount: runs.length,
      latestRunId,
      items,
    };
  }

  return {
    state: "idle",
    label: "No build run",
    headline: "No build execution detail is available yet",
    nextAction: "Move the project into build handoff before expecting project-local execution detail.",
    runCount: runs.length,
    latestRunId,
    items,
  };
}
