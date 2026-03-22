/**
 * useSocraticWebSocket — WebSocket hook for the Socratic interview.
 *
 * Connects to /api/sessions/:id/socratic/ws, dispatches typed
 * ServerWsMessages into React state, and provides send helpers.
 *
 * After convergence, the same socket transitions to pipeline-poll mode
 * (the server keeps it alive and pushes stage_update / pipeline_complete).
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { WS_PROTOCOL } from '../config.ts';
import { uuidv4 } from '../lib/uuid.ts';
import {
  buildUiCapabilities,
  sameUiCapabilities,
} from './uiCapabilities.ts';
import type {
  BeliefState,
  ChatMessage,
  Classification,
  ClientWsMessage,
  Contradiction,
  EventLevel,
  EventSourceType,
  IntakePhase,
  PipelineStage,
  PipelineStageName,
  PlannerEvent,
  PromptAnswer,
  SocraticCategorySnapshot,
  SocraticWorkspaceSnapshot,
  PromptEnvelope,
  ServerWsMessage,
  Session,
  SpeculativeDraft,
  StageStatus,
} from '../types.ts';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

const PIPELINE_STAGE_NAMES: PipelineStageName[] = [
  'Intake', 'Chunk', 'Compile', 'Lint', 'AR Review', 'Refine',
  'Scenarios', 'Ralph', 'Graph', 'Factory', 'Validate', 'Git',
];

function buildInitialStages(): PipelineStage[] {
  return PIPELINE_STAGE_NAMES.map((name) => ({ name, status: 'pending' as StageStatus }));
}

function normalizeDimensionLabel(value: unknown): string {
  if (typeof value === 'string') return value;
  if (value && typeof value === 'object') {
    const keys = Object.keys(value as Record<string, unknown>);
    if (keys.length === 1) {
      const inner = (value as Record<string, unknown>)[keys[0]];
      if (typeof inner === 'string') return inner;
    }
  }
  return JSON.stringify(value);
}

function hydrateDraftFromPrompt(prompt: PromptEnvelope | null): SpeculativeDraft | null {
  if (!prompt) return null;
  const draft = prompt.draft_snapshot;
  if (!draft) return null;
  return {
    sections: draft.sections.map((section) => ({
      heading: section.heading,
      content: section.content,
    })),
    assumptions: draft.assumptions.map((assumption) => ({
      dimension: normalizeDimensionLabel((assumption as { dimension?: unknown }).dimension),
      assumption: assumption.assumption,
    })),
    not_discussed: (draft.not_discussed ?? []).map(normalizeDimensionLabel),
  };
}

function buildInitialSessionSignature(
  session: Session | null,
  sessionId: string | null,
): string | null {
  if (!session || !sessionId || session.id !== sessionId) return null;

  return [
    session.id,
    session.intake_phase,
    session.pipeline_running ? '1' : '0',
    session.resume_status,
    String(session.messages?.length ?? 0),
    String(session.events?.length ?? 0),
    session.current_step ?? '',
    session.error_message ?? '',
    session.checkpoint?.last_checkpoint_at ?? '',
    session.stages?.map((stage) => `${stage.name}:${stage.status}`).join('|') ?? '',
  ].join('::');
}

function dedupeEventsById(events: PlannerEvent[]): PlannerEvent[] {
  const seen = new Set<string>();
  const deduped: PlannerEvent[] = [];
  for (const event of events) {
    if (seen.has(event.id)) continue;
    seen.add(event.id);
    deduped.push(event);
  }
  return deduped;
}

function isPipelineStageName(value: unknown): value is PipelineStageName {
  return typeof value === 'string' && PIPELINE_STAGE_NAMES.includes(value as PipelineStageName);
}

function toRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function normalizePipelineStageName(raw: string): PipelineStageName | null {
  const normalized = raw.trim().toLowerCase();
  switch (normalized) {
    case 'intake':
      return 'Intake';
    case 'chunk':
    case 'chunk planning':
      return 'Chunk';
    case 'compile':
    case 'specification compilation':
      return 'Compile';
    case 'lint':
      return 'Lint';
    case 'ar review':
    case 'adversarial review':
      return 'AR Review';
    case 'refine':
    case 'refinement':
      return 'Refine';
    case 'scenarios':
    case 'scenario generation':
      return 'Scenarios';
    case 'ralph':
    case 'ralph advisory':
      return 'Ralph';
    case 'graph':
    case 'graph compilation':
      return 'Graph';
    case 'factory':
      return 'Factory';
    case 'validate':
    case 'validation':
      return 'Validate';
    case 'git':
    case 'git projection':
      return 'Git';
    default:
      return null;
  }
}

function getPipelineStageFromMetadata(metadata: Record<string, unknown>): PipelineStageName | null {
  if (typeof metadata.stage === 'string') {
    const normalized = normalizePipelineStageName(metadata.stage);
    if (normalized) return normalized;
  }
  if (typeof metadata.stage_name === 'string') {
    const normalized = normalizePipelineStageName(metadata.stage_name);
    if (normalized) return normalized;
  }
  if (typeof metadata.pipeline_stage === 'string') {
    const normalized = normalizePipelineStageName(metadata.pipeline_stage);
    if (normalized) return normalized;
  }
  if (isPipelineStageName(metadata.stage)) return metadata.stage;
  const details = toRecord(metadata.details);
  if (details) {
    const detailStage = details.stage;
    const detailStageName = details.stage_name;
    if (typeof detailStage === 'string') {
      const normalized = normalizePipelineStageName(detailStage);
      if (normalized) return normalized;
    }
    if (typeof detailStageName === 'string') {
      const normalized = normalizePipelineStageName(detailStageName);
      if (normalized) return normalized;
    }
    if (isPipelineStageName(details.stage)) return details.stage;
  }
  return null;
}

function inferPipelineStageFromText(text: string | undefined): PipelineStageName | null {
  if (!text) return null;
  const lower = text.toLowerCase();
  const patterns: Array<[string, PipelineStageName]> = [
    ['intake stage', 'Intake'],
    ['chunk planning stage', 'Chunk'],
    ['chunk stage', 'Chunk'],
    ['specification compilation stage', 'Compile'],
    ['compile stage', 'Compile'],
    ['lint stage', 'Lint'],
    ['adversarial review stage', 'AR Review'],
    ['ar review stage', 'AR Review'],
    ['refinement stage', 'Refine'],
    ['refine stage', 'Refine'],
    ['scenario generation stage', 'Scenarios'],
    ['scenarios stage', 'Scenarios'],
    ['ralph advisory stage', 'Ralph'],
    ['graph compilation stage', 'Graph'],
    ['factory stage', 'Factory'],
    ['validation stage', 'Validate'],
    ['git projection stage', 'Git'],
  ];
  const match = patterns.find(([needle]) => lower.includes(needle));
  return match ? match[1] : null;
}

function soleRunningStage(stages: PipelineStage[]): PipelineStageName | null {
  const running = stages.filter((stage) => stage.status === 'running');
  if (running.length !== 1) return null;
  return running[0].name;
}

function getBooleanMetadata(metadata: Record<string, unknown>, key: string): boolean | null {
  if (typeof metadata[key] === 'boolean') return metadata[key] as boolean;
  const details = toRecord(metadata.details);
  if (details && typeof details[key] === 'boolean') return details[key] as boolean;
  return null;
}

function applyPipelineEventToStages(
  stages: PipelineStage[],
  step: string | undefined,
  metadata: Record<string, unknown>,
  message?: string,
): PipelineStage[] {
  if (!step) return stages;

  const stage =
    getPipelineStageFromMetadata(metadata)
    ?? inferPipelineStageFromText(message);

  switch (step) {
    case 'pipeline.stage.started': {
      if (!stage) return stages;
      return stages.map((entry) => (
        entry.name === stage ? { ...entry, status: 'running' } : entry
      ));
    }
    case 'pipeline.stage.completed': {
      const completedStage = stage ?? soleRunningStage(stages);
      if (!completedStage) return stages;
      return stages.map((entry) => (
        entry.name === completedStage ? { ...entry, status: 'complete' } : entry
      ));
    }
    case 'pipeline.stage.failed': {
      const failedStage = stage ?? soleRunningStage(stages);
      if (!failedStage) return stages;
      return stages.map((entry) => (
        entry.name === failedStage ? { ...entry, status: 'failed' } : entry
      ));
    }
    case 'pipeline.retry.started': {
      const retryStage: PipelineStageName = stage ?? soleRunningStage(stages) ?? 'Factory';
      return stages.map((entry) => (
        entry.name === retryStage ? { ...entry, status: 'running' } : entry
      ));
    }
    case 'pipeline.validation.completed': {
      const validationStage: PipelineStageName = stage ?? 'Validate';
      const gatesPassed = getBooleanMetadata(metadata, 'gates_passed');
      if (gatesPassed === null) return stages;
      return stages.map((entry) => (
        entry.name === validationStage
          ? { ...entry, status: gatesPassed ? 'complete' : 'failed' }
          : entry
      ));
    }
    default:
      return stages;
  }
}

type GetTokenFn = () => Promise<string>;

export interface UseSocraticWebSocketOptions {
  sessionId: string | null;
  getToken: GetTokenFn;
  initialSession?: Session | null;
}

export interface UseSocraticWebSocketResult {
  // Connection
  isConnected: boolean;
  reconnectFailed: boolean;

  // Intake phase
  intakePhase: IntakePhase;

  // Chat
  messages: ChatMessage[];

  // Socratic interview state
  classification: Classification | null;
  beliefState: BeliefState | null;
  convergencePct: number;
  currentCategorySnapshot: SocraticCategorySnapshot | null;
  currentWorkspace: SocraticWorkspaceSnapshot | null;
  pendingCategoryId: string | null;
  workspaceNotice: string | null;
  currentPrompt: PromptEnvelope | null;
  speculativeDraft: SpeculativeDraft | null;
  confirmedSections: Set<string>;
  contradictions: Contradiction[];

  // Pipeline state (active after convergence)
  stages: PipelineStage[];
  pipelineComplete: boolean;
  pipelineSummary: string | null;

  // Observability
  events: PlannerEvent[];
  currentStep: string | null;

  // Actions
  attach: () => void;
  sendDescription: (description: string) => void;
  submitPromptAnswers: (answers: PromptAnswer[]) => void;
  enterCategory: (categoryId: string, revision: string) => void;
  backToCategories: () => void;
  sendDone: () => void;
  sendDimensionEdit: (dimension: string, newValue: string) => void;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_RETRIES = 3;
const BASE_DELAY_MS = 1000;

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useSocraticWebSocket({
  sessionId,
  getToken,
  initialSession = null,
}: UseSocraticWebSocketOptions): UseSocraticWebSocketResult {
  // Connection
  const [isConnected, setIsConnected] = useState(false);
  const [reconnectFailed, setReconnectFailed] = useState(false);

  // Phase
  const [intakePhase, setIntakePhase] = useState<IntakePhase>('waiting');

  // Chat
  const [messages, setMessages] = useState<ChatMessage[]>([]);

  // Socratic
  const [classification, setClassification] = useState<Classification | null>(null);
  const [beliefState, setBeliefState] = useState<BeliefState | null>(null);
  const [convergencePct, setConvergencePct] = useState(0);
  const [currentCategorySnapshot, setCurrentCategorySnapshot] = useState<SocraticCategorySnapshot | null>(null);
  const [currentWorkspace, setCurrentWorkspace] = useState<SocraticWorkspaceSnapshot | null>(null);
  const [pendingCategoryId, setPendingCategoryId] = useState<string | null>(null);
  const [workspaceNotice, setWorkspaceNotice] = useState<string | null>(null);
  const [currentPrompt, setCurrentPrompt] = useState<PromptEnvelope | null>(null);
  const [speculativeDraft, setSpeculativeDraft] = useState<SpeculativeDraft | null>(null);
  const [contradictions, setContradictions] = useState<Contradiction[]>([]);

  // Draft section confirmation state — survives across re-renders, tab switches,
  // and new draft arrivals. Keyed by section target ("0", "1", … or "assumptions").
  const [confirmedSections, setConfirmedSections] = useState<Set<string>>(new Set());

  // Pipeline
  const [stages, setStages] = useState<PipelineStage[]>(buildInitialStages);
  const [pipelineComplete, setPipelineComplete] = useState(false);
  const [pipelineSummary, setPipelineSummary] = useState<string | null>(null);

  // Observability
  const [events, setEvents] = useState<PlannerEvent[]>([]);
  const [currentStep, setCurrentStep] = useState<string | null>(null);

  // Refs
  const wsRef = useRef<WebSocket | null>(null);
  const retryCountRef = useRef(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);
  const sessionIdRef = useRef(sessionId);
  const intakePhaseRef = useRef<IntakePhase>('waiting');
  const currentPromptRef = useRef<PromptEnvelope | null>(null);
  const uiCapabilitiesRef = useRef(buildUiCapabilities());
  const lastSentUiCapabilitiesRef = useRef<typeof uiCapabilitiesRef.current | null>(null);
  const hydratedSnapshotRef = useRef<string | null>(null);

  useEffect(() => { sessionIdRef.current = sessionId; }, [sessionId]);
  useEffect(() => { intakePhaseRef.current = intakePhase; }, [intakePhase]);
  useEffect(() => { currentPromptRef.current = currentPrompt; }, [currentPrompt]);

  const clearRetryTimer = (): void => {
    if (retryTimerRef.current !== null) {
      clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
  };

  // -------------------------------------------------------------------------
  // Message dispatch
  // -------------------------------------------------------------------------

  const handleServerMessage = useCallback((msg: ServerWsMessage): void => {
    switch (msg.type) {
      // --- Socratic messages ---

      case 'classified': {
        setClassification({
          project_type: msg.project_type,
          complexity: msg.complexity,
        });
        // Add a planner message for the chat
        setMessages((prev) => [...prev, {
          id: uuidv4(),
          role: 'planner',
          content: `Classified as: **${msg.project_type}** (${msg.complexity}).`,
          timestamp: new Date().toISOString(),
        }]);
        break;
      }

      case 'belief_state_update': {
        const bs: BeliefState = {
          filled: msg.filled as BeliefState['filled'],
          uncertain: msg.uncertain as BeliefState['uncertain'],
          missing: (msg.missing ?? []).map(normalizeDimensionLabel),
          out_of_scope: (msg.out_of_scope ?? []).map(normalizeDimensionLabel),
          convergence_pct: msg.convergence_pct,
        };
        setBeliefState(bs);
        setConvergencePct(msg.convergence_pct);
        break;
      }

      case 'prompt': {
        const prompt = msg.prompt;
        setCurrentPrompt(prompt);
        setPendingCategoryId(null);
        setWorkspaceNotice(null);
        if (prompt.category_path.length > 0) {
          setCurrentCategorySnapshot((previous) => (
            previous
              ? {
                ...previous,
                active_category_path: prompt.category_path,
              }
              : previous
          ));
          const focusedCategoryId = prompt.origin_category_id
            ?? prompt.category_path[prompt.category_path.length - 1]?.category_id
            ?? null;
          setCurrentWorkspace((previous) => (
            previous
              ? {
                ...previous,
                focused_category_id: focusedCategoryId,
                groups: previous.groups.map((group) => ({
                  ...group,
                  is_focused: focusedCategoryId === group.category_id,
                })),
              }
              : previous
          ));
        }
        setSpeculativeDraft(hydrateDraftFromPrompt(prompt));

        const promptSummary = prompt.items?.length === 1
          ? prompt.items[0].text
          : `${prompt.title} (${prompt.items.length} items)`;
        if (promptSummary) {
          setMessages((prev) => [...prev, {
            id: uuidv4(),
            role: 'planner',
            content: promptSummary,
            timestamp: new Date().toISOString(),
          }]);
        }
        break;
      }

      case 'category_state': {
        setCurrentCategorySnapshot(msg.snapshot);
        setCurrentPrompt(null);
        setSpeculativeDraft(null);
        break;
      }

      case 'workspace_state': {
        setCurrentWorkspace(msg.workspace);
        setCurrentCategorySnapshot(msg.workspace.category_snapshot);
        setWorkspaceNotice(msg.workspace.branch_notice ?? null);
        setPendingCategoryId((previous) => (
          previous && msg.workspace.focused_category_id !== previous ? previous : null
        ));
        break;
      }

      case 'converged': {
        setConvergencePct(msg.convergence_pct);
        setCurrentCategorySnapshot(null);
        setCurrentWorkspace(null);
        setPendingCategoryId(null);
        setWorkspaceNotice(null);
        setCurrentPrompt(null);
        setIntakePhase('pipeline_running');
        setMessages((prev) => [...prev, {
          id: uuidv4(),
          role: 'planner',
          content: `Requirements gathering complete (${Math.round(msg.convergence_pct * 100)}% converged). Starting the planning pipeline\u2026`,
          timestamp: new Date().toISOString(),
        }]);
        break;
      }

      case 'contradiction_detected': {
        const c: Contradiction = {
          dimension_a: msg.dimension_a,
          value_a: msg.value_a,
          dimension_b: msg.dimension_b,
          value_b: msg.value_b,
          explanation: msg.explanation,
        };
        setContradictions((prev) => [...prev, c]);
        setMessages((prev) => [...prev, {
          id: uuidv4(),
          role: 'system',
          content: `\u26a0 Contradiction detected: ${msg.dimension_a} ("${msg.value_a}") conflicts with ${msg.dimension_b} ("${msg.value_b}") \u2014 ${msg.explanation}`,
          timestamp: new Date().toISOString(),
        }]);
        break;
      }

      // --- Pipeline messages (post-convergence) ---

      case 'stage_update': {
        setStages((prev) =>
          prev.map((s) =>
            s.name === msg.stage ? { ...s, status: msg.status } : s,
          ),
        );
        break;
      }

      case 'message': {
        if (msg.role === 'event') {
          // Legacy compatibility: hide operational event-role messages from chat.
          break;
        }
        const cm: ChatMessage = {
          id: msg.id ?? uuidv4(),
          role: msg.role,
          content: msg.content,
          timestamp: msg.timestamp ?? new Date().toISOString(),
        };
        setMessages((prev) => (
          prev.some(existing => existing.id === cm.id) ? prev : [...prev, cm]
        ));
        break;
      }

      case 'pipeline_complete': {
        setPipelineComplete(true);
        setPipelineSummary(msg.summary);
        intakePhaseRef.current = 'complete';
        clearRetryTimer();
        setIntakePhase('complete');
        break;
      }

      case 'error': {
        console.error('[Socratic WS] server error:', msg.message);
        setMessages((prev) => [...prev, {
          id: uuidv4(),
          role: 'system',
          content: `Error: ${msg.message}`,
          timestamp: new Date().toISOString(),
        }]);
        if (intakePhaseRef.current !== 'complete') {
          intakePhaseRef.current = 'error';
          clearRetryTimer();
        }
        setIntakePhase((prev) => {
          if (prev === 'complete') return prev;
          if (prev === 'waiting' || prev === 'interviewing' || prev === 'pipeline_running') {
            return 'error';
          }
          return prev;
        });
        break;
      }

      case 'planner_event': {
        const event: PlannerEvent = {
          id: msg.id,
          timestamp: msg.timestamp,
          level: msg.level as EventLevel,
          source: msg.source as EventSourceType,
          step: msg.step,
          message: msg.message,
          duration_ms: msg.duration_ms,
          metadata: msg.metadata ?? {},
        };
        setEvents((prev) => (
          prev.some(existing => existing.id === event.id) ? prev : [event, ...prev]
        ));
        if (msg.step) {
          setCurrentStep(msg.step);
        }
        setStages((prev) => applyPipelineEventToStages(prev, msg.step, msg.metadata ?? {}, msg.message));
        break;
      }
    }
  }, []);

  // -------------------------------------------------------------------------
  // Connection
  // -------------------------------------------------------------------------

  const connect = useCallback(async (): Promise<void> => {
    const id = sessionIdRef.current;
    if (!id || !mountedRef.current) return;

    // Close existing
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }

    const token = await getToken();
    const host = window.location.host;
    const url = `${WS_PROTOCOL}//${host}/api/sessions/${id}/socratic/ws${token ? `?token=${encodeURIComponent(token)}` : ''}`;

    let ws: WebSocket;
    try {
      ws = new WebSocket(url);
    } catch (err) {
      console.error('[Socratic WS] failed to construct WebSocket', err);
      return;
    }

    wsRef.current = ws;

    ws.onopen = (): void => {
      if (!mountedRef.current) return;
      setIsConnected(true);
      retryCountRef.current = 0;
      if (token) {
        ws.send(JSON.stringify({ type: 'auth', token }));
      }
      const capabilities = uiCapabilitiesRef.current;
      ws.send(JSON.stringify({
        type: 'ui_capabilities',
        ...capabilities,
      } satisfies ClientWsMessage));
      lastSentUiCapabilitiesRef.current = capabilities;
    };

    ws.onmessage = (event: MessageEvent): void => {
      if (!mountedRef.current) return;
      let msg: ServerWsMessage;
      try {
        msg = JSON.parse(event.data as string) as ServerWsMessage;
      } catch {
        console.warn('[Socratic WS] invalid JSON', event.data);
        return;
      }
      handleServerMessage(msg);
    };

    ws.onerror = (ev: Event): void => {
      console.error('[Socratic WS] error', ev);
    };

    ws.onclose = (): void => {
      if (!mountedRef.current) return;
      setIsConnected(false);
      wsRef.current = null;

      if (intakePhaseRef.current === 'complete' || intakePhaseRef.current === 'error') {
        clearRetryTimer();
        return;
      }

      if (retryCountRef.current < MAX_RETRIES) {
        const delay = BASE_DELAY_MS * Math.pow(2, retryCountRef.current);
        retryCountRef.current += 1;
        console.info(`[Socratic WS] reconnecting in ${delay}ms (attempt ${retryCountRef.current}/${MAX_RETRIES})`);
        retryTimerRef.current = setTimeout(() => {
          void connect();
        }, delay);
      } else {
        console.warn('[Socratic WS] max retries reached');
        setReconnectFailed(true);
      }
    };
  }, [getToken, handleServerMessage]);

  useEffect(() => {
    const pushUiCapabilitiesIfNeeded = (): void => {
      const nextCapabilities = buildUiCapabilities();
      if (sameUiCapabilities(uiCapabilitiesRef.current, nextCapabilities)) {
        return;
      }

      uiCapabilitiesRef.current = nextCapabilities;

      const ws = wsRef.current;
      if (ws?.readyState !== WebSocket.OPEN) {
        return;
      }

      if (sameUiCapabilities(lastSentUiCapabilitiesRef.current, nextCapabilities)) {
        return;
      }

      ws.send(JSON.stringify({
        type: 'ui_capabilities',
        ...nextCapabilities,
      } satisfies ClientWsMessage));
      lastSentUiCapabilitiesRef.current = nextCapabilities;
    };

    window.addEventListener('resize', pushUiCapabilitiesIfNeeded);
    return () => {
      window.removeEventListener('resize', pushUiCapabilitiesIfNeeded);
    };
  }, []);

  // -------------------------------------------------------------------------
  // Lifecycle
  // -------------------------------------------------------------------------

  useEffect(() => {
    mountedRef.current = true;
    retryCountRef.current = 0;
    hydratedSnapshotRef.current = null;
    uiCapabilitiesRef.current = buildUiCapabilities();
    lastSentUiCapabilitiesRef.current = null;

    // Reset all state when session changes
    setIsConnected(false);
    setReconnectFailed(false);
    setIntakePhase('waiting');
    setMessages([]);
    setClassification(null);
    setBeliefState(null);
    setConvergencePct(0);
    setCurrentCategorySnapshot(null);
    setCurrentWorkspace(null);
    setPendingCategoryId(null);
    setWorkspaceNotice(null);
    setCurrentPrompt(null);
    setSpeculativeDraft(null);
    setConfirmedSections(new Set());
    setStages(buildInitialStages());
    setPipelineComplete(false);
    setPipelineSummary(null);
    setEvents([]);
    setCurrentStep(null);

    // Don't auto-connect — the session page will trigger connect after
    // POST /socratic succeeds and we get the ws_url back.

    return (): void => {
      mountedRef.current = false;
      clearRetryTimer();
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [sessionId]);

  // Seed local websocket state from REST session snapshot.
  useEffect(() => {
    const signature = buildInitialSessionSignature(initialSession, sessionId);
    if (!initialSession || !sessionId) return;
    if (initialSession.id !== sessionId) return;
    if (signature === null || hydratedSnapshotRef.current === signature) return;

    const checkpoint = initialSession.checkpoint ?? null;
    const checkpointBeliefState = checkpoint?.belief_state ?? null;
    const checkpointPrompt = checkpoint?.current_prompt ?? null;
    const checkpointCategorySnapshot = checkpoint?.current_category_snapshot ?? null;

    const hydratedDraft: SpeculativeDraft | null = hydrateDraftFromPrompt(checkpointPrompt);

    const hydratedContradictions: Contradiction[] = (checkpoint?.contradictions ?? []).map(
      (entry) => ({
        dimension_a: normalizeDimensionLabel(entry.dimension_a),
        value_a: entry.value_a,
        dimension_b: normalizeDimensionLabel(entry.dimension_b),
        value_b: entry.value_b,
        explanation: entry.explanation,
      }),
    );

    setIntakePhase(initialSession.intake_phase ?? 'waiting');
    setMessages((initialSession.messages ?? []).filter((message) => message.role !== 'event'));
    setClassification(initialSession.classification ?? checkpoint?.classification ?? null);
    setBeliefState(initialSession.belief_state ?? checkpointBeliefState);
    setStages(initialSession.stages?.length ? initialSession.stages : buildInitialStages());
    setEvents(dedupeEventsById(initialSession.events ?? []));
    setCurrentStep(initialSession.current_step ?? null);
    setConvergencePct((initialSession.belief_state ?? checkpointBeliefState)?.convergence_pct ?? 0);
    setCurrentCategorySnapshot(checkpointCategorySnapshot);
    setCurrentWorkspace(null);
    setPendingCategoryId(null);
    setWorkspaceNotice(null);
    setCurrentPrompt(checkpointPrompt);
    setSpeculativeDraft(hydratedDraft);
    setConfirmedSections(new Set());
    setContradictions(hydratedContradictions);
    setPipelineComplete(initialSession.intake_phase === 'complete');
    setPipelineSummary(initialSession.intake_phase === 'complete' ? 'Pipeline finished' : null);

    hydratedSnapshotRef.current = signature;
  }, [initialSession, sessionId]);

  // -------------------------------------------------------------------------
  // Send helpers
  // -------------------------------------------------------------------------

  const sendRaw = useCallback((msg: ClientWsMessage): void => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    } else {
      console.warn('[Socratic WS] cannot send, socket not open');
    }
  }, []);

  /** Attach to an existing Socratic WebSocket session without restarting interview flow. */
  const attach = useCallback((): void => {
    void connect();
  }, [connect]);

  const sendPromptResponse = useCallback((promptId: string, answers: PromptAnswer[]): void => {
    const capabilities = uiCapabilitiesRef.current;
    sendRaw({
      type: 'prompt_response',
      prompt_id: promptId,
      answers,
      submitted_at: new Date().toISOString(),
      client_context: {
        viewport_class: capabilities.viewport_class,
      },
    });
  }, [sendRaw]);

  const submitPromptAnswers = useCallback((answers: PromptAnswer[]): void => {
    const prompt = currentPromptRef.current;
    if (!prompt) return;

    const normalizedAnswers = answers
      .map((answer) => ({
        item_id: answer.item_id,
        selected_option_id: answer.selected_option_id?.trim() || undefined,
        custom_text: answer.custom_text?.trim() || undefined,
        skipped: answer.skipped,
      }))
      .filter((answer) => answer.selected_option_id || answer.custom_text || answer.skipped);

    if (normalizedAnswers.length === 0) return;
    sendPromptResponse(prompt.prompt_id, normalizedAnswers);
  }, [sendPromptResponse]);

  const enterCategory = useCallback((categoryId: string, revision: string): void => {
    setPendingCategoryId(categoryId);
    setWorkspaceNotice(null);
    sendRaw({
      type: 'enter_category',
      category_id: categoryId,
      revision,
    });
  }, [sendRaw]);

  const backToCategories = useCallback((): void => {
    setPendingCategoryId(null);
    sendRaw({ type: 'back_to_categories' });
  }, [sendRaw]);

  /** Send the initial project description — this starts the interview. */
  const sendDescription = useCallback((description: string): void => {
    // Connect the WS first if not already connected
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      // Queue the description send for after connection
      const id = sessionIdRef.current;
      if (!id) return;

      void (async () => {
        await connect();
        // Wait for connection to open (up to 5s)
        const ws = wsRef.current;
        if (!ws) return;
        await new Promise<void>((resolve) => {
          if (ws.readyState === WebSocket.OPEN) { resolve(); return; }
          const origOnOpen = ws.onopen;
          ws.onopen = (ev) => {
            if (origOnOpen) (origOnOpen as (ev: Event) => void)(ev);
            resolve();
          };
          setTimeout(resolve, 5000);
        });
        sendPromptResponse('initial_description', [{
          item_id: 'initial_description',
          custom_text: description,
        }]);
        setIntakePhase('interviewing');
      })();
    } else {
      sendPromptResponse('initial_description', [{
        item_id: 'initial_description',
        custom_text: description,
      }]);
      setIntakePhase('interviewing');
    }
  }, [connect, sendPromptResponse]);

  /** Signal "done, start building." */
  const sendDone = useCallback((): void => {
    sendRaw({ type: 'done' });
  }, [sendRaw]);

  /** Send a dimension value edit from the belief state panel. */
  const sendDimensionEdit = useCallback((dimension: string, newValue: string): void => {
    sendRaw({ type: 'dimension_edit', dimension, new_value: newValue });
    setMessages((prev) => [...prev, {
      id: uuidv4(),
      role: 'user',
      content: `[Edit] ${dimension} → "${newValue}"`,
      timestamp: new Date().toISOString(),
    }]);
  }, [sendRaw]);

  return {
    isConnected,
    reconnectFailed,
    intakePhase,
    messages,
    classification,
    beliefState,
    convergencePct,
    currentCategorySnapshot,
    currentWorkspace,
    pendingCategoryId,
    workspaceNotice,
    currentPrompt,
    speculativeDraft,
    confirmedSections,
    contradictions,
    stages,
    pipelineComplete,
    pipelineSummary,
    events,
    currentStep,
    attach,
    sendDescription,
    submitPromptAnswers,
    enterCategory,
    backToCategories,
    sendDone,
    sendDimensionEdit,
  };
}
