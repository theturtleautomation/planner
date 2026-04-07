import type {
  AdminEventsResponse,
  AdminStatusResponse,
  BlueprintEventsResponse,
  BlueprintExportHistoryResponse,
  BlueprintResponse,
  CreateProjectRequest,
  CreateSessionResponse,
  DeleteProjectResponse,
  DiscoveryRunResponse,
  GetSessionResponse,
  HistoryListResponse,
  ListProjectsResponse,
  ListSessionsResponse,
  Project,
  ProjectImportHistoryComparisonResponse,
  ProjectImportHistoryPairComparisonResponse,
  ProjectImportHistoryResponse,
  ProjectImportResponse,
  ProjectResponse,
  PromptAnswer,
  PromptBankResponse,
  ProposedEdgesResponse,
  ProposedNodesResponse,
  SavedPromptAnswerDraft,
  SavePromptDraftsResponse,
  Session,
  SessionEventsResponse,
  SessionExportResponse,
} from "~/lib/types";

import { createScenarioState, type MockState } from "./scenarios";
import { getFrontendMockScenarioKey } from "./runtime";

let activeScenarioKey: string | null = null;
let activeState: MockState | null = null;

function cloneState<T>(value: T): T {
  return typeof structuredClone === "function"
    ? structuredClone(value)
    : JSON.parse(JSON.stringify(value)) as T;
}

function ensureState(): MockState {
  const scenarioKey = getFrontendMockScenarioKey();
  if (!activeState || activeScenarioKey !== scenarioKey) {
    activeScenarioKey = scenarioKey;
    activeState = cloneState(createScenarioState(scenarioKey));
  }
  return activeState;
}

function nowIso(): string {
  return new Date().toISOString();
}

function slugify(input: string): string {
  return input
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48) || "project";
}

function projectRefs(projectRecord: Project): string[] {
  return [projectRecord.id, projectRecord.slug];
}

function findProject(state: MockState, projectRef: string): Project | null {
  return state.projects.find(project => project.slug === projectRef || project.id === projectRef) ?? null;
}

function findSession(state: MockState, sessionId: string): Session | null {
  return state.sessions.find(session => session.id === sessionId) ?? null;
}

function readyWorkspaceStatus(description: string | null) {
  return {
    state: "ready_to_start" as const,
    label: "Ready to start analysis",
    detail: description?.trim()
      ? "Waiting for the session workspace to begin from the saved brief."
      : "This session does not have a saved brief yet.",
    tone: "neutral" as const,
  };
}

function awaitingWorkspaceStatus() {
  return {
    state: "awaiting_response" as const,
    label: "Waiting for your response",
    detail: "A mock prompt bank is ready for browsing.",
    tone: "neutral" as const,
  };
}

function completeWorkspaceStatus() {
  return {
    state: "complete" as const,
    label: "Plan complete",
    detail: "The mock session is complete and ready for review.",
    tone: "success" as const,
  };
}

function pipelineWorkspaceStatus() {
  return {
    state: "pipeline_running" as const,
    label: "Running the build pipeline",
    detail: "Mock pipeline execution is currently in progress.",
    tone: "active" as const,
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

function buildSessionPromptBank(sessionId: string): PromptBankResponse {
  return {
    session_id: sessionId,
    active_thread_id: "workflow",
    banked_threads: [
      {
        category_id: "workflow",
        title: "Workflow",
        summary: "Confirm the main workflow shape.",
        question_count: 2,
        prompt: {
          prompt_id: `prompt-${sessionId}`,
          title: "Workflow",
          kind: "question_batch",
          instructions: "Answer the two core workflow questions.",
          origin_category_id: "workflow",
          allow_partial_submit: true,
          items: [
            {
              item_id: `item-${sessionId}-1`,
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
        summary: "Follow-on build questions unlock after the workflow answers settle.",
        question_count: 1,
        status: "queued",
      },
    ],
    build_ready: false,
    build_readiness_message: null,
    initial_bank_complete: true,
    saved_drafts: {},
  };
}

function storeProjectScoped<T>(
  target: Record<string, T>,
  projectRecord: Project,
  value: T,
): void {
  for (const ref of projectRefs(projectRecord)) {
    target[ref] = value;
  }
}

function removeProjectScoped<T>(target: Record<string, T>, projectRecord: Project): void {
  for (const ref of projectRefs(projectRecord)) {
    delete target[ref];
  }
}

function projectImportResponseForHistory(
  projectRecord: Project,
  entry: ProjectImportHistoryResponse["history"][number],
  status: ProjectImportResponse["import_job"]["status"],
): ProjectImportResponse {
  const sourceBinding = {
    project_id: projectRecord.id,
    provider: entry.import_job.provider,
    canonical_ref: entry.source_metadata?.canonical_ref ?? `thetu/${projectRecord.slug}`,
    default_branch: entry.source_metadata?.default_branch ?? "main",
    head_revision: entry.source_metadata?.head_revision ?? null,
    local_root: entry.source_metadata?.local_root ?? null,
    managed_checkout: true,
    created_at: entry.import_job.created_at,
    updated_at: entry.import_job.updated_at,
  };

  return {
    project: projectRecord,
    import_job: {
      ...entry.import_job,
      status,
      progress_message: status === "review_pending" ? "History entry restored into review mode." : null,
    },
    source_binding: sourceBinding,
    import_draft: {
      job_id: entry.import_job.id,
      project_id: projectRecord.id,
      analysis_summary: entry.import_job.analysis_summary ?? "Restored mock import review draft.",
      source_metadata: {
        provider: sourceBinding.provider,
        canonical_ref: sourceBinding.canonical_ref,
        local_root: sourceBinding.local_root ?? `/mock/${projectRecord.slug}`,
        default_branch: sourceBinding.default_branch ?? "main",
        head_revision: sourceBinding.head_revision ?? null,
      },
      discovered_nodes: [],
      created_at: entry.import_job.created_at,
      updated_at: nowIso(),
    },
    import_review_selection: status === "review_pending"
      ? {
          job_id: entry.import_job.id,
          excluded_node_ids: [],
          included_node_count: entry.effective_included_node_count ?? entry.discovered_node_count ?? 0,
          excluded_node_count: entry.effective_excluded_node_count ?? 0,
        }
      : null,
    review_nodes: status === "review_pending"
      ? [
          {
            node_id: `${entry.import_job.id}-node-1`,
            node_name: "Restored route graph",
            node_type: "component",
            included: true,
          },
        ]
      : null,
  };
}

function comparisonDiff(currentJobId: string, comparedToJobId: string) {
  return {
    current_job_id: currentJobId,
    compared_to_job_id: comparedToJobId,
    added_nodes: [
      { node_id: "diff-added-1", node_name: "Reminder engine", node_type: "component" },
    ],
    removed_nodes: [
      { node_id: "diff-removed-1", node_name: "Legacy sync layer", node_type: "technology" },
    ],
    added_node_types: [{ node_type: "component", count: 1 }],
    removed_node_types: [{ node_type: "technology", count: 1 }],
    current_head_revision: "abc1234",
    compared_head_revision: "def5678",
  };
}

function latestImportJob(state: MockState, projectRecord: Project): ProjectImportResponse | null {
  return state.importReviewByProjectRef[projectRecord.slug]
    ?? state.importStateByProjectRef[projectRecord.slug]
    ?? null;
}

export function resetMockStateForTesting(): void {
  activeScenarioKey = null;
  activeState = null;
}

export function listMockProjects(): ListProjectsResponse {
  const state = ensureState();
  return { projects: cloneState(state.projects) };
}

export function getMockProject(projectRef: string): ProjectResponse {
  const state = ensureState();
  const projectRecord = findProject(state, projectRef);
  if (!projectRecord) {
    throw new Error(`Mock project not found: ${projectRef}`);
  }
  return { project: cloneState(projectRecord) };
}

export function createMockProject(request: CreateProjectRequest): ProjectResponse {
  const state = ensureState();
  const id = `project-${state.nextProjectIndex++}`;
  const timestamp = nowIso();
  const slugBase = request.slug?.trim() || slugify(request.name);
  let slug = slugBase;
  let suffix = 2;
  while (state.projects.some(project => project.slug === slug)) {
    slug = `${slugBase}-${suffix++}`;
  }

  const projectRecord: Project = {
    id,
    slug,
    name: request.name.trim(),
    description: request.description?.trim() || null,
    owner_user_id: "dev|mock",
    team_label: request.team_label?.trim() || null,
    created_at: timestamp,
    updated_at: timestamp,
    archived_at: null,
    legacy_scope_keys: request.legacy_scope_keys ?? [],
  };

  state.projects.unshift(projectRecord);
  const blueprint: BlueprintResponse = {
    nodes: [
      {
        id: `${projectRecord.id}-project`,
        name: projectRecord.name,
        node_type: "project",
        status: "active",
        scope_class: "project",
        scope_visibility: "project_local",
        is_shared: false,
        lifecycle: "active",
        project_id: projectRecord.id,
        project_name: projectRecord.name,
        secondary_scope: {},
        linked_project_ids: [projectRecord.id],
        tags: ["project"],
        has_documentation: false,
        updated_at: timestamp,
      },
    ],
    edges: [],
    counts: { project: 1 },
    total_nodes: 1,
    total_edges: 0,
  };
  storeProjectScoped(state.blueprintsByProjectRef, projectRecord, blueprint);
  storeProjectScoped(state.importReviewByProjectRef, projectRecord, null);
  storeProjectScoped(state.importStateByProjectRef, projectRecord, null);
  storeProjectScoped(state.importHistoryByProjectRef, projectRecord, null);
  storeProjectScoped(state.blueprintExportHistoryByProjectRef, projectRecord, { entries: [], total: 0 });

  return { project: cloneState(projectRecord) };
}

export function deleteMockProject(projectRef: string): DeleteProjectResponse {
  const state = ensureState();
  const projectRecord = findProject(state, projectRef);
  if (!projectRecord) {
    throw new Error(`Mock project not found: ${projectRef}`);
  }

  const deletedProjectSessions = state.sessions.filter(session => session.project_id === projectRecord.id);
  state.projects = state.projects.filter(candidate => candidate.id !== projectRecord.id);
  state.sessions = state.sessions.filter(session => session.project_id !== projectRecord.id);
  for (const session of deletedProjectSessions) {
    delete state.promptBanks[session.id];
  }
  removeProjectScoped(state.blueprintsByProjectRef, projectRecord);
  removeProjectScoped(state.importReviewByProjectRef, projectRecord);
  removeProjectScoped(state.importStateByProjectRef, projectRecord);
  removeProjectScoped(state.importHistoryByProjectRef, projectRecord);
  removeProjectScoped(state.blueprintExportHistoryByProjectRef, projectRecord);

  return {
    project_id: projectRecord.id,
    project_name: projectRecord.name,
    stopped_live_sessions: 0,
    stopped_pipeline_sessions: 0,
    deleted_sessions: deletedProjectSessions.length,
    deleted_session_event_files: 0,
    deleted_cxdb_runs: 0,
    deleted_blueprint_nodes: 0,
    unlinked_shared_blueprint_nodes: 0,
    deleted_project_record: true,
    blueprint_events_pruned: 0,
    blueprint_history_snapshots_pruned: 0,
    deleted_import_jobs: 0,
    deleted_import_drafts: 0,
    deleted_import_managed_roots: 0,
  };
}

export function listMockSessions(): ListSessionsResponse {
  const state = ensureState();
  return { sessions: cloneState(state.sessions) };
}

export function listMockProjectSessions(projectRef: string): ListSessionsResponse {
  const state = ensureState();
  const projectRecord = findProject(state, projectRef);
  const sessions = projectRecord
    ? state.sessions.filter(session => session.project_id === projectRecord.id)
    : [];
  return { sessions: cloneState(sessions) };
}

export function createMockSession(payload?: {
  projectRef?: string | null;
  description?: string | null;
  title?: string | null;
}): CreateSessionResponse {
  const state = ensureState();
  const timestamp = nowIso();
  const projectRecord = payload?.projectRef ? findProject(state, payload.projectRef) : null;
  const id = `session-${state.nextSessionIndex++}`;
  const description = payload?.description?.trim() || projectRecord?.description || null;

  const created: Session = {
    id,
    title: payload?.title?.trim() || (description ? description.slice(0, 96) : "Direct session"),
    archived: false,
    created_at: timestamp,
    last_activity_at: timestamp,
    pipeline_running: false,
    intake_phase: "waiting",
    project_description: description,
    project_id: projectRecord?.id ?? null,
    project_slug: projectRecord?.slug ?? null,
    project_name: projectRecord?.name ?? null,
    current_step: null,
    error_message: null,
    can_resume_live: false,
    can_resume_checkpoint: false,
    can_restart_from_description: Boolean(description),
    can_retry_pipeline: false,
    has_checkpoint: false,
    resume_status: "ready_to_start",
    workspace_status: readyWorkspaceStatus(description),
  };
  state.sessions.unshift(created);
  state.promptBanks[id] = emptyPromptBank(id);
  return { session: cloneState(created) };
}

export function createMockProjectSession(
  projectRef: string,
  payload?: { title?: string | null; description?: string | null },
): CreateSessionResponse {
  return createMockSession({
    projectRef,
    title: payload?.title ?? null,
    description: payload?.description ?? null,
  });
}

export function getMockSession(sessionId: string): GetSessionResponse {
  const state = ensureState();
  const session = findSession(state, sessionId);
  if (!session) {
    throw new Error(`Mock session not found: ${sessionId}`);
  }
  return { session: cloneState(session) };
}

export function duplicateMockSession(
  sessionId: string,
  payload?: { title?: string | null },
): GetSessionResponse {
  const state = ensureState();
  const existing = findSession(state, sessionId);
  if (!existing) {
    throw new Error(`Mock session not found: ${sessionId}`);
  }
  const created = createMockSession({
    projectRef: existing.project_slug ?? existing.project_id ?? null,
    description: existing.project_description ?? null,
    title: payload?.title ?? `${existing.title ?? "Session"} copy`,
  });
  state.promptBanks[created.session.id] = cloneState(state.promptBanks[existing.id] ?? emptyPromptBank(existing.id));
  state.promptBanks[created.session.id].session_id = created.session.id;
  state.promptBanks[created.session.id].banked_threads = state.promptBanks[created.session.id].banked_threads.map(thread => ({
    ...thread,
    prompt: {
      ...thread.prompt,
      prompt_id: thread.prompt.prompt_id.replace(existing.id, created.session.id),
      items: thread.prompt.items.map(item => ({
        ...item,
        item_id: item.item_id.replace(existing.id, created.session.id),
        options: item.options.map(option => ({
          ...option,
          option_id: option.option_id.replace(existing.id, created.session.id),
        })),
      })),
    },
  }));
  return { session: cloneState(created.session) };
}

export function exportMockSession(sessionId: string): SessionExportResponse {
  const session = getMockSession(sessionId).session;
  return {
    exported_at: nowIso(),
    session: {
      ...session,
      messages: [],
    },
  };
}

export function restartMockSessionFromDescription(sessionId: string): GetSessionResponse {
  const state = ensureState();
  const activeSession = findSession(state, sessionId);
  if (!activeSession) {
    throw new Error(`Mock session not found: ${sessionId}`);
  }

  Object.assign(activeSession, {
    intake_phase: "waiting" as const,
    pipeline_running: false,
    error_message: null,
    current_step: null,
    can_retry_pipeline: false,
    has_checkpoint: false,
    can_resume_live: false,
    resume_status: "ready_to_start" as const,
    workspace_status: readyWorkspaceStatus(activeSession.project_description ?? null),
    last_activity_at: nowIso(),
  });
  state.promptBanks[sessionId] = emptyPromptBank(sessionId);
  return { session: cloneState(activeSession) };
}

export function retryMockSessionPipeline(sessionId: string): GetSessionResponse {
  const state = ensureState();
  const activeSession = findSession(state, sessionId);
  if (!activeSession) {
    throw new Error(`Mock session not found: ${sessionId}`);
  }

  Object.assign(activeSession, {
    intake_phase: "pipeline_running" as const,
    pipeline_running: true,
    can_retry_pipeline: true,
    workspace_status: pipelineWorkspaceStatus(),
    last_activity_at: nowIso(),
  });
  return { session: cloneState(activeSession) };
}

export function getMockPromptBank(sessionId: string): PromptBankResponse {
  const state = ensureState();
  return cloneState(state.promptBanks[sessionId] ?? emptyPromptBank(sessionId));
}

export function saveMockPromptDrafts(
  sessionId: string,
  payload: { promptId: string; answers: PromptAnswer[] },
): SavePromptDraftsResponse {
  const state = ensureState();
  const promptBank = state.promptBanks[sessionId];
  if (!promptBank) {
    throw new Error(`Mock prompt bank not found for session: ${sessionId}`);
  }
  const savedDrafts = { ...(promptBank.saved_drafts ?? {}) };
  let savedCount = 0;
  let clearedCount = 0;
  const savedAt = nowIso();

  for (const answer of payload.answers) {
    const structuredPayload = answer.structured_payload;
    const hasContent = Boolean(
      answer.selected_option_id
        || answer.custom_text?.trim()
        || structuredPayload?.ordered_option_ids?.length
        || Object.keys(structuredPayload?.field_values ?? {}).length
        || structuredPayload?.scalar_value !== undefined
        || structuredPayload?.selected_path?.trim(),
    );
    if (!hasContent || answer.skipped) {
      if (savedDrafts[answer.item_id]) {
        delete savedDrafts[answer.item_id];
        clearedCount += 1;
      }
      continue;
    }

    const savedDraft: SavedPromptAnswerDraft = {
      prompt_id: payload.promptId,
      item_id: answer.item_id,
      selected_option_id: answer.selected_option_id ?? null,
      custom_text: answer.custom_text ?? null,
      skipped: false,
      updated_at: savedAt,
    };
    if (structuredPayload) {
      savedDraft.structured_payload = structuredPayload;
    }
    savedDrafts[answer.item_id] = savedDraft;
    savedCount += 1;
  }

  state.promptBanks[sessionId] = {
    ...promptBank,
    saved_drafts: savedDrafts,
  };

  return {
    session_id: sessionId,
    prompt_id: payload.promptId,
    saved_count: savedCount,
    cleared_count: clearedCount,
    saved_at: savedAt,
  };
}

export function startMockSessionInterview(sessionId: string, description: string | null): PromptBankResponse {
  const state = ensureState();
  const activeSession = findSession(state, sessionId);
  if (!activeSession) {
    throw new Error(`Mock session not found: ${sessionId}`);
  }

  const promptBank = buildSessionPromptBank(sessionId);
  state.promptBanks[sessionId] = promptBank;
  Object.assign(activeSession, {
    intake_phase: "interviewing" as const,
    project_description: description?.trim() || activeSession.project_description || null,
    has_checkpoint: true,
    can_resume_live: true,
    resume_status: "interview_attached" as const,
    workspace_status: awaitingWorkspaceStatus(),
    last_activity_at: nowIso(),
  });
  return cloneState(promptBank);
}

export function completeMockSessionPrompt(
  sessionId: string,
  promptId: string,
  answers: PromptAnswer[],
): PromptBankResponse | null {
  const state = ensureState();
  saveMockPromptDrafts(sessionId, { promptId, answers });
  const activeSession = findSession(state, sessionId);
  if (!activeSession) {
    throw new Error(`Mock session not found: ${sessionId}`);
  }

  const promptBank = state.promptBanks[sessionId];
  if (sessionId === "session-11" && promptId === "prompt-session-11-workflow" && promptBank) {
    const workflowThread = promptBank.banked_threads.find(thread => thread.category_id === "workflow");
    if (workflowThread) {
      const nextItems = workflowThread.prompt.items.filter(item => item.item_id !== "item-session-11-1");
      const nextBank: PromptBankResponse = {
        ...promptBank,
        active_thread_id: "workflow",
        banked_threads: promptBank.banked_threads.map(thread =>
          thread.category_id === "workflow"
            ? {
                ...thread,
                question_count: nextItems.length,
                prompt: {
                  ...thread.prompt,
                  prompt_id: "prompt-session-11-workflow-next",
                  items: nextItems,
                },
              }
            : thread,
        ),
        saved_drafts: {},
      };
      state.promptBanks[sessionId] = nextBank;
      Object.assign(activeSession, {
        intake_phase: "interviewing" as const,
        can_resume_live: true,
        has_checkpoint: true,
        resume_status: "interview_attached" as const,
        workspace_status: awaitingWorkspaceStatus(),
        last_activity_at: nowIso(),
      });
      return cloneState(nextBank);
    }
  }

  Object.assign(activeSession, {
    intake_phase: "complete" as const,
    can_resume_live: false,
    has_checkpoint: false,
    resume_status: "ready_to_start" as const,
    workspace_status: completeWorkspaceStatus(),
    last_activity_at: nowIso(),
  });
  return null;
}

export function getMockProjectImportReview(projectRef: string): ProjectImportResponse | null {
  const state = ensureState();
  return cloneState(state.importReviewByProjectRef[projectRef] ?? null);
}

export function updateMockProjectImportReviewSelection(
  projectRef: string,
  request: { nodeId: string; included: boolean },
): ProjectImportResponse {
  const state = ensureState();
  const current = state.importReviewByProjectRef[projectRef];
  if (!current) {
    throw new Error(`Mock import review not found: ${projectRef}`);
  }

  const nextReviewNodes = (current.review_nodes ?? []).map(node =>
    node.node_id === request.nodeId ? { ...node, included: request.included } : node,
  );
  const nextResponse: ProjectImportResponse = {
    ...current,
    review_nodes: nextReviewNodes,
    import_review_selection: {
      job_id: current.import_job.id,
      excluded_node_ids: nextReviewNodes.filter(node => !node.included).map(node => node.node_id),
      included_node_count: nextReviewNodes.filter(node => node.included).length,
      excluded_node_count: nextReviewNodes.filter(node => !node.included).length,
    },
  };
  storeProjectScoped(state.importReviewByProjectRef, current.project, nextResponse);
  return cloneState(nextResponse);
}

export function applyMockProjectImportReview(projectRef: string): ProjectImportResponse {
  const state = ensureState();
  const current = state.importReviewByProjectRef[projectRef];
  if (!current) {
    throw new Error(`Mock import review not found: ${projectRef}`);
  }

  const applied: ProjectImportResponse = {
    ...current,
    import_job: {
      ...current.import_job,
      status: "applied",
      progress_message: "Mock import review applied.",
      analysis_summary: "The imported project structure is now attached to the stable project state.",
      updated_at: nowIso(),
    },
    import_review_selection: null,
    review_nodes: null,
  };
  storeProjectScoped(state.importReviewByProjectRef, current.project, null);
  storeProjectScoped(state.importStateByProjectRef, current.project, applied);

  const history = state.importHistoryByProjectRef[current.project.slug];
  if (history) {
    const nextHistory: ProjectImportHistoryResponse = {
      ...history,
      history: [
        {
          import_job: applied.import_job,
          source_metadata: history.source_binding
            ? {
                provider: history.source_binding.provider,
                canonical_ref: history.source_binding.canonical_ref,
                local_root: history.source_binding.local_root ?? `/mock/${current.project.slug}`,
                default_branch: history.source_binding.default_branch ?? "main",
                head_revision: history.source_binding.head_revision ?? null,
              }
            : null,
          discovered_node_count: (current.review_nodes ?? []).length,
          effective_included_node_count: current.import_review_selection?.included_node_count ?? 0,
          effective_excluded_node_count: current.import_review_selection?.excluded_node_count ?? 0,
        },
        ...history.history,
      ],
    };
    storeProjectScoped(state.importHistoryByProjectRef, current.project, nextHistory);
  }

  return cloneState(applied);
}

export function getMockProjectImportState(projectRef: string): ProjectImportResponse | null {
  const state = ensureState();
  return cloneState(state.importStateByProjectRef[projectRef] ?? null);
}

export function getMockProjectImportHistory(projectRef: string): ProjectImportHistoryResponse | null {
  const state = ensureState();
  return cloneState(state.importHistoryByProjectRef[projectRef] ?? null);
}

export function getMockProjectImportHistoryComparison(
  projectRef: string,
  jobId: string,
): ProjectImportHistoryComparisonResponse {
  const state = ensureState();
  const projectRecord = findProject(state, projectRef);
  const history = state.importHistoryByProjectRef[projectRef];
  const current = latestImportJob(state, projectRecord ?? ({ id: "", slug: "", name: "", owner_user_id: "", created_at: "", updated_at: "", legacy_scope_keys: [] } as Project));
  if (!projectRecord || !history || !current) {
    throw new Error(`Mock import comparison not found: ${projectRef}`);
  }
  const selectedEntry = history.history.find(entry => entry.import_job.id === jobId);
  if (!selectedEntry) {
    throw new Error(`Mock import history entry not found: ${jobId}`);
  }
  return cloneState({
    project: projectRecord,
    source_binding: history.source_binding,
    selected_entry: selectedEntry,
    current_import_job: current.import_job,
    selected_entry_uses_selection_filter: Boolean(selectedEntry.effective_excluded_node_count),
    current_import_job_uses_selection_filter: Boolean(current.import_review_selection?.excluded_node_count),
    diff_summary: comparisonDiff(current.import_job.id, selectedEntry.import_job.id),
  });
}

export function getMockProjectImportHistoryPairComparison(
  projectRef: string,
  baseJobId: string,
  jobId: string,
): ProjectImportHistoryPairComparisonResponse {
  const state = ensureState();
  const projectRecord = findProject(state, projectRef);
  const history = state.importHistoryByProjectRef[projectRef];
  if (!projectRecord || !history) {
    throw new Error(`Mock import pair comparison not found: ${projectRef}`);
  }
  const baselineEntry = history.history.find(entry => entry.import_job.id === baseJobId);
  const comparedEntry = history.history.find(entry => entry.import_job.id === jobId);
  if (!baselineEntry || !comparedEntry) {
    throw new Error(`Mock import history entries not found for pair comparison.`);
  }
  return cloneState({
    project: projectRecord,
    source_binding: history.source_binding,
    baseline_entry: baselineEntry,
    compared_entry: comparedEntry,
    baseline_entry_uses_selection_filter: Boolean(baselineEntry.effective_excluded_node_count),
    compared_entry_uses_selection_filter: Boolean(comparedEntry.effective_excluded_node_count),
    diff_summary: comparisonDiff(comparedEntry.import_job.id, baselineEntry.import_job.id),
  });
}

function setCurrentImportResponse(state: MockState, response: ProjectImportResponse | null): void {
  if (!response) return;
  storeProjectScoped(state.importStateByProjectRef, response.project, response.import_job.status === "applied" ? response : null);
  storeProjectScoped(state.importReviewByProjectRef, response.project, response.import_job.status === "review_pending" ? response : null);
}

export function restoreMockProjectImportHistoryEntry(projectRef: string, jobId: string): ProjectImportResponse {
  const state = ensureState();
  const projectRecord = findProject(state, projectRef);
  const history = state.importHistoryByProjectRef[projectRef];
  if (!projectRecord || !history) {
    throw new Error(`Mock import history not found: ${projectRef}`);
  }
  const entry = history.history.find(candidate => candidate.import_job.id === jobId);
  if (!entry) {
    throw new Error(`Mock import history entry not found: ${jobId}`);
  }
  const restored = projectImportResponseForHistory(projectRecord, entry, "applied");
  setCurrentImportResponse(state, restored);
  return cloneState(restored);
}

export function restoreMockProjectImportHistoryEntryForReview(projectRef: string, jobId: string): ProjectImportResponse {
  const state = ensureState();
  const projectRecord = findProject(state, projectRef);
  const history = state.importHistoryByProjectRef[projectRef];
  if (!projectRecord || !history) {
    throw new Error(`Mock import history not found: ${projectRef}`);
  }
  const entry = history.history.find(candidate => candidate.import_job.id === jobId);
  if (!entry) {
    throw new Error(`Mock import history entry not found: ${jobId}`);
  }
  const restored = projectImportResponseForHistory(projectRecord, entry, "review_pending");
  setCurrentImportResponse(state, restored);
  return cloneState(restored);
}

export function restoreMockProjectImportReviewDraft(projectRef: string, jobId: string): ProjectImportResponse {
  return restoreMockProjectImportHistoryEntryForReview(projectRef, jobId);
}

export function reimportMockProject(projectRef: string): ProjectImportResponse {
  const state = ensureState();
  const projectRecord = findProject(state, projectRef);
  if (!projectRecord) {
    throw new Error(`Mock project not found: ${projectRef}`);
  }
  const response: ProjectImportResponse = {
    project: projectRecord,
    import_job: {
      id: `import-${projectRecord.id}-reimport-${Date.now()}`,
      project_id: projectRecord.id,
      provider: "github",
      requested_ref: `thetu/${projectRecord.slug}`,
      status: "review_pending",
      restored_from_job_id: null,
      seed_session_id: state.sessions.find(session => session.project_id === projectRecord.id)?.id ?? null,
      analysis_summary: "Mock re-import generated a fresh review draft.",
      progress_message: "Re-import request queued in frontend mock mode.",
      error_message: null,
      created_at: nowIso(),
      updated_at: nowIso(),
    },
    source_binding: {
      project_id: projectRecord.id,
      provider: "github",
      canonical_ref: `thetu/${projectRecord.slug}`,
      default_branch: "main",
      head_revision: "abc1234",
      local_root: `/mock/${projectRecord.slug}`,
      managed_checkout: true,
      created_at: nowIso(),
      updated_at: nowIso(),
    },
    import_draft: {
      job_id: `import-${projectRecord.id}-draft-${Date.now()}`,
      project_id: projectRecord.id,
      analysis_summary: "Mock re-import refreshed route and dependency discovery.",
      source_metadata: {
        provider: "github",
        canonical_ref: `thetu/${projectRecord.slug}`,
        local_root: `/mock/${projectRecord.slug}`,
        default_branch: "main",
        head_revision: "abc1234",
      },
      discovered_nodes: [],
      created_at: nowIso(),
      updated_at: nowIso(),
    },
    import_review_selection: {
      job_id: `import-${projectRecord.id}-selection-${Date.now()}`,
      excluded_node_ids: [],
      included_node_count: 2,
      excluded_node_count: 0,
    },
    review_nodes: [
      {
        node_id: `reimport-${projectRecord.id}-1`,
        node_name: "Refreshed route topology",
        node_type: "component",
        included: true,
      },
      {
        node_id: `reimport-${projectRecord.id}-2`,
        node_name: "Runtime sync rule",
        node_type: "constraint",
        included: true,
      },
    ],
  };
  setCurrentImportResponse(state, response);
  return cloneState(response);
}

export function getMockProjectBlueprint(projectRef: string): BlueprintResponse {
  const state = ensureState();
  const blueprint = state.blueprintsByProjectRef[projectRef];
  if (!blueprint) {
    throw new Error(`Mock blueprint not found: ${projectRef}`);
  }
  return cloneState(blueprint);
}

export function listMockBlueprintHistory(): HistoryListResponse {
  const state = ensureState();
  return cloneState(state.blueprintHistory);
}

export function createMockBlueprintSnapshot(): { timestamp: string; filename: string } {
  const state = ensureState();
  const timestamp = nowIso();
  const filename = `blueprint-${state.nextSnapshotIndex++}-${timestamp.replace(/[:.]/g, "-")}.json`;
  state.blueprintHistory.snapshots.unshift({ timestamp, filename });
  state.blueprintEvents.unshift({
    event_type: "export_recorded",
    summary: "Recorded a new mock blueprint snapshot.",
    timestamp,
    data: { filename },
  });
  return { timestamp, filename };
}

export function listMockBlueprintEvents(params?: { nodeId?: string; limit?: number }): BlueprintEventsResponse {
  const state = ensureState();
  const events = (state.blueprintEvents ?? [])
    .filter(event => !params?.nodeId || event.data?.node_id === params.nodeId)
    .slice(0, params?.limit ?? state.blueprintEvents.length);
  return {
    events: cloneState(events),
    total: events.length,
  };
}

export function listMockBlueprintExportHistory(params?: { projectId?: string; limit?: number }): BlueprintExportHistoryResponse {
  const state = ensureState();
  const source = params?.projectId
    ? state.blueprintExportHistoryByProjectRef[params.projectId] ?? { entries: [], total: 0 }
    : {
        entries: Object.values(state.blueprintExportHistoryByProjectRef)
          .flatMap(response => response.entries)
          .sort((left, right) => Date.parse(right.timestamp) - Date.parse(left.timestamp)),
        total: Object.values(state.blueprintExportHistoryByProjectRef).reduce((sum, response) => sum + response.total, 0),
      };

  return {
    entries: cloneState(source.entries.slice(0, params?.limit ?? source.entries.length)),
    total: source.total,
  };
}

export function runMockDiscoveryScan(): DiscoveryRunResponse {
  const state = ensureState();
  const timestamp = nowIso();
  const projectRecord = state.projects[0] ?? null;
  const pendingNodeCount = state.proposedNodes.filter(proposal => proposal.status === "pending").length;
  const pendingEdgeCount = state.proposedEdges.filter(proposal => proposal.status === "pending").length;

  if (projectRecord && pendingNodeCount === 0) {
    const proposalIndex = state.nextProposalIndex++;
    state.proposedNodes.unshift({
      id: `proposal-node-${proposalIndex}`,
      node: {
        id: `proposal-node-${proposalIndex}-node`,
        node_type: "component",
        name: "Scan-created route group",
        scope: {
          project: {
            project_id: projectRecord.id,
            project_name: projectRecord.name,
          },
          secondary: {},
        },
      },
      source: "directory_scan",
      reason: "Mock discovery scan refreshed pending route-group work.",
      status: "pending",
      proposed_at: timestamp,
      confidence: 0.77,
      source_artifact: "src/routes",
    });
  }

  if (projectRecord && pendingEdgeCount === 0) {
    const targetNode =
      state.proposedNodes.find(proposal => proposal.status === "pending")?.node.id
      ?? state.proposedNodes[0]?.node.id
      ?? `${projectRecord.id}-project`;
    const edgeIndex = state.nextProposalIndex++;
    state.proposedEdges.unshift({
      id: `edge-proposal-${edgeIndex}`,
      edge: {
        source: `${projectRecord.id}-decision`,
        target: targetNode,
        edge_type: "decided_by",
      },
      source: "code_graph_context",
      reason: "Mock discovery scan linked the refreshed route group to the current project decision.",
      status: "pending",
      proposed_at: timestamp,
      confidence: 0.71,
      source_artifact: "graph.json",
    });
  }

  state.blueprintEvents.unshift({
    event_type: "node_updated",
    summary: "Discovery scan refreshed the mock proposal queue.",
    timestamp,
    data: {
      pending_nodes: state.proposedNodes.filter(proposal => proposal.status === "pending").length,
      pending_edges: state.proposedEdges.filter(proposal => proposal.status === "pending").length,
    },
  });

  return {
    results: [
      {
        scanner: "all",
        proposed_count: state.proposedNodes.filter(proposal => proposal.status === "pending").length,
        skipped_count: 0,
        proposed_edge_count: state.proposedEdges.filter(proposal => proposal.status === "pending").length,
        skipped_edge_count: 0,
        errors: [],
        duration_ms: 420,
      },
    ],
    total_proposed: state.proposedNodes.filter(proposal => proposal.status === "pending").length,
    total_edge_proposed: state.proposedEdges.filter(proposal => proposal.status === "pending").length,
  };
}

export function listMockProposedNodes(status?: string): ProposedNodesResponse {
  const state = ensureState();
  const proposals = status
    ? state.proposedNodes.filter(proposal => proposal.status === status)
    : state.proposedNodes;
  return {
    proposals: cloneState(proposals),
    total: proposals.length,
  };
}

export function listMockProposedEdges(status?: string): ProposedEdgesResponse {
  const state = ensureState();
  const proposals = status
    ? state.proposedEdges.filter(proposal => proposal.status === status)
    : state.proposedEdges;
  return {
    proposals: cloneState(proposals),
    total: proposals.length,
  };
}

function updateProposalStatus<T extends { id: string; status: string; reviewed_at?: string }>(
  proposals: T[],
  proposalId: string,
  status: string,
): T {
  const proposal = proposals.find(candidate => candidate.id === proposalId);
  if (!proposal) {
    throw new Error(`Mock proposal not found: ${proposalId}`);
  }
  proposal.status = status;
  proposal.reviewed_at = nowIso();
  return proposal;
}

export function acceptMockProposal(proposalId: string): { node_id: string; message: string } {
  const state = ensureState();
  const proposal = updateProposalStatus(state.proposedNodes, proposalId, "accepted");
  return { node_id: proposal.node.id, message: "Mock proposal accepted." };
}

export function rejectMockProposal(proposalId: string): { message: string } {
  const state = ensureState();
  updateProposalStatus(state.proposedNodes, proposalId, "rejected");
  return { message: "Mock proposal rejected." };
}

export function acceptMockEdgeProposal(proposalId: string): { edge: unknown; message: string } {
  const state = ensureState();
  const proposal = updateProposalStatus(state.proposedEdges, proposalId, "accepted");
  return { edge: cloneState(proposal.edge), message: "Mock edge proposal accepted." };
}

export function rejectMockEdgeProposal(proposalId: string): { message: string } {
  const state = ensureState();
  updateProposalStatus(state.proposedEdges, proposalId, "rejected");
  return { message: "Mock edge proposal rejected." };
}

export function getMockAdminStatus(): AdminStatusResponse {
  const state = ensureState();
  return cloneState(state.adminStatus);
}

export function getMockAdminEvents(params?: { limit?: number; level?: string; sessionId?: string }): AdminEventsResponse {
  const state = ensureState();
  const events = state.adminEvents
    .filter(event => !params?.level || event.level === params.level)
    .filter(event => !params?.sessionId || event.session_id === params.sessionId)
    .slice(0, params?.limit ?? state.adminEvents.length);
  return {
    events: cloneState(events),
    total: events.length,
  };
}

export function getMockSessionRuns(sessionId: string): { runs: string[] } {
  const activeSession = getMockSession(sessionId).session;
  return {
    runs: activeSession.intake_phase === "complete" || activeSession.intake_phase === "pipeline_running"
      ? [`run-${sessionId}-1`, `run-${sessionId}-2`]
      : [],
  };
}

export function getMockSessionEvents(sessionId: string): SessionEventsResponse {
  const activeSession = getMockSession(sessionId).session;
  const timestamp = nowIso();
  const events = activeSession.intake_phase === "complete" || activeSession.intake_phase === "pipeline_running"
    ? [
        {
          id: `event-${sessionId}-1`,
          timestamp,
          level: "info" as const,
          source: "pipeline" as const,
          session_id: sessionId,
          step: "mock_pipeline",
          message: "Mock pipeline event recorded for the current session.",
          metadata: {},
        },
      ]
    : [];

  return {
    session_id: sessionId,
    events,
    count: events.length,
  };
}
