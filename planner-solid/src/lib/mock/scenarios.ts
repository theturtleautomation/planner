import type {
  AdminEventEntry,
  AdminStatusResponse,
  BlueprintEventPayload,
  BlueprintExportHistoryResponse,
  BlueprintResponse,
  HistoryListResponse,
  Project,
  ProjectImportDiffSummary,
  ProjectImportHistoryResponse,
  ProjectImportResponse,
  PromptBankResponse,
  ProposedEdge,
  ProposedNode,
  Session,
  WorkspaceStatus,
} from "~/lib/types";

import type { MockScenarioKey } from "./runtime";

export interface MockState {
  projects: Project[];
  sessions: Session[];
  promptBanks: Record<string, PromptBankResponse>;
  blueprintsByProjectRef: Record<string, BlueprintResponse>;
  importReviewByProjectRef: Record<string, ProjectImportResponse | null>;
  importStateByProjectRef: Record<string, ProjectImportResponse | null>;
  importHistoryByProjectRef: Record<string, ProjectImportHistoryResponse | null>;
  blueprintHistory: HistoryListResponse;
  blueprintEvents: BlueprintEventPayload[];
  blueprintExportHistoryByProjectRef: Record<string, BlueprintExportHistoryResponse>;
  proposedNodes: ProposedNode[];
  proposedEdges: ProposedEdge[];
  adminStatus: AdminStatusResponse;
  adminEvents: AdminEventEntry[];
  nextProjectIndex: number;
  nextSessionIndex: number;
  nextSnapshotIndex: number;
  nextProposalIndex: number;
}

const NOW = "2026-03-30T12:00:00Z";
const EARLIER = "2026-03-30T10:45:00Z";
const YESTERDAY = "2026-03-29T16:20:00Z";

function workspaceStatus(
  state: WorkspaceStatus["state"],
  label: string,
  detail: string,
  tone: WorkspaceStatus["tone"],
): WorkspaceStatus {
  return { state, label, detail, tone };
}

function project(overrides: Partial<Project> = {}): Project {
  const currentId = overrides.id ?? "project-1";
  return {
    id: currentId,
    slug: "personal-calendar",
    name: "Personal Calendar",
    description: "A local-first calendar app with task planning.",
    owner_user_id: "dev|mock",
    team_label: null,
    created_at: EARLIER,
    updated_at: NOW,
    archived_at: null,
    legacy_scope_keys: [],
    ...overrides,
  };
}

function session(overrides: Partial<Session> = {}): Session {
  const description = overrides.project_description ?? "A local-first calendar app with task planning.";
  return {
    id: "session-1",
    title: "Calendar intake",
    archived: false,
    created_at: EARLIER,
    last_activity_at: NOW,
    pipeline_running: false,
    intake_phase: "waiting",
    project_description: description,
    project_id: "project-1",
    project_slug: "personal-calendar",
    project_name: "Personal Calendar",
    current_step: null,
    error_message: null,
    can_resume_live: false,
    can_resume_checkpoint: false,
    can_restart_from_description: true,
    can_retry_pipeline: false,
    has_checkpoint: false,
    resume_status: "ready_to_start",
    workspace_status: workspaceStatus(
      "ready_to_start",
      "Ready to start analysis",
      description?.trim()
        ? "Waiting for the session workspace to begin from the saved brief."
        : "This session does not have a saved brief yet.",
      "neutral",
    ),
    ...overrides,
  };
}

function promptBank(
  sessionId: string,
  overrides: Partial<PromptBankResponse> = {},
): PromptBankResponse {
  const primaryPromptId = `prompt-${sessionId}`;
  const firstItemId = `item-${sessionId}-1`;

  return {
    session_id: sessionId,
    active_thread_id: overrides.active_thread_id ?? "workflow",
    banked_threads: [
      {
        category_id: "workflow",
        title: "Workflow",
        summary: "Confirm the main workflow shape.",
        question_count: 2,
        prompt: {
          prompt_id: primaryPromptId,
          title: "Workflow",
          kind: "question_batch",
          instructions: "Answer the two core workflow questions.",
          origin_category_id: "workflow",
          allow_partial_submit: true,
          items: [
            {
              item_id: firstItemId,
              kind: "discovery",
              target_dimension: "goal",
              text: "What is the main user flow this first version needs to support?",
              required: true,
              options: [
                {
                  option_id: `option-${sessionId}-calendar`,
                  label: "Calendar-first planning",
                  semantic_value: "calendar-first-planning",
                },
                {
                  option_id: `option-${sessionId}-task`,
                  label: "Task planning flow",
                  semantic_value: "task-planning-flow",
                },
              ],
            },
            {
              item_id: `item-${sessionId}-2`,
              kind: "verification",
              target_dimension: "out_of_scope",
              text: "What should the first shipped scope avoid?",
              required: true,
              options: [
                {
                  option_id: `option-${sessionId}-avoid-sync`,
                  label: "Avoid cross-device sync",
                  semantic_value: "avoid-cross-device-sync",
                },
                {
                  option_id: `option-${sessionId}-avoid-collab`,
                  label: "Avoid collaboration features",
                  semantic_value: "avoid-collaboration-features",
                },
              ],
            },
          ],
        },
      },
    ],
    queued_threads: [
      {
        category_id: "delivery",
        title: "Delivery",
        summary: "Follow-on build questions unlock after workflow answers settle.",
        question_count: 1,
        status: "queued",
      },
    ],
    build_ready: false,
    build_readiness_message: null,
    initial_bank_complete: true,
    saved_drafts: {},
    ...overrides,
  };
}

function emptyPromptBank(sessionId: string): PromptBankResponse {
  return {
    session_id: sessionId,
    active_thread_id: null,
    banked_threads: [],
    queued_threads: [],
    build_ready: false,
    build_readiness_message: null,
    initial_bank_complete: false,
    saved_drafts: {},
  };
}

function sourceBinding(projectRecord: Project) {
  return {
    project_id: projectRecord.id,
    provider: "github",
    canonical_ref: `thetu/${projectRecord.slug}`,
    default_branch: "main",
    head_revision: "abc1234",
    local_root: `/mock/${projectRecord.slug}`,
    managed_checkout: true,
    created_at: EARLIER,
    updated_at: NOW,
  };
}

function importJob(
  projectRecord: Project,
  overrides: Partial<ProjectImportResponse["import_job"]>,
): ProjectImportResponse["import_job"] {
  return {
    id: overrides.id ?? `import-${projectRecord.id}`,
    project_id: projectRecord.id,
    provider: "github",
    requested_ref: sourceBinding(projectRecord).canonical_ref,
    status: overrides.status ?? "review_pending",
    restored_from_job_id: overrides.restored_from_job_id ?? null,
    seed_session_id: overrides.seed_session_id ?? null,
    analysis_summary: overrides.analysis_summary ?? "Imported structure is ready for merge review.",
    progress_message: overrides.progress_message ?? null,
    error_message: overrides.error_message ?? null,
    created_at: overrides.created_at ?? EARLIER,
    updated_at: overrides.updated_at ?? NOW,
  };
}

function importReviewNode(nodeId: string, nodeName: string, nodeType: string, included = true) {
  return {
    node_id: nodeId,
    node_name: nodeName,
    node_type: nodeType,
    included,
  };
}

function importResponse(
  projectRecord: Project,
  status: ProjectImportResponse["import_job"]["status"],
  overrides: Partial<ProjectImportResponse> = {},
): ProjectImportResponse {
  const reviewNodes = overrides.review_nodes ?? [
    importReviewNode(`import-node-${projectRecord.id}-1`, "Calendar domain", "component", true),
    importReviewNode(`import-node-${projectRecord.id}-2`, "Reminder rules", "constraint", true),
    importReviewNode(`import-node-${projectRecord.id}-3`, "Offline sync", "technology", false),
  ];
  const includedNodeCount = reviewNodes.filter(node => node.included).length;
  const excludedNodeCount = reviewNodes.filter(node => !node.included).length;

  return {
    project: projectRecord,
    import_job: importJob(projectRecord, {
      ...overrides.import_job,
      status,
    }),
    source_binding: overrides.source_binding ?? sourceBinding(projectRecord),
    import_draft: overrides.import_draft ?? {
      job_id: overrides.import_job?.id ?? `import-${projectRecord.id}`,
      project_id: projectRecord.id,
      analysis_summary: "Planner identified a stable route family, reminder rules, and sync constraints.",
      source_metadata: {
        provider: "github",
        canonical_ref: sourceBinding(projectRecord).canonical_ref,
        local_root: `/mock/${projectRecord.slug}`,
        default_branch: "main",
        head_revision: "abc1234",
      },
      discovered_nodes: reviewNodes.map(node => ({
        id: node.node_id,
        name: node.node_name,
        node_type: node.node_type,
      })),
      created_at: EARLIER,
      updated_at: NOW,
    },
    import_review_selection: status === "review_pending"
      ? {
          job_id: overrides.import_job?.id ?? `import-${projectRecord.id}`,
          excluded_node_ids: reviewNodes.filter(node => !node.included).map(node => node.node_id),
          included_node_count: includedNodeCount,
          excluded_node_count: excludedNodeCount,
        }
      : null,
    review_nodes: status === "review_pending" ? reviewNodes : null,
    ...overrides,
  };
}

function importDiffSummary(
  currentJobId: string,
  comparedToJobId: string,
): ProjectImportDiffSummary {
  return {
    current_job_id: currentJobId,
    compared_to_job_id: comparedToJobId,
    added_nodes: [
      { node_id: "node-added-1", node_name: "Weekly planner", node_type: "component" },
      { node_id: "node-added-2", node_name: "Reminder cadence", node_type: "constraint" },
    ],
    removed_nodes: [{ node_id: "node-removed-1", node_name: "Legacy sync layer", node_type: "technology" }],
    added_node_types: [
      { node_type: "component", count: 1 },
      { node_type: "constraint", count: 1 },
    ],
    removed_node_types: [{ node_type: "technology", count: 1 }],
    current_head_revision: "abc1234",
    compared_head_revision: "def5678",
  };
}

function importHistory(projectRecord: Project): ProjectImportHistoryResponse {
  const appliedJob = importJob(projectRecord, {
    id: `import-${projectRecord.id}-applied`,
    status: "applied",
    analysis_summary: "Imported route structure applied to the project blueprint.",
    updated_at: NOW,
  });
  const previousJob = importJob(projectRecord, {
    id: `import-${projectRecord.id}-review`,
    status: "review_pending",
    analysis_summary: "Review draft saved before apply.",
    updated_at: YESTERDAY,
  });

  return {
    project: projectRecord,
    source_binding: sourceBinding(projectRecord),
    history: [
      {
        import_job: appliedJob,
        source_metadata: {
          provider: "github",
          canonical_ref: sourceBinding(projectRecord).canonical_ref,
          local_root: `/mock/${projectRecord.slug}`,
          default_branch: "main",
          head_revision: "abc1234",
        },
        discovered_node_count: 8,
        effective_included_node_count: 6,
        effective_excluded_node_count: 2,
      },
      {
        import_job: previousJob,
        source_metadata: {
          provider: "github",
          canonical_ref: sourceBinding(projectRecord).canonical_ref,
          local_root: `/mock/${projectRecord.slug}`,
          default_branch: "main",
          head_revision: "def5678",
        },
        discovered_node_count: 6,
        effective_included_node_count: 5,
        effective_excluded_node_count: 1,
      },
    ],
    diff_summary: importDiffSummary(appliedJob.id, previousJob.id),
  };
}

function node(
  projectRecord: Project,
  overrides: Partial<BlueprintResponse["nodes"][number]> = {},
): BlueprintResponse["nodes"][number] {
  return {
    id: overrides.id ?? `node-${projectRecord.id}-1`,
    name: overrides.name ?? projectRecord.name,
    node_type: overrides.node_type ?? "project",
    status: overrides.status ?? "active",
    scope_class: overrides.scope_class ?? "project",
    scope_visibility: overrides.scope_visibility ?? "project_local",
    is_shared: overrides.is_shared ?? false,
    lifecycle: overrides.lifecycle ?? "active",
    project_id: overrides.project_id ?? projectRecord.id,
    project_name: overrides.project_name ?? projectRecord.name,
    secondary_scope: overrides.secondary_scope ?? {},
    linked_project_ids: overrides.linked_project_ids ?? [projectRecord.id],
    override_source_id: overrides.override_source_id,
    override_reason: overrides.override_reason,
    override_effective_from: overrides.override_effective_from,
    scope_review_deferred_reason: overrides.scope_review_deferred_reason,
    scope_review_owner: overrides.scope_review_owner,
    scope_review_due_at: overrides.scope_review_due_at,
    tags: overrides.tags ?? [],
    has_documentation: overrides.has_documentation ?? true,
    updated_at: overrides.updated_at ?? NOW,
  };
}

function blueprint(projectRecord: Project, nodeOverrides: Array<Partial<BlueprintResponse["nodes"][number]>> = []): BlueprintResponse {
  const nodes = [
    node(projectRecord, {
      id: `${projectRecord.id}-project`,
      name: projectRecord.name,
      node_type: "project",
      tags: ["project"],
    }),
    node(projectRecord, {
      id: `${projectRecord.id}-decision`,
      name: "Project-first workflow",
      node_type: "decision",
      tags: ["workflow", "product"],
    }),
    node(projectRecord, {
      id: `${projectRecord.id}-component`,
      name: "Session workspace",
      node_type: "component",
      tags: ["ui", "route"],
    }),
    node(projectRecord, {
      id: `${projectRecord.id}-technology`,
      name: "Frontend mock mode",
      node_type: "technology",
      scope_visibility: "shared",
      is_shared: true,
      linked_project_ids: [projectRecord.id],
      tags: ["infra"],
    }),
    node(projectRecord, {
      id: `${projectRecord.id}-constraint`,
      name: "Offline-first browsing",
      node_type: "constraint",
      tags: ["ux"],
    }),
    ...nodeOverrides.map(override => node(projectRecord, override)),
  ];

  return {
    nodes,
    edges: [
      { source: `${projectRecord.id}-project`, target: `${projectRecord.id}-decision`, edge_type: "decided_by" },
      { source: `${projectRecord.id}-project`, target: `${projectRecord.id}-component`, edge_type: "contains" },
      { source: `${projectRecord.id}-component`, target: `${projectRecord.id}-technology`, edge_type: "uses" },
      { source: `${projectRecord.id}-component`, target: `${projectRecord.id}-constraint`, edge_type: "constrains" },
    ],
    counts: {
      project: 1,
      decision: 1,
      component: 1,
      technology: 1,
      constraint: 1,
    },
    total_nodes: nodes.length,
    total_edges: 4,
  };
}

function snapshot(filename: string, timestamp: string) {
  return { filename, timestamp };
}

function blueprintEvent(eventType: string, summary: string, timestamp: string, data: Record<string, unknown>): BlueprintEventPayload {
  return { event_type: eventType, summary, timestamp, data };
}

function exportHistory(projectRecord: Project): BlueprintExportHistoryResponse {
  return {
    entries: [
      {
        export_id: `export-${projectRecord.id}-1`,
        kind: "snapshot",
        actor: "frontend-mock",
        node_id: null,
        node_count: 5,
        edge_count: 4,
        project_id: projectRecord.id,
        project_name: projectRecord.name,
        scope_snapshot: null,
        scope_snapshot_redacted: false,
        scope_snapshot_redacted_fields: [],
        retention_expires_at: null,
        summary: "Mock export recorded after project review apply.",
        timestamp: NOW,
      },
    ],
    total: 1,
  };
}

function proposedNode(id: string, projectRecord: Project, status: ProposedNode["status"]): ProposedNode {
  return {
    id,
    node: {
      id: `${id}-node`,
      node_type: "component",
      name: "Reminder engine",
      scope: {
        project: {
          project_id: projectRecord.id,
          project_name: projectRecord.name,
        },
        secondary: {},
      },
    },
    source: "directory_scan",
    reason: "Route and feature names imply a dedicated reminder engine.",
    status,
    proposed_at: NOW,
    reviewed_at: status === "pending" ? undefined : NOW,
    confidence: 0.82,
    source_artifact: "src/routes/sessions",
  };
}

function proposedEdge(id: string, status: ProposedEdge["status"]): ProposedEdge {
  return {
    id,
    edge: {
      source: "node-project-workspace",
      target: "node-session-workspace",
      edge_type: "implements",
    },
    source: "code_graph_context",
    reason: "The project workspace links directly into session execution.",
    status,
    proposed_at: NOW,
    reviewed_at: status === "pending" ? undefined : NOW,
    confidence: 0.74,
    source_artifact: "src/components/projects",
  };
}

function adminStatus(degraded: boolean): AdminStatusResponse {
  return {
    status: degraded ? "degraded" : "ok",
    version: "mock-2026.03.30",
    uptime_secs: degraded ? 13_440 : 286_240,
    sessions: {
      active: degraded ? 3 : 1,
      total_events: degraded ? 148 : 18,
    },
    providers: [
      { name: "planner-server", binary: "planner-server", available: true },
      { name: "builder-bridge", binary: "builder-bridge", available: !degraded },
      { name: "ollama", binary: "ollama", available: true },
    ],
  };
}

function adminEvent(id: string, level: string, message: string, overrides: Partial<AdminEventEntry> = {}): AdminEventEntry {
  return {
    id,
    timestamp: overrides.timestamp ?? NOW,
    level,
    source: overrides.source ?? "system",
    session_id: overrides.session_id,
    project_id: overrides.project_id,
    project_name: overrides.project_name,
    step: overrides.step,
    message,
    duration_ms: overrides.duration_ms,
    metadata: overrides.metadata ?? {},
  };
}

function baseState(): MockState {
  const currentProject = project();
  const currentSession = session();
  const currentImportState = importResponse(currentProject, "applied", {
    review_nodes: null,
    import_review_selection: null,
    import_job: importJob(currentProject, {
      id: `import-${currentProject.id}-applied`,
      status: "applied",
      analysis_summary: "Imported route structure is already applied.",
      updated_at: NOW,
    }),
  });

  return {
    projects: [currentProject],
    sessions: [currentSession],
    promptBanks: {
      [currentSession.id]: emptyPromptBank(currentSession.id),
    },
    blueprintsByProjectRef: {
      [currentProject.id]: blueprint(currentProject),
      [currentProject.slug]: blueprint(currentProject),
    },
    importReviewByProjectRef: {
      [currentProject.id]: null,
      [currentProject.slug]: null,
    },
    importStateByProjectRef: {
      [currentProject.id]: currentImportState,
      [currentProject.slug]: currentImportState,
    },
    importHistoryByProjectRef: {
      [currentProject.id]: importHistory(currentProject),
      [currentProject.slug]: importHistory(currentProject),
    },
    blueprintHistory: {
      snapshots: [
        snapshot("blueprint-2026-03-30T12-00-00.json", NOW),
        snapshot("blueprint-2026-03-29T16-20-00.json", YESTERDAY),
      ],
    },
    blueprintEvents: [
      blueprintEvent("node_updated", "Updated project workflow node", NOW, {
        project: currentProject.slug,
        node_id: `${currentProject.id}-decision`,
      }),
      blueprintEvent("export_recorded", "Recorded a blueprint checkpoint export", YESTERDAY, {
        project: currentProject.slug,
      }),
    ],
    blueprintExportHistoryByProjectRef: {
      [currentProject.id]: exportHistory(currentProject),
      [currentProject.slug]: exportHistory(currentProject),
    },
    proposedNodes: [proposedNode("proposal-node-1", currentProject, "pending")],
    proposedEdges: [proposedEdge("proposal-edge-1", "pending")],
    adminStatus: adminStatus(false),
    adminEvents: [
      adminEvent("admin-1", "info", "Planner runtime healthy.", {
        source: "system",
        project_id: currentProject.id,
        project_name: currentProject.name,
      }),
    ],
    nextProjectIndex: 2,
    nextSessionIndex: 2,
    nextSnapshotIndex: 3,
    nextProposalIndex: 2,
  };
}

function emptyState(): MockState {
  return {
    projects: [],
    sessions: [],
    promptBanks: {},
    blueprintsByProjectRef: {},
    importReviewByProjectRef: {},
    importStateByProjectRef: {},
    importHistoryByProjectRef: {},
    blueprintHistory: { snapshots: [] },
    blueprintEvents: [],
    blueprintExportHistoryByProjectRef: {},
    proposedNodes: [],
    proposedEdges: [],
    adminStatus: adminStatus(false),
    adminEvents: [adminEvent("admin-quiet", "info", "No operator-visible events yet.", { timestamp: NOW })],
    nextProjectIndex: 1,
    nextSessionIndex: 1,
    nextSnapshotIndex: 1,
    nextProposalIndex: 1,
  };
}

export function createScenarioState(scenarioKey: MockScenarioKey): MockState {
  const state = baseState();
  const currentProject = state.projects[0]!;
  const currentSession = state.sessions[0]!;

  switch (scenarioKey) {
    case "empty":
      return emptyState();
    case "session-workspace":
      state.projects = [];
      state.sessions = [
        session({
          id: "session-11",
          title: "Session workspace mock",
          project_id: null,
          project_slug: null,
          project_name: null,
          intake_phase: "interviewing",
          has_checkpoint: true,
          can_resume_live: true,
          resume_status: "interview_attached",
          workspace_status: workspaceStatus(
            "awaiting_response",
            "Waiting for your response",
            "A mock prompt bank is ready for browsing.",
            "neutral",
          ),
        }),
      ];
      state.promptBanks = {
        "session-11": promptBank("session-11", {
          banked_threads: [
            {
              category_id: "workflow",
              title: "Workflow",
              summary: "Confirm the main workflow shape.",
              question_count: 2,
              prompt: {
                prompt_id: "prompt-session-11-workflow",
                title: "Workflow",
                kind: "question_batch",
                instructions: "Answer the workflow questions first.",
                origin_category_id: "workflow",
                allow_partial_submit: true,
                items: [
                  {
                    item_id: "item-session-11-1",
                    kind: "discovery",
                    target_dimension: "goal",
                    text: "What is the main user flow this first version needs to support?",
                    required: true,
                    options: [
                      {
                        option_id: "option-session-11-calendar",
                        label: "Calendar-first planning",
                        semantic_value: "calendar-first-planning",
                      },
                      {
                        option_id: "option-session-11-task",
                        label: "Task planning flow",
                        semantic_value: "task-planning-flow",
                      },
                    ],
                  },
                  {
                    item_id: "item-session-11-2",
                    kind: "verification",
                    target_dimension: "out_of_scope",
                    text: "What should the first shipped scope avoid?",
                    required: true,
                    options: [
                      {
                        option_id: "option-session-11-avoid-sync",
                        label: "Avoid cross-device sync",
                        semantic_value: "avoid-cross-device-sync",
                      },
                      {
                        option_id: "option-session-11-avoid-collab",
                        label: "Avoid collaboration features",
                        semantic_value: "avoid-collaboration-features",
                      },
                    ],
                  },
                ],
              },
            },
            {
              category_id: "actors",
              title: "Actors",
              summary: "Name the first people who need to trust and use this flow.",
              question_count: 1,
              prompt: {
                prompt_id: "prompt-session-11-actors",
                title: "Actors",
                kind: "question_batch",
                instructions: "Clarify who the first release is truly for.",
                origin_category_id: "actors",
                allow_partial_submit: true,
                items: [
                  {
                    item_id: "item-session-11-actors-1",
                    kind: "discovery",
                    target_dimension: "stakeholders",
                    text: "Who needs to trust this first version immediately?",
                    required: true,
                    options: [
                      {
                        option_id: "option-session-11-owner",
                        label: "The individual planning their week",
                        semantic_value: "individual-planner",
                      },
                      {
                        option_id: "option-session-11-manager",
                        label: "A manager reviewing team plans",
                        semantic_value: "manager-reviewer",
                      },
                    ],
                  },
                ],
              },
            },
            {
              category_id: "scope",
              title: "Scope",
              summary: "Define the release boundaries before delivery handoff.",
              question_count: 2,
              prompt: {
                prompt_id: "prompt-session-11-scope",
                title: "Scope",
                kind: "question_batch",
                instructions: "Define the first-release scope boundaries.",
                origin_category_id: "scope",
                allow_partial_submit: true,
                items: [
                  {
                    item_id: "item-session-11-3",
                    kind: "discovery",
                    target_dimension: "success_criteria",
                    text: "Which planning output needs to feel complete in v1?",
                    required: true,
                    options: [
                      {
                        option_id: "option-session-11-summary",
                        label: "A trustworthy weekly plan summary",
                        semantic_value: "weekly-plan-summary",
                      },
                      {
                        option_id: "option-session-11-review",
                        label: "A review flow for pending work",
                        semantic_value: "pending-work-review",
                      },
                    ],
                  },
                  {
                    item_id: "item-session-11-4",
                    kind: "verification",
                    target_dimension: "out_of_scope",
                    text: "What belongs after the first release?",
                    required: true,
                    options: [
                      {
                        option_id: "option-session-11-sync-later",
                        label: "Cross-device sync",
                        semantic_value: "cross-device-sync",
                      },
                      {
                        option_id: "option-session-11-sharing-later",
                        label: "Shared calendars",
                        semantic_value: "shared-calendars",
                      },
                    ],
                  },
                ],
              },
            },
          ],
          queued_threads: [
            {
              category_id: "confidence-refresh",
              title: "Confidence refresh",
              summary: "Raise confidence after recent answers.",
              question_count: 1,
              status: "queued",
              revision_area_id: "transformation",
              low_risk_update: true,
            },
            {
              category_id: "direction-promotion",
              title: "Direction promotion",
              summary: "Promote the weekly review lane into the canonical path for this area.",
              question_count: 1,
              status: "queued",
              revision_kind: "direction_promotion",
              revision_area_id: "transformation",
            },
            {
              category_id: "north-star-revision",
              title: "North-star revision",
              summary: "Promote manager review as the primary goal for the first release.",
              question_count: 1,
              status: "queued",
              revision_kind: "north_star",
              revision_area_id: "transformation",
              revision_conflict: true,
            },
          ],
          saved_drafts: {
            "item-session-11-1": {
              prompt_id: "prompt-session-11-workflow",
              item_id: "item-session-11-1",
              selected_option_id: "option-session-11-calendar",
              custom_text: "The first release should make weekly planning effortless.",
              skipped: false,
              updated_at: NOW,
            },
          },
        }),
      };
      return state;
    case "session-startup":
      state.projects = [];
      state.sessions = [
        session({
          id: "session-12",
          title: "Startup reveal mock",
          project_id: null,
          project_slug: null,
          project_name: null,
        }),
      ];
      state.promptBanks = {
        "session-12": emptyPromptBank("session-12"),
      };
      return state;
    case "session-complete":
      state.projects = [];
      state.sessions = [
        session({
          id: "session-13",
          title: "Completed session mock",
          project_id: null,
          project_slug: null,
          project_name: null,
          intake_phase: "complete",
          workspace_status: workspaceStatus(
            "complete",
            "Plan complete",
            "The mock session is complete and ready for review.",
            "success",
          ),
        }),
      ];
      state.promptBanks = {
        "session-13": promptBank("session-13", {
          queued_threads: [],
          build_ready: true,
          build_readiness_message: "The draft plan is ready for downstream review.",
        }),
      };
      return state;
    case "session-attention":
      state.projects = [];
      state.sessions = [
        session({
          id: "session-14",
          title: "Attention state mock",
          project_id: null,
          project_slug: null,
          project_name: null,
          intake_phase: "error",
          error_message: "The live connection dropped before prompt-bank startup completed.",
          workspace_status: workspaceStatus(
            "attention_required",
            "Needs attention",
            "The live Socratic connection dropped before startup could finish.",
            "error",
          ),
        }),
      ];
      state.promptBanks = {
        "session-14": emptyPromptBank("session-14"),
      };
      return state;
    case "project-active":
      state.importReviewByProjectRef[currentProject.id] = importResponse(currentProject, "review_pending", {
        import_job: importJob(currentProject, {
          id: `import-${currentProject.id}-review`,
          seed_session_id: currentSession.id,
          analysis_summary: "Imported routes need final inclusion decisions before apply.",
        }),
      });
      state.importReviewByProjectRef[currentProject.slug] = state.importReviewByProjectRef[currentProject.id];
      state.promptBanks[currentSession.id] = promptBank(currentSession.id);
      state.sessions[0] = session({
        intake_phase: "interviewing",
        has_checkpoint: true,
        can_resume_live: true,
        resume_status: "interview_attached",
        workspace_status: workspaceStatus(
          "awaiting_response",
          "Waiting for your response",
          "A mock prompt bank is ready for browsing.",
          "neutral",
        ),
      });
      return state;
    case "project-ready":
      state.sessions[0] = session({
        intake_phase: "complete",
        workspace_status: workspaceStatus(
          "build_ready",
          "Build ready",
          "The project has enough confirmed structure to move into the build path.",
          "success",
        ),
      });
      state.promptBanks[currentSession.id] = promptBank(currentSession.id, {
        queued_threads: [],
        build_ready: true,
        build_readiness_message: "Planner has enough confirmed answers to proceed into build handoff.",
      });
      return state;
    case "project-empty":
      state.sessions = [];
      state.promptBanks = {};
      state.blueprintsByProjectRef[currentProject.id] = {
        nodes: [node(currentProject, { id: `${currentProject.id}-project`, node_type: "project", name: currentProject.name })],
        edges: [],
        counts: { project: 1 },
        total_nodes: 1,
        total_edges: 0,
      };
      state.blueprintsByProjectRef[currentProject.slug] = state.blueprintsByProjectRef[currentProject.id]!;
      state.importStateByProjectRef[currentProject.id] = null;
      state.importStateByProjectRef[currentProject.slug] = null;
      state.importHistoryByProjectRef[currentProject.id] = null;
      state.importHistoryByProjectRef[currentProject.slug] = null;
      return state;
    case "import-review":
      state.importReviewByProjectRef[currentProject.id] = importResponse(currentProject, "review_pending", {
        import_job: importJob(currentProject, {
          id: `import-${currentProject.id}-review`,
          seed_session_id: currentSession.id,
          analysis_summary: "Three imported nodes need a final include or exclude decision.",
        }),
      });
      state.importReviewByProjectRef[currentProject.slug] = state.importReviewByProjectRef[currentProject.id];
      state.importStateByProjectRef[currentProject.id] = null;
      state.importStateByProjectRef[currentProject.slug] = null;
      return state;
    case "import-applied":
      state.importReviewByProjectRef[currentProject.id] = null;
      state.importReviewByProjectRef[currentProject.slug] = null;
      state.importStateByProjectRef[currentProject.id] = importResponse(currentProject, "applied", {
        review_nodes: null,
        import_review_selection: null,
        import_job: importJob(currentProject, {
          id: `import-${currentProject.id}-applied`,
          status: "applied",
          analysis_summary: "Import review was already applied to the project blueprint.",
        }),
      });
      state.importStateByProjectRef[currentProject.slug] = state.importStateByProjectRef[currentProject.id];
      return state;
    case "import-empty":
      state.importReviewByProjectRef[currentProject.id] = null;
      state.importReviewByProjectRef[currentProject.slug] = null;
      state.importStateByProjectRef[currentProject.id] = null;
      state.importStateByProjectRef[currentProject.slug] = null;
      state.importHistoryByProjectRef[currentProject.id] = null;
      state.importHistoryByProjectRef[currentProject.slug] = null;
      return state;
    case "multi-project-graph": {
      const secondProject = project({
        id: "project-2",
        slug: "studio-dashboard",
        name: "Studio Dashboard",
        description: "An operational dashboard for content and release orchestration.",
      });
      state.projects = [currentProject, secondProject];
      state.blueprintsByProjectRef[secondProject.id] = blueprint(secondProject, [
        {
          id: `${secondProject.id}-quality`,
          name: "Release confidence",
          node_type: "quality_requirement",
          tags: ["ops"],
        },
      ]);
      state.blueprintsByProjectRef[secondProject.slug] = state.blueprintsByProjectRef[secondProject.id]!;
      state.importStateByProjectRef[secondProject.id] = null;
      state.importStateByProjectRef[secondProject.slug] = null;
      state.importReviewByProjectRef[secondProject.id] = null;
      state.importReviewByProjectRef[secondProject.slug] = null;
      state.importHistoryByProjectRef[secondProject.id] = null;
      state.importHistoryByProjectRef[secondProject.slug] = null;
      state.blueprintExportHistoryByProjectRef[secondProject.id] = exportHistory(secondProject);
      state.blueprintExportHistoryByProjectRef[secondProject.slug] = state.blueprintExportHistoryByProjectRef[secondProject.id]!;
      state.nextProjectIndex = 3;
      return state;
    }
    case "ops-quiet":
      state.proposedNodes = [];
      state.proposedEdges = [];
      state.blueprintEvents = [
        blueprintEvent("node_updated", "Updated the shared project workflow node.", NOW, { severity: "info" }),
      ];
      state.adminStatus = adminStatus(false);
      state.adminEvents = [adminEvent("admin-quiet", "info", "Planner runtime healthy and quiet.", { timestamp: NOW })];
      return state;
    case "ops-history":
      state.proposedNodes = [proposedNode("proposal-node-history-1", currentProject, "pending")];
      state.proposedEdges = [proposedEdge("proposal-edge-history-1", "pending")];
      state.blueprintHistory = {
        snapshots: [
          snapshot("blueprint-2026-03-30T12-00-00.json", NOW),
          snapshot("blueprint-2026-03-30T10-45-00.json", EARLIER),
          snapshot("blueprint-2026-03-29T16-20-00.json", YESTERDAY),
        ],
      };
      state.blueprintEvents = [
        blueprintEvent("export_recorded", "Recorded project export for Personal Calendar.", NOW, {
          project: currentProject.slug,
          severity: "info",
          node_count: 6,
        }),
        blueprintEvent("node_updated", "Updated Task Service component.", EARLIER, {
          project: currentProject.slug,
          severity: "info",
          node_id: `${currentProject.id}-component`,
        }),
        blueprintEvent("export_recorded", "Archived the prior blueprint checkpoint after review apply.", YESTERDAY, {
          project: currentProject.slug,
          severity: "info",
          snapshot_count: 3,
        }),
      ];
      state.adminStatus = adminStatus(false);
      state.adminEvents = [
        adminEvent("admin-history-1", "info", "Snapshot history refreshed after blueprint review.", {
          source: "blueprint_history",
          project_id: currentProject.id,
          project_name: currentProject.name,
          timestamp: NOW,
        }),
      ];
      return state;
    case "ops-attention":
      state.proposedNodes = [
        proposedNode("proposal-node-1", currentProject, "pending"),
        proposedNode("proposal-node-2", currentProject, "pending"),
      ];
      state.proposedEdges = [
        proposedEdge("proposal-edge-1", "pending"),
        proposedEdge("proposal-edge-2", "accepted"),
      ];
      state.blueprintEvents = [
        blueprintEvent("node_updated", "Updated session workspace structure after import review.", NOW, { severity: "warn" }),
        blueprintEvent("edge_created", "Recorded a new project to session edge.", EARLIER, { severity: "info" }),
        blueprintEvent("export_recorded", "Exported a degraded-posture blueprint checkpoint.", YESTERDAY, { severity: "warn" }),
      ];
      state.adminStatus = adminStatus(true);
      state.adminEvents = [
        adminEvent("admin-ops-1", "warn", "Builder bridge is currently unavailable.", {
          source: "builder-bridge",
          project_id: currentProject.id,
          project_name: currentProject.name,
          timestamp: NOW,
        }),
        adminEvent("admin-ops-2", "error", "A session startup retry is waiting for operator review.", {
          source: "socratic_engine",
          session_id: currentSession.id,
          project_id: currentProject.id,
          project_name: currentProject.name,
          step: "startup",
          timestamp: EARLIER,
        }),
      ];
      return state;
    case "default":
    default:
      return state;
  }
}
