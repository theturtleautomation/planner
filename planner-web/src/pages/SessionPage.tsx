import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import ChatPanel from '../components/ChatPanel.tsx';
import MessageInput from '../components/MessageInput.tsx';
import QuestionCanvas from '../components/QuestionCanvas.tsx';
import CategoryNavigator from '../components/CategoryNavigator.tsx';
import SocraticWorkspace from '../components/SocraticWorkspace.tsx';
import InterviewProgressPanel from '../components/InterviewProgressPanel.tsx';
import BeliefStatePanel from '../components/BeliefStatePanel.tsx';
import SpeculativeDraftView from '../components/SpeculativeDraftView.tsx';
import SessionEventsTable from '../components/SessionEventsTable.tsx';
import SessionStatusHeader from '../components/SessionStatusHeader.tsx';
import SessionPulseBar from '../components/SessionPulseBar.tsx';
import type { SessionHeaderAction } from '../components/SessionStatusHeader.tsx';
import {
  getSocraticDocumentGraphState,
  hydrateSocraticDocumentGraph,
  resetSocraticDocumentGraph,
  useSocraticDocumentKnownQuestionCount,
} from '../stores/socraticDocumentStore.ts';
import { buildKnowledgeDeepLink } from '../lib/knowledgeDeepLinks.ts';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import { useSocraticWebSocket } from '../hooks/useSocraticWebSocket.ts';
import type {
  InterviewCheckpoint,
  PlannerEvent,
  PromptEnvelope,
  ResumeStatus,
  Session,
  SessionExportResponse,
  SocraticCategorySnapshot,
  SocraticWorkspaceSnapshot,
} from '../types.ts';

function getInterviewResumeNotice(status: ResumeStatus):
  | { tone: 'warning' | 'info'; text: string }
  | null {
  switch (status) {
    case 'live_attach_available':
      return {
        tone: 'info',
        text: 'A live interview runtime is still available for this session. Reconnecting now…',
      };
    case 'interview_attached':
      return {
        tone: 'info',
        text: 'This interview is currently attached to a live websocket session.',
      };
    case 'interview_checkpoint_resumable':
      return {
        tone: 'info',
        text: 'This interview is resumable from a saved checkpoint. Reconnecting now…',
      };
    case 'interview_restart_only':
      return {
        tone: 'warning',
        text: 'Live interview resume is not supported yet. Restarting will begin from the saved brief.',
      };
    case 'interview_resume_unknown':
      return {
        tone: 'warning',
        text: 'Interview resume state is unknown for this session. It may require restart from the saved brief.',
      };
    default:
      return null;
  }
}

function formatCheckpointTimestamp(raw: string): string {
  const parsed = new Date(raw);
  if (Number.isNaN(parsed.getTime())) return raw;
  return parsed.toLocaleString();
}

function formatDimensionLabel(value: string | Record<string, unknown> | undefined): string {
  if (!value) return 'Unknown';
  if (typeof value === 'string') return value;
  const keys = Object.keys(value);
  if (keys.length === 1) {
    const inner = value[keys[0]];
    if (typeof inner === 'string') return inner;
  }
  return JSON.stringify(value);
}

function getCheckpointSummary(checkpoint: InterviewCheckpoint): string[] {
  const lines: string[] = [];
  const prompt = checkpoint.current_prompt;
  if (prompt?.items?.length) {
    if (prompt.kind === 'draft_review') {
      const heading = prompt.draft_snapshot?.sections?.[0]?.heading;
      if (heading) {
        lines.push(`Pending draft review: ${heading}`);
      } else {
        lines.push('Pending draft review is available.');
      }
    } else {
      lines.push(`Current prompt: ${prompt.items[0].text}`);
    }
  }
  if (checkpoint.contradictions?.length) {
    const unresolved = checkpoint.contradictions.filter((c) => !c.resolved).length;
    if (unresolved > 0) {
      lines.push(`${unresolved} unresolved contradiction${unresolved === 1 ? '' : 's'}.`);
    }
  }
  return lines;
}

function getCheckpointTargetDimension(checkpoint: InterviewCheckpoint): string | null {
  const target = checkpoint.current_prompt?.items?.find((item) => item.target_dimension)?.target_dimension;
  return target ? formatDimensionLabel(target) : null;
}

function getPromptFocusCategoryId(prompt: PromptEnvelope): string | null {
  return prompt.origin_category_id
    ?? prompt.category_path[prompt.category_path.length - 1]?.category_id
    ?? null;
}

function buildFallbackCategorySnapshot(prompt: PromptEnvelope): SocraticCategorySnapshot {
  const focusCategoryId = getPromptFocusCategoryId(prompt) ?? prompt.prompt_id;
  const focusTitle = prompt.category_path[prompt.category_path.length - 1]?.title ?? prompt.title;
  const rootCategoryId = prompt.category_path[0]?.category_id ?? focusCategoryId;

  return {
    revision: `hydrated-${prompt.prompt_id}`,
    root_category_ids: rootCategoryId ? [rootCategoryId] : [],
    nodes: [
      {
        category_id: focusCategoryId,
        parent_category_id: null,
        title: focusTitle,
        summary: prompt.instructions?.trim() || 'Current questions are ready.',
        status: 'active',
        depth: prompt.category_path.length > 0 ? prompt.category_path.length - 1 : 0,
        mapped_dimensions: [],
        has_children: false,
        has_prompt_ready: true,
        item_count_hint: Math.max(prompt.items.length, 1),
      },
    ],
    active_category_path: prompt.category_path,
    newly_available_category_ids: [],
    build_ready: false,
    build_readiness_message: 'Planning questions are still in progress.',
  };
}

function buildHydratedWorkspace(
  workspace: SocraticWorkspaceSnapshot | null,
  prompt: PromptEnvelope | null,
  categorySnapshot: SocraticCategorySnapshot | null,
): SocraticWorkspaceSnapshot | null {
  if (workspace) return workspace;
  if (!prompt && !categorySnapshot) return null;

  const nextCategorySnapshot = categorySnapshot ?? buildFallbackCategorySnapshot(prompt!);
  const activePathCategoryId =
    nextCategorySnapshot.active_category_path[nextCategorySnapshot.active_category_path.length - 1]?.category_id
    ?? null;
  const activePathNode = activePathCategoryId
    ? nextCategorySnapshot.nodes.find((node) => node.category_id === activePathCategoryId) ?? null
    : null;
  const visibleNodes = activePathCategoryId
    ? nextCategorySnapshot.nodes.filter((node) => node.parent_category_id === activePathCategoryId)
    : nextCategorySnapshot.root_category_ids
      .map((categoryId) => nextCategorySnapshot.nodes.find((node) => node.category_id === categoryId) ?? null)
      .filter((node): node is NonNullable<typeof node> => node !== null);

  if (prompt) {
    const focusCategoryId =
      getPromptFocusCategoryId(prompt)
      ?? nextCategorySnapshot.root_category_ids[0]
      ?? prompt.prompt_id;
    const focusedNode = nextCategorySnapshot.nodes.find((node) => node.category_id === focusCategoryId);

    return {
      focused_category_id: focusCategoryId,
      branch_notice: null,
      category_snapshot: prompt.category_path.length > 0
        ? {
            ...nextCategorySnapshot,
            active_category_path: prompt.category_path,
          }
        : nextCategorySnapshot,
      groups: [
        {
          category_id: focusCategoryId,
          title: focusedNode?.title ?? prompt.category_path[prompt.category_path.length - 1]?.title ?? prompt.title,
          summary: focusedNode?.summary ?? prompt.instructions?.trim() ?? 'Answer the current questions to keep planning moving.',
          status: 'active',
          question_count: Math.max(prompt.items.length, focusedNode?.item_count_hint ?? 0, 1),
          is_focused: true,
          is_new: false,
          preview_items: prompt.items.slice(0, 3).map((item) => ({
            item_id: item.item_id,
            kind: item.kind,
            text: item.text,
          })),
        },
      ],
    };
  }

  return {
    focused_category_id: activePathCategoryId,
    branch_notice: null,
    category_snapshot: nextCategorySnapshot,
    groups: (
      activePathNode && (activePathNode.has_prompt_ready || visibleNodes.length === 0)
        ? [activePathNode]
        : visibleNodes
    ).map((node) => ({
      category_id: node.category_id,
      title: node.title,
      summary: node.summary,
      status: node.status,
      question_count: Math.max(node.item_count_hint, node.has_prompt_ready ? 1 : 0),
      is_focused: node.category_id === activePathCategoryId,
      is_new: nextCategorySnapshot.newly_available_category_ids.includes(node.category_id),
      preview_items: [],
    })),
  };
}

function getSessionTitle(
  session: Pick<Session, 'title' | 'project_description' | 'id'>,
): string {
  const explicit = session.title?.trim();
  if (explicit) return explicit;

  const description = session.project_description?.trim();
  if (description) {
    const singleLine = description.replace(/\s+/g, ' ').trim();
    return singleLine.length > 72 ? `${singleLine.slice(0, 72)}…` : singleLine;
  }

  return `Session ${session.id.slice(0, 8)}`;
}

function makeExportFilename(session: Session): string {
  const slug = getSessionTitle(session)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/(^-|-$)/g, '')
    .slice(0, 48) || session.id.slice(0, 8);
  return `${slug}-session-export.json`;
}

function downloadExport(payload: SessionExportResponse): void {
  const contents = `${JSON.stringify(payload, null, 2)}\n`;
  const blob = new Blob([contents], { type: 'application/json' });
  const href = window.URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = href;
  link.download = makeExportFilename(payload.session);
  document.body.appendChild(link);
  link.click();
  link.remove();
  window.URL.revokeObjectURL(href);
}

const FIRST_REVEAL_PRELOAD_TARGET = 8;
const FIRST_REVEAL_SOFT_TARGET_MS = 4_000;
const FIRST_REVEAL_HARD_TIMEOUT_MS = 8_000;

function dedupePlannerEvents(events: PlannerEvent[]): PlannerEvent[] {
  const seen = new Set<string>();
  const deduped: PlannerEvent[] = [];
  for (const event of events) {
    if (seen.has(event.id)) continue;
    seen.add(event.id);
    deduped.push(event);
  }
  return deduped;
}

interface RetryFeedbackSummary {
  feedbackCount: number;
  categories: Record<string, number>;
  severities: Record<string, number>;
  attempt: number | null;
}

interface ArtifactProgressSummary {
  totalPersisted: number;
  latestTypeId: string | null;
  byType: Record<string, number>;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function asNumber(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  return null;
}

function asString(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value : null;
}

function getEventDetail(event: PlannerEvent, key: string): unknown {
  if (Object.prototype.hasOwnProperty.call(event.metadata, key)) {
    return event.metadata[key];
  }
  const details = asRecord(event.metadata.details);
  if (details && Object.prototype.hasOwnProperty.call(details, key)) {
    return details[key];
  }
  return undefined;
}

function summarizeRetryFeedback(events: PlannerEvent[]): RetryFeedbackSummary | null {
  const latest = events.find((event) => event.step === 'pipeline.retry.feedback');
  if (!latest) return null;

  const categories = asRecord(getEventDetail(latest, 'categories')) ?? {};
  const severities = asRecord(getEventDetail(latest, 'severities')) ?? {};

  const categoryCounts = Object.fromEntries(
    Object.entries(categories).map(([key, value]) => [key, asNumber(value) ?? 0]),
  );
  const severityCounts = Object.fromEntries(
    Object.entries(severities).map(([key, value]) => [key, asNumber(value) ?? 0]),
  );

  return {
    feedbackCount: asNumber(getEventDetail(latest, 'feedback_count')) ?? 0,
    categories: categoryCounts,
    severities: severityCounts,
    attempt: asNumber(getEventDetail(latest, 'attempt')),
  };
}

function summarizeArtifactProgress(events: PlannerEvent[]): ArtifactProgressSummary | null {
  const artifactEvents = events.filter((event) => event.step === 'pipeline.artifact.persisted');
  if (artifactEvents.length === 0) return null;

  const byType: Record<string, number> = {};
  for (const event of artifactEvents) {
    const typeId = asString(getEventDetail(event, 'type_id'));
    if (!typeId) continue;
    byType[typeId] = (byType[typeId] ?? 0) + 1;
  }

  const latestTypeId = asString(getEventDetail(artifactEvents[0], 'type_id'));

  return {
    totalPersisted: artifactEvents.length,
    latestTypeId,
    byType,
  };
}

export default function SessionPage() {
  const { id: routeId } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const getToken = useGetAccessToken();

  const api = useMemo(() => createApiClient(getToken), [getToken]);
  const isExistingSessionRoute = Boolean(routeId && routeId !== 'new');
  const projectRef = searchParams.get('project')?.trim() || undefined;

  // Core session state
  const [session, setSession] = useState<Session | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(
    routeId && routeId !== 'new' ? routeId : null,
  );
  const [initError, setInitError] = useState<string | null>(null);

  // Waiting phase state
  const [description, setDescription] = useState('');
  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const [workflowError, setWorkflowError] = useState<string | null>(null);
  const [workflowAction, setWorkflowAction] = useState<
    'restart' | 'retry' | 'rename' | 'duplicate' | 'archive' | 'export' | null
  >(null);
  const [firstRevealGateArmed, setFirstRevealGateArmed] = useState(false);
  const [hasRevealedFirstLobby, setHasRevealedFirstLobby] = useState(false);
  const [firstRevealUsedTimeoutFallback, setFirstRevealUsedTimeoutFallback] = useState(false);
  const [firstRevealGateStartedAtMs, setFirstRevealGateStartedAtMs] = useState<number | null>(null);
  const [firstRevealElapsedMs, setFirstRevealElapsedMs] = useState(0);

  // Context shelf tab: 'belief' | 'draft' | 'events' | 'transcript'
  type RightPanelTab = 'belief' | 'draft' | 'events' | 'transcript';
  const [rightTab, setRightTab] = useState<RightPanelTab>('belief');
  const [eventUnreadCount, setEventUnreadCount] = useState(0);
  const [contextShelfOpen, setContextShelfOpen] = useState(false);

  // Helper to switch to draft tab
  const setShowDraft = (v: boolean) => setRightTab(v ? 'draft' : 'belief');

  // Socratic WebSocket hook
  const socratic = useSocraticWebSocket({ sessionId, getToken, initialSession: session });
  const knownDocumentQuestionCount = useSocraticDocumentKnownQuestionCount();

  // Auto-show draft when it arrives
  useEffect(() => {
    if (socratic.speculativeDraft) {
      setShowDraft(true);
    }
  }, [socratic.speculativeDraft]);

  const eventCounts = useMemo(() => {
    return socratic.events.reduce(
      (acc, event) => {
        if (event.level === 'error') acc.errors += 1;
        if (event.level === 'warn') acc.warnings += 1;
        return acc;
      },
      { total: socratic.events.length, errors: 0, warnings: 0 },
    );
  }, [socratic.events]);

  const retryFeedbackSummary = useMemo(
    () => summarizeRetryFeedback(socratic.events),
    [socratic.events],
  );
  const artifactProgressSummary = useMemo(
    () => summarizeArtifactProgress(socratic.events),
    [socratic.events],
  );

  const previousEventCountRef = useRef(0);
  const autoForegroundEventsRef = useRef<string | null>(null);
  useEffect(() => {
    const nextCount = socratic.events.length;
    const delta = nextCount - previousEventCountRef.current;

    if (rightTab === 'events') {
      setEventUnreadCount(0);
    } else if (delta > 0) {
      setEventUnreadCount((previous) => previous + delta);
    }

    previousEventCountRef.current = nextCount;
  }, [rightTab, socratic.events.length]);

  useEffect(() => {
    previousEventCountRef.current = 0;
    setEventUnreadCount(0);
    autoForegroundEventsRef.current = null;
    setContextShelfOpen(false);
  }, [sessionId]);

  // Track whether we've triggered attach for an existing session
  const autoAttachAttemptedRef = useRef<string | null>(null);

  // ── Init: create or load session ──
  useEffect(() => {
    let cancelled = false;
    autoAttachAttemptedRef.current = null;

    const init = async (): Promise<void> => {
      try {
        if (!routeId || routeId === 'new') {
          // Create a new session — don't auto-connect WS
          const resp = await api.createSession(projectRef ? { projectRef } : undefined);
          if (cancelled) return;
          const s = resp.session ?? resp;
          setSession(s as Session);
          setSessionId((s as Session).id);
          setDescription((s as Session).project_description ?? '');
          void navigate(`/session/${(s as Session).id}`, { replace: true });
        } else {
          // Load existing session
          const resp = await api.getSession(routeId);
          if (cancelled) return;
          const s = resp.session ?? resp;
          setSession(s as Session);
          setSessionId((s as Session).id);
          setDescription((s as Session).project_description ?? '');
        }
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : String(err);
        console.error('[SessionPage] init error:', msg);
        setInitError(msg);
      }
    };

    void init();
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [routeId, projectRef]);

  useEffect(() => {
    if (!sessionId) return;
    let cancelled = false;

    const hydrateEvents = async (): Promise<void> => {
      try {
        const response = await api.getSessionEvents(sessionId, { limit: 500 });
        if (cancelled) return;
        setSession((previous) => {
          if (!previous || previous.id !== sessionId) return previous;
          return {
            ...previous,
            events: dedupePlannerEvents(response.events ?? []),
          };
        });
      } catch {
        // Keep the lobby usable even if event hydration fails.
      }
    };

    void hydrateEvents();
    return () => {
      cancelled = true;
    };
  }, [api, sessionId]);

  // Auto-attach WS for existing sessions that should resume in read-only mode.
  useEffect(() => {
    if (!session || !sessionId) return;
    if (autoAttachAttemptedRef.current === sessionId) return;
    if (session.can_resume_live || session.can_resume_checkpoint) {
      autoAttachAttemptedRef.current = sessionId;
      socratic.attach();
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session, sessionId]);

  // ── Description submission ──
  const handleStartInterview = useCallback(async (): Promise<void> => {
    if (!sessionId || !description.trim()) return;
    setIsStarting(true);
    setStartError(null);
    setWorkflowError(null);
    try {
      // 1. Create the server-side Socratic session
      await api.startSocratic(sessionId, description.trim());
      setSession((previous) => (
        previous
          ? {
              ...previous,
              intake_phase: 'interviewing',
              pipeline_running: false,
              error_message: null,
              project_description: description.trim(),
            }
          : previous
      ));
      setFirstRevealGateArmed(true);
      setHasRevealedFirstLobby(false);
      setFirstRevealUsedTimeoutFallback(false);
      setFirstRevealGateStartedAtMs(null);
      setFirstRevealElapsedMs(0);
      // 2. Connect WS and send initial description
      socratic.sendDescription(description.trim());
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[SessionPage] startSocratic error:', msg);
      setStartError('Failed to start interview. Please try again.');
      setTimeout(() => setStartError(null), 6000);
    } finally {
      setIsStarting(false);
    }
  }, [sessionId, description, api, socratic]);

  const applySessionSnapshot = useCallback((nextSession: Session): void => {
    autoAttachAttemptedRef.current = null;
    setSession(nextSession);
    setDescription(nextSession.project_description ?? '');
    setStartError(null);
    setWorkflowError(null);
    setRightTab('belief');
  }, []);

  const handleResume = useCallback((): void => {
    setWorkflowError(null);
    socratic.attach();
  }, [socratic]);

  const handleRenameSession = useCallback(async (): Promise<void> => {
    if (!sessionId || !session) return;
    const currentTitle = getSessionTitle(session);
    const nextTitle = window.prompt('Rename session', currentTitle);
    if (nextTitle === null) return;

    const trimmed = nextTitle.trim();
    if (!trimmed || trimmed === currentTitle) return;

    setWorkflowAction('rename');
    setWorkflowError(null);
    try {
      const resp = await api.updateSession(sessionId, { title: trimmed });
      applySessionSnapshot(resp.session);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[SessionPage] renameSession error:', msg);
      setWorkflowError('Failed to rename the session. Please try again.');
    } finally {
      setWorkflowAction(null);
    }
  }, [api, applySessionSnapshot, session, sessionId]);

  const handleDuplicateSession = useCallback(async (): Promise<void> => {
    if (!sessionId || !session) return;
    const suggestedTitle = `${getSessionTitle(session)} (Copy)`;
    const requestedTitle = window.prompt('Name the duplicate session', suggestedTitle);
    if (requestedTitle === null) return;

    setWorkflowAction('duplicate');
    setWorkflowError(null);
    try {
      const trimmed = requestedTitle.trim();
      const resp = await api.duplicateSession(
        sessionId,
        trimmed ? { title: trimmed } : undefined,
      );
      void navigate(`/session/${resp.session.id}`);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[SessionPage] duplicateSession error:', msg);
      setWorkflowError('Failed to duplicate the session. Please try again.');
    } finally {
      setWorkflowAction(null);
    }
  }, [api, navigate, session, sessionId]);

  const handleArchiveToggle = useCallback(async (): Promise<void> => {
    if (!sessionId || !session) return;
    if (!session.archived) {
      const confirmed = window.confirm(`Archive "${getSessionTitle(session)}"?`);
      if (!confirmed) return;
    }

    setWorkflowAction('archive');
    setWorkflowError(null);
    try {
      const resp = await api.updateSession(sessionId, { archived: !session.archived });
      applySessionSnapshot(resp.session);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[SessionPage] archiveSession error:', msg);
      setWorkflowError(
        session.archived
          ? 'Failed to restore the session. Please try again.'
          : 'Failed to archive the session. Please try again.',
      );
    } finally {
      setWorkflowAction(null);
    }
  }, [api, applySessionSnapshot, session, sessionId]);

  const handleExportSession = useCallback(async (): Promise<void> => {
    if (!sessionId) return;
    setWorkflowAction('export');
    setWorkflowError(null);
    try {
      const payload = await api.exportSession(sessionId);
      downloadExport(payload);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[SessionPage] exportSession error:', msg);
      setWorkflowError('Failed to export the session history. Please try again.');
    } finally {
      setWorkflowAction(null);
    }
  }, [api, sessionId]);

  const handleRestartFromDescription = useCallback(async (): Promise<void> => {
    if (!sessionId) return;
    setWorkflowAction('restart');
    setWorkflowError(null);

    try {
      const resp = await api.restartFromDescription(sessionId);
      const nextSession = resp.session;
      const savedDescription = nextSession.project_description?.trim();
      if (!savedDescription) {
        throw new Error('Saved planning brief is unavailable for this session.');
      }
      applySessionSnapshot(nextSession);
      setFirstRevealGateArmed(true);
      setHasRevealedFirstLobby(false);
      setFirstRevealUsedTimeoutFallback(false);
      setFirstRevealGateStartedAtMs(null);
      setFirstRevealElapsedMs(0);
      socratic.sendDescription(savedDescription);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[SessionPage] restartFromDescription error:', msg);
      setWorkflowError('Failed to restart from the saved brief. Please try again.');
    } finally {
      setWorkflowAction(null);
    }
  }, [sessionId, api, applySessionSnapshot, socratic]);

  const handleRetryPipeline = useCallback(async (): Promise<void> => {
    if (!sessionId) return;
    setWorkflowAction('retry');
    setWorkflowError(null);

    try {
      const resp = await api.retryPipeline(sessionId);
      applySessionSnapshot(resp.session);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[SessionPage] retryPipeline error:', msg);
      setWorkflowError('Failed to retry the pipeline. Please try again.');
    } finally {
      setWorkflowAction(null);
    }
  }, [sessionId, api, applySessionSnapshot]);

  // Textarea auto-grow ref
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = 'auto';
      el.style.height = Math.min(el.scrollHeight, 300) + 'px';
    }
  }, [description]);

  const checkpointPrompt = session?.checkpoint?.current_prompt ?? null;
  const checkpointCategorySnapshot = session?.checkpoint?.current_category_snapshot ?? null;
  const displayPrompt = socratic.currentPrompt ?? checkpointPrompt;
  const displayCategorySnapshot = socratic.currentCategorySnapshot ?? checkpointCategorySnapshot;
  const displayWorkspace = useMemo(
    () => buildHydratedWorkspace(socratic.currentWorkspace, displayPrompt, displayCategorySnapshot),
    [socratic.currentWorkspace, displayPrompt, displayCategorySnapshot],
  );
  const hasResumableInterviewState = Boolean(displayWorkspace || displayPrompt || displayCategorySnapshot);

  useEffect(() => {
    if (!displayWorkspace) return;
    hydrateSocraticDocumentGraph({
      workspace: displayWorkspace,
      currentPrompt: displayPrompt,
    });
  }, [displayWorkspace, displayPrompt]);

  useEffect(() => {
    resetSocraticDocumentGraph();
    setFirstRevealGateArmed(false);
    setHasRevealedFirstLobby(false);
    setFirstRevealUsedTimeoutFallback(false);
    setFirstRevealGateStartedAtMs(null);
    setFirstRevealElapsedMs(0);
  }, [sessionId]);

  // ── Effective intake phase ──
  // Use the WS hook's phase once connected; fall back to the session's stored phase
  const sessionPhase = session?.intake_phase ?? 'waiting';
  const effectivePhase = socratic.intakePhase !== 'waiting'
    ? socratic.intakePhase
    : (sessionPhase === 'waiting' && hasResumableInterviewState ? 'interviewing' : sessionPhase);

  useEffect(() => {
    if (effectivePhase === 'waiting') {
      setFirstRevealGateArmed(false);
      setHasRevealedFirstLobby(false);
      setFirstRevealUsedTimeoutFallback(false);
      setFirstRevealGateStartedAtMs(null);
      setFirstRevealElapsedMs(0);
    }
  }, [effectivePhase]);

  useEffect(() => {
    if (!sessionId) return;
    if (effectivePhase !== 'pipeline_running') return;
    if (autoForegroundEventsRef.current === sessionId) return;
    setRightTab('events');
    setEventUnreadCount(0);
    autoForegroundEventsRef.current = sessionId;
  }, [effectivePhase, sessionId]);

  const isInterviewing = effectivePhase === 'interviewing';
  const isPipelineRunning = effectivePhase === 'pipeline_running';
  const isComplete = effectivePhase === 'complete';
  const isError = effectivePhase === 'error';
  const showFocusedLobby = isInterviewing;
  const buildReady = Boolean(displayWorkspace?.category_snapshot.build_ready);
  const knownQuestionIds = useMemo(
    () => new Set(Object.keys(getSocraticDocumentGraphState().questionsById)),
    [knownDocumentQuestionCount],
  );
  const currentPromptOverflowCount = useMemo(() => {
    if (!displayPrompt) return 0;
    let overflow = 0;
    for (const item of displayPrompt.items) {
      if (!knownQuestionIds.has(item.item_id)) {
        overflow += 1;
      }
    }
    return overflow;
  }, [displayPrompt, knownQuestionIds]);
  const previewOnlyCount = useMemo(() => {
    if (!displayWorkspace) return 0;
    const previewIds = new Set<string>();
    for (const group of displayWorkspace.groups) {
      for (const item of group.preview_items) {
        if (!knownQuestionIds.has(item.item_id)) {
          previewIds.add(item.item_id);
        }
      }
    }
    return previewIds.size;
  }, [displayWorkspace, knownQuestionIds]);
  const knownQuestionCount = knownDocumentQuestionCount + currentPromptOverflowCount + previewOnlyCount;
  const canRevealBestKnownLobby = Boolean(displayWorkspace);
  const showFirstRevealPreload = showFocusedLobby && firstRevealGateArmed && !hasRevealedFirstLobby;
  const firstRevealSoftTargetReached = firstRevealElapsedMs >= FIRST_REVEAL_SOFT_TARGET_MS;
  const firstRevealHardTimeoutReached = firstRevealElapsedMs >= FIRST_REVEAL_HARD_TIMEOUT_MS;

  useEffect(() => {
    if (!showFirstRevealPreload) {
      setFirstRevealGateStartedAtMs(null);
      setFirstRevealElapsedMs(0);
      return;
    }
    if (firstRevealGateStartedAtMs !== null) return;
    const now = Date.now();
    setFirstRevealGateStartedAtMs(now);
    setFirstRevealElapsedMs(0);
  }, [firstRevealGateStartedAtMs, showFirstRevealPreload]);

  useEffect(() => {
    if (!showFirstRevealPreload || firstRevealGateStartedAtMs === null) return;
    const tick = () => {
      setFirstRevealElapsedMs(Date.now() - firstRevealGateStartedAtMs);
    };
    tick();
    const intervalId = window.setInterval(tick, 250);
    return () => window.clearInterval(intervalId);
  }, [firstRevealGateStartedAtMs, showFirstRevealPreload]);

  useEffect(() => {
    if (!showFirstRevealPreload) return;
    if (buildReady || knownQuestionCount >= FIRST_REVEAL_PRELOAD_TARGET) {
      setHasRevealedFirstLobby(true);
      setFirstRevealUsedTimeoutFallback(false);
      return;
    }
    if (firstRevealHardTimeoutReached && canRevealBestKnownLobby) {
      setHasRevealedFirstLobby(true);
      setFirstRevealUsedTimeoutFallback(true);
    }
  }, [
    buildReady,
    canRevealBestKnownLobby,
    firstRevealHardTimeoutReached,
    knownQuestionCount,
    showFirstRevealPreload,
  ]);

  const sessionActions = useMemo<SessionHeaderAction[]>(() => {
    const actions: SessionHeaderAction[] = [];
    const projectBackPath = session?.project_slug
      ? `/projects/${encodeURIComponent(session.project_slug)}/sessions`
      : null;
    const knowledgePath = (session?.project_id && sessionId)
      ? buildKnowledgeDeepLink({
          projectId: session.project_id,
          originPath: `/session/${encodeURIComponent(sessionId)}`,
          originLabel: 'Session',
        })
      : null;

    if (session && !socratic.isConnected && (session.can_resume_live || session.can_resume_checkpoint)) {
      actions.push({
        key: 'resume',
        label: session.can_resume_checkpoint ? 'Resume Checkpoint' : 'Reconnect Live',
        onClick: handleResume,
        tone: 'primary',
      });
    }

    if (session) {
      actions.push({
        key: 'rename-session',
        label: workflowAction === 'rename' ? 'Renaming…' : 'Rename',
        onClick: () => { void handleRenameSession(); },
        disabled: workflowAction !== null || isStarting,
      });

      actions.push({
        key: 'duplicate-session',
        label: workflowAction === 'duplicate' ? 'Duplicating…' : 'Duplicate',
        onClick: () => { void handleDuplicateSession(); },
        disabled: workflowAction !== null || isStarting,
      });

      actions.push({
        key: session.archived ? 'unarchive-session' : 'archive-session',
        label: session.archived
          ? workflowAction === 'archive'
            ? 'Restoring…'
            : 'Unarchive'
          : workflowAction === 'archive'
            ? 'Archiving…'
            : 'Archive',
        onClick: () => { void handleArchiveToggle(); },
        disabled: workflowAction !== null
          || isStarting
          || (!session.archived && (session.intake_phase === 'interviewing' || session.pipeline_running)),
        tone: session.archived ? 'default' : 'danger',
      });

      actions.push({
        key: 'export-session',
        label: workflowAction === 'export' ? 'Exporting…' : 'Export',
        onClick: () => { void handleExportSession(); },
        disabled: workflowAction !== null || isStarting,
      });
    }

    if (session?.can_restart_from_description) {
      actions.push({
        key: 'restart',
        label: workflowAction === 'restart' ? 'Restarting…' : 'Restart from Description',
        onClick: () => { void handleRestartFromDescription(); },
        disabled: workflowAction !== null || isStarting,
      });
    }

    if (session?.can_retry_pipeline) {
      actions.push({
        key: 'retry',
        label: workflowAction === 'retry' ? 'Retrying…' : 'Retry Pipeline',
        onClick: () => { void handleRetryPipeline(); },
        disabled: workflowAction !== null || isStarting,
      });
    }

    if (sessionId) {
      if (knowledgePath) {
        actions.push({
          key: 'knowledge',
          label: 'Knowledge',
          onClick: () => { void navigate(knowledgePath); },
        });
      }

      actions.push({
        key: 'back',
        label: projectBackPath ? 'Back to Project' : 'Back to Sessions',
        onClick: () => { void navigate(projectBackPath ?? '/sessions'); },
      });
    }

    return actions;
  }, [
    handleArchiveToggle,
    handleDuplicateSession,
    handleExportSession,
    handleRenameSession,
    handleResume,
    handleRestartFromDescription,
    handleRetryPipeline,
    isStarting,
    sessionId,
    navigate,
    session,
    socratic.isConnected,
    workflowAction,
  ]);

  // ── Right panel content ──
  const rightPanelContent = (() => {
    if (rightTab === 'transcript') {
      return <ChatPanel messages={socratic.messages} />;
    }

    if (rightTab === 'events') {
      const retryCategorySummary = retryFeedbackSummary
        ? Object.entries(retryFeedbackSummary.categories)
          .filter(([, count]) => count > 0)
          .map(([category, count]) => `${category}: ${count}`)
          .join(' • ')
        : '';
      const retrySeveritySummary = retryFeedbackSummary
        ? Object.entries(retryFeedbackSummary.severities)
          .filter(([, count]) => count > 0)
          .map(([severity, count]) => `${severity}: ${count}`)
          .join(' • ')
        : '';
      const artifactTypeSummary = artifactProgressSummary
        ? Object.entries(artifactProgressSummary.byType)
          .filter(([, count]) => count > 0)
          .slice(0, 4)
          .map(([typeId, count]) => `${typeId} (${count})`)
          .join(' • ')
        : '';

      return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', height: '100%', overflow: 'hidden' }}>
          {(retryFeedbackSummary || artifactProgressSummary) && (
            <div style={{ display: 'grid', gap: '8px', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', padding: '10px 12px 0' }}>
              {retryFeedbackSummary && (
                <section
                  aria-label="Retry feedback summary"
                  style={{
                    borderRadius: '14px',
                    padding: '12px 14px',
                    background: 'var(--color-surface-offset)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '6px',
                  }}
                >
                  <span style={{ fontSize: '10px', color: 'var(--color-primary)', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase' }}>
                    Retry Feedback
                  </span>
                  <span style={{ fontSize: '12px', color: 'var(--color-text)' }}>
                    {retryFeedbackSummary.feedbackCount} categorized item{retryFeedbackSummary.feedbackCount === 1 ? '' : 's'}
                    {retryFeedbackSummary.attempt ? ` (attempt ${retryFeedbackSummary.attempt})` : ''}
                  </span>
                  {retryCategorySummary && (
                    <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
                      Categories: {retryCategorySummary}
                    </span>
                  )}
                  {retrySeveritySummary && (
                    <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
                      Severities: {retrySeveritySummary}
                    </span>
                  )}
                </section>
              )}
              {artifactProgressSummary && (
                <section
                  aria-label="Artifact persistence summary"
                  style={{
                    borderRadius: '14px',
                    padding: '12px 14px',
                    background: 'var(--color-surface-offset)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '6px',
                  }}
                >
                  <span style={{ fontSize: '10px', color: 'var(--color-primary)', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase' }}>
                    Artifact Progress
                  </span>
                  <span style={{ fontSize: '12px', color: 'var(--color-text)' }}>
                    {artifactProgressSummary.totalPersisted} artifact{artifactProgressSummary.totalPersisted === 1 ? '' : 's'} persisted
                  </span>
                  {artifactProgressSummary.latestTypeId && (
                    <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
                      Latest: {artifactProgressSummary.latestTypeId}
                    </span>
                  )}
                  {artifactTypeSummary && (
                    <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
                      By type: {artifactTypeSummary}
                    </span>
                  )}
                </section>
              )}
            </div>
          )}

          <div style={{ minHeight: 0, flex: 1 }}>
            <SessionEventsTable events={socratic.events} />
          </div>
        </div>
      );
    }
    if (rightTab === 'draft' && socratic.speculativeDraft) {
      return (
                <SpeculativeDraftView
          draft={socratic.speculativeDraft}
          onBack={() => setRightTab('belief')}
        />
      );
    }
    return (
      <BeliefStatePanel
        beliefState={socratic.beliefState}
        classification={socratic.classification}
        contradictions={socratic.contradictions}
        onDimensionEdit={socratic.sendDimensionEdit}
      />
    );
  })();

  // ── Error state ──
  if (initError) {
    const is404 = initError.includes('404');
    const fallbackPath = session?.project_slug
      ? `/projects/${encodeURIComponent(session.project_slug)}/sessions`
      : '/sessions';
    const fallbackLabel = session?.project_slug ? 'project' : 'sessions';
    return (
      <Layout>
        <div style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          padding: '24px',
        }}>
          <span style={{ color: 'var(--color-error)', fontSize: '14px' }}>[ ERROR ]</span>
          <span style={{ color: 'var(--color-text-muted)', fontSize: '13px', textAlign: 'center', maxWidth: '500px' }}>
            {is404 ? 'Session not found.' : initError}
          </span>
          <button
            onClick={() => void navigate(fallbackPath)}
            style={{
              marginTop: '8px',
              background: 'var(--color-surface)',
              boxShadow: 'inset 0 0 0 1px var(--color-divider)',
              color: 'var(--color-text-muted)',
              padding: '7px 16px',
              fontSize: '12px',
              cursor: 'pointer',
              fontFamily: 'inherit',
              borderRadius: '8px',
            }}
          >
            {`← back to ${fallbackLabel}`}
          </button>
        </div>
      </Layout>
    );
  }

  // ── Loading state ──
  if ((!sessionId || (isExistingSessionRoute && !session)) && !initError) {
    return (
      <Layout>
        <div style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--color-text-muted)',
          fontSize: '13px',
        }}>
          loading session…
        </div>
      </Layout>
    );
  }

  // ── Reconnect failure banner ──
  const reconnectBanner = socratic.reconnectFailed && (
    <div style={{
      padding: '8px 16px',
      background: 'color-mix(in srgb, var(--color-error-highlight) 72%, transparent)',
      color: 'var(--color-error)',
      fontSize: '12px',
      textAlign: 'center',
      flexShrink: 0,
    }}>
      Connection lost. Please refresh to reconnect.
    </div>
  );

  const workflowErrorBanner = workflowError && (
    <div style={{
      padding: '8px 16px',
      background: 'color-mix(in srgb, var(--color-error-highlight) 72%, transparent)',
      color: 'var(--color-error)',
      fontSize: '12px',
      textAlign: 'center',
      flexShrink: 0,
    }}>
      {workflowError}
    </div>
  );

  // ─────────────────────────────────────────────────────────────────
  // PHASE: waiting — show description form
  // ─────────────────────────────────────────────────────────────────
  if (effectivePhase === 'waiting') {
    return (
      <Layout sessionId={sessionId} isConnected={false}>
        <div style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}>
          {reconnectBanner}
          {workflowErrorBanner}
          {sessionActions.length > 0 && (
            <SessionStatusHeader
              currentStep={null}
              events={[]}
              isError={false}
              actions={sessionActions}
            />
          )}
          <div style={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '32px',
            overflow: 'auto',
          }}>
            <div style={{
              width: '100%',
              maxWidth: '600px',
              background: 'var(--color-surface)',
              borderRadius: '20px',
              padding: '32px 36px',
              display: 'flex',
              flexDirection: 'column',
              gap: '20px',
              boxShadow: 'var(--shadow-lg)',
            }}>
              {/* Header */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                <span className="page-kicker">
                  Planner v2
                </span>
                <h2 className="display-heading" style={{ margin: 0, fontSize: 'clamp(1.9rem, 1.55rem + 1vw, 2.5rem)' }}>
                  Start with the planning brief
                </h2>
                <p className="section-copy" style={{ margin: 0 }}>
                  Describe what you are building, who it serves, and the constraints that matter. Planner will move straight into the next question from there.
                </p>
              </div>

              {/* Textarea */}
              <div style={{
                background: 'var(--color-surface-offset)',
                boxShadow: `inset 0 0 0 1px ${description.trim() ? 'var(--color-primary)' : 'var(--color-ghost-border)'}`,
                borderRadius: '14px',
                padding: '12px 16px',
                transition: 'box-shadow 0.18s',
              }}>
                <textarea
                  ref={textareaRef}
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey && !isStarting && description.trim()) {
                      e.preventDefault();
                      void handleStartInterview();
                    }
                  }}
                  disabled={isStarting}
                  placeholder="e.g. A multi-tenant SaaS dashboard for tracking equipment rentals, with role-based access for admins and field staff…"
                  rows={4}
                  aria-label="Planning brief"
                  style={{
                    width: '100%',
                    background: 'transparent',
                    border: 'none',
                    outline: 'none',
                    color: isStarting ? 'var(--color-text-muted)' : 'var(--color-text)',
                    fontSize: '13px',
                    lineHeight: '1.6',
                    resize: 'none',
                    minHeight: '90px',
                    maxHeight: '300px',
                    overflowY: 'auto',
                    fontFamily: 'inherit',
                    boxSizing: 'border-box',
                    cursor: isStarting ? 'not-allowed' : 'text',
                  }}
                />
              </div>

              {/* Character count */}
              <div style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
              }}>
                <span style={{
                  fontSize: '11px',
                  color: description.length > 2000 ? 'var(--color-error)' : 'var(--color-text-muted)',
                }}>
                  {description.length} chars
                  {description.length > 2000 && ' — try to keep it concise'}
                </span>
                <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
                  Enter to submit
                </span>
              </div>

              {/* Error */}
              {startError && (
                <div style={{
                  padding: '10px 12px',
                  background: 'rgba(255,68,68,0.10)',
                  borderRadius: '10px',
                  boxShadow: 'inset 0 0 0 1px rgba(209, 99, 167, 0.24)',
                  color: 'var(--color-error)',
                  fontSize: '12px',
                }}>
                  {startError}
                </div>
              )}

              {/* Submit button */}
              <button
                className={!description.trim() || isStarting ? 'btn' : 'btn btn-primary'}
                onClick={() => void handleStartInterview()}
                disabled={!description.trim() || isStarting}
                style={{
                  alignSelf: 'flex-end',
                  minWidth: '182px',
                }}
              >
                {isStarting ? 'Starting…' : 'Start Session'}
              </button>
            </div>
          </div>

          {/* Disabled MessageInput for consistent chrome */}
          <MessageInput
            onSend={() => undefined}
            disabled={true}
            intakePhase="waiting"
          />
        </div>
      </Layout>
    );
  }

  // ─────────────────────────────────────────────────────────────────
  // PHASE: interviewing, pipeline_running, complete, or error
  // All share the split-pane layout
  // ─────────────────────────────────────────────────────────────────
  const interviewResumeNotice =
    isExistingSessionRoute &&
    session?.intake_phase === 'interviewing' &&
    !socratic.isConnected
      ? getInterviewResumeNotice(session.resume_status)
      : null;
  const detachedCheckpoint =
    isExistingSessionRoute &&
    session?.intake_phase === 'interviewing' &&
    !socratic.isConnected
      ? session?.checkpoint ?? null
      : null;
  const checkpointSummaryLines = detachedCheckpoint
    ? getCheckpointSummary(detachedCheckpoint)
    : [];
  const sessionTitle = session ? getSessionTitle(session) : null;

  return (
    <Layout sessionId={sessionId} isConnected={socratic.isConnected}>
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        overflow: 'hidden',
      }}>
        {reconnectBanner}
        {workflowErrorBanner}

        

        {/* Error banner */}
        {isError && (
          <div style={{
            padding: '12px 16px',
            background: 'color-mix(in srgb, var(--color-error-highlight) 72%, transparent)',
            color: 'var(--color-error)',
            fontSize: '12px',
            textAlign: 'center',
            flexShrink: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '12px',
          }}>
            <span>Interview failed. Check server logs for details.</span>
            <button
              onClick={() => {
                const target = session?.project_slug
                  ? `/projects/${encodeURIComponent(session.project_slug)}/sessions`
                  : '/sessions';
                void navigate(target);
              }}
              style={{
                background: 'rgba(209, 99, 167, 0.12)',
                boxShadow: 'inset 0 0 0 1px rgba(209, 99, 167, 0.24)',
                borderRadius: '999px',
                color: 'var(--color-error)',
                fontSize: '11px',
                fontFamily: 'inherit',
                padding: '4px 12px',
                cursor: 'pointer',
              }}
            >
              ← Back
            </button>
          </div>
        )}

        {/* Interview resume limitation banner */}
        {interviewResumeNotice && (
          <div style={{
            padding: '10px 16px',
            background: interviewResumeNotice.tone === 'warning'
              ? 'color-mix(in srgb, var(--color-gold-highlight) 76%, transparent)'
              : 'color-mix(in srgb, var(--color-primary-highlight) 68%, transparent)',
            color: interviewResumeNotice.tone === 'warning'
              ? 'var(--color-gold)'
              : 'var(--color-primary)',
            fontSize: '12px',
            textAlign: 'center',
            flexShrink: 0,
          }}>
            {interviewResumeNotice.text}
          </div>
        )}

        {detachedCheckpoint && (
          <div style={{
            padding: '10px 16px',
            background: 'color-mix(in srgb, var(--color-primary-highlight) 68%, transparent)',
            color: 'var(--color-text)',
            fontSize: '12px',
            lineHeight: '1.45',
            display: 'flex',
            flexDirection: 'column',
            gap: '4px',
            flexShrink: 0,
          }}>
            <span style={{ color: 'var(--color-primary)', fontWeight: 700, letterSpacing: '0.04em' }}>
              Saved Interview Checkpoint
            </span>
            <span style={{ color: 'var(--color-text-muted)' }}>
              Last saved: {formatCheckpointTimestamp(detachedCheckpoint.last_checkpoint_at)}
            </span>
            {getCheckpointTargetDimension(detachedCheckpoint) && (
              <span>
                Target dimension: {getCheckpointTargetDimension(detachedCheckpoint)}
              </span>
            )}
            {checkpointSummaryLines.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </div>
        )}

        {/* Status header + Top progress bar */}
        {(isInterviewing || isPipelineRunning || isComplete || isError || sessionActions.length > 0) && (
          <SessionStatusHeader
            sessionTitle={sessionTitle}
            sessionId={session?.id}
            isArchived={session?.archived}
            currentStep={socratic.currentStep}
            events={socratic.events}
            isError={isError}
            errorMessage={session?.error_message}
            actions={sessionActions}
            eventSummary={{
              total: eventCounts.total,
              warnings: eventCounts.warnings,
              errors: eventCounts.errors,
              unread: eventUnreadCount,
            }}
            onOpenEvents={() => {
              setRightTab('events');
              setEventUnreadCount(0);
              setContextShelfOpen(true);
            }}
          />
        )}



        {showFocusedLobby ? (
          <div className="socratic-focused-lobby-shell">
            {!showFirstRevealPreload && (
              <SessionPulseBar
                sessionTitle={sessionTitle}
                currentStep={socratic.currentStep}
                events={socratic.events}
                isError={isError}
                errorMessage={session?.error_message}
                workspace={displayWorkspace}
                unreadEventCount={eventUnreadCount}
                hasDraft={Boolean(socratic.speculativeDraft)}
                isContextShelfOpen={contextShelfOpen}
                onToggleContextShelf={() => setContextShelfOpen((value) => !value)}
              />
            )}

            {showFirstRevealPreload ? (
              <div
                style={{
                  minHeight: 0,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  padding: '32px 24px 40px',
                }}
              >
                <div
                  style={{
                    width: 'min(760px, 100%)',
                    display: 'grid',
                    gap: '16px',
                  }}
                >
                  <div style={{ display: 'grid', gap: '6px' }}>
                    <span className="page-kicker">First reveal</span>
                    <h2 className="display-heading" style={{ margin: 0, fontSize: 'clamp(1.6rem, 1.3rem + 0.9vw, 2.2rem)' }}>
                      Planner is preparing the first working set of questions.
                    </h2>
                    <p className="section-copy" style={{ margin: 0 }}>
                      The lobby will open once the initial set feels substantially loaded, not while it is still assembling.
                    </p>
                  </div>
                  <div
                    style={{
                      display: 'grid',
                      gap: '10px',
                      padding: '14px 16px',
                      borderRadius: '16px',
                      background: 'color-mix(in srgb, var(--color-surface) 84%, transparent)',
                      boxShadow: 'inset 0 0 0 1px color-mix(in srgb, var(--color-divider) 70%, transparent)',
                    }}
                  >
                    <span style={{ fontSize: '11px', color: 'var(--color-primary)', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase' }}>
                      Initial target
                    </span>
                    <span style={{ fontSize: '13px', color: 'var(--color-text)' }}>
                      {knownQuestionCount}/{FIRST_REVEAL_PRELOAD_TARGET} locally known question items ready for the first reveal
                    </span>
                    <span style={{ fontSize: '12px', color: 'var(--color-text-muted)' }}>
                      Once the desk opens, browsing known categories and questions stays immediate on this client.
                    </span>
                    {firstRevealSoftTargetReached && (
                      <span style={{ fontSize: '12px', color: 'var(--color-text-muted)' }}>
                        This first load is taking longer than usual. Planner will open the best available initial set shortly if it cannot fill the full target in time.
                      </span>
                    )}
                  </div>
                  <InterviewProgressPanel
                    currentStep={socratic.currentStep}
                    events={socratic.events}
                    isConnected={socratic.isConnected}
                  />
                </div>
              </div>
            ) : (
              <>
                {firstRevealUsedTimeoutFallback && knownQuestionCount < FIRST_REVEAL_PRELOAD_TARGET && !buildReady && (
                  <div
                    style={{
                      padding: '10px 16px',
                      background: 'color-mix(in srgb, var(--color-primary-highlight) 68%, transparent)',
                      color: 'var(--color-text)',
                      fontSize: '12px',
                      textAlign: 'center',
                      flexShrink: 0,
                    }}
                  >
                    Planner opened the desk with a partial initial set so you can begin while more questions continue to arrive.
                  </div>
                )}
                {displayWorkspace ? (
                  <SocraticWorkspace
                    workspace={displayWorkspace}
                    currentPrompt={displayPrompt}
                    pendingCategoryId={socratic.pendingCategoryId}
                    workspaceNotice={socratic.workspaceNotice}
                    disabled={!socratic.isConnected}
                    onFocusCategory={socratic.enterCategory}
                    onShowAll={socratic.backToCategories}
                    onSubmitAnswers={socratic.submitPromptAnswers}
                    onDone={socratic.sendDone}
                  />
                ) : (
                  <div
                    style={{
                      minHeight: 0,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      padding: '32px 24px 40px',
                    }}
                  >
                    <div
                      style={{
                        width: 'min(760px, 100%)',
                        display: 'grid',
                        gap: '16px',
                      }}
                    >
                      <div style={{ display: 'grid', gap: '6px' }}>
                        <span className="page-kicker">Focused intake</span>
                        <h2 className="display-heading" style={{ margin: 0, fontSize: 'clamp(1.6rem, 1.3rem + 0.9vw, 2.2rem)' }}>
                          Planner is opening the first question.
                        </h2>
                        <p className="section-copy" style={{ margin: 0 }}>
                          Stay on this screen. The next question will land here automatically as soon as the interview runtime responds.
                        </p>
                      </div>
                      <InterviewProgressPanel
                        currentStep={socratic.currentStep}
                        events={socratic.events}
                        isConnected={socratic.isConnected}
                      />
                    </div>
                  </div>
                )}
              </>
            )}

            {contextShelfOpen && (
              <div
                style={{
                  position: 'fixed',
                  top: 0,
                  left: 0,
                  right: 0,
                  bottom: 0,
                  backgroundColor: 'rgba(0,0,0,0.4)',
                  zIndex: 40,
                  display: 'flex',
                  justifyContent: 'flex-end',
                  backdropFilter: 'blur(2px)'
                }}
                onClick={() => setContextShelfOpen(false)}
              >
                <aside
                  aria-label="Context shelf"
                  onClick={(e) => e.stopPropagation()}
                  style={{
                    width: 'min(480px, 100vw)',
                    height: '100%',
                    display: 'flex',
                    flexDirection: 'column',
                    background: 'rgba(20, 20, 22, 0.75)',
                    backdropFilter: 'blur(24px)',
                    WebkitBackdropFilter: 'blur(24px)',
                    boxShadow: '-12px 0 48px rgba(0,0,0,0.3)',
                    borderLeft: '1px solid rgba(255, 255, 255, 0.1)',
                    animation: 'slideInRight 0.3s cubic-bezier(0.16, 1, 0.3, 1)',
                  }}
                >
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: '12px',
                    padding: '14px 16px 0',
                  }}
                >
                  <div style={{ display: 'grid', gap: '4px' }}>
                    <span style={{ fontSize: '11px', color: 'var(--color-primary)', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase' }}>
                      Context shelf
                    </span>
                    <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>
                      Open belief state, draft, transcript, or events without leaving the active question flow.
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={() => setContextShelfOpen(false)}
                    style={{
                      background: 'var(--color-surface-2)',
                      boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                      borderRadius: '999px',
                      color: 'var(--color-text)',
                      fontSize: '11px',
                      fontWeight: 700,
                      padding: '6px 12px',
                    }}
                  >
                    Close
                  </button>
                </div>

                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    background: 'rgba(255, 255, 255, 0.03)',
                    border: '1px solid rgba(255, 255, 255, 0.05)',
                    flexShrink: 0,
                    overflowX: 'auto',
                    margin: '14px 16px 0',
                    padding: '4px',
                    borderRadius: '12px',
                  }}
                >
                  <button
                    onClick={() => setRightTab('belief')}
                    style={{
                      flex: 1,
                      background: rightTab === 'belief' ? 'rgba(255, 255, 255, 0.1)' : 'transparent',
                      border: 'none',
                      color: rightTab === 'belief' ? 'var(--color-text)' : 'var(--color-text-muted)',
                      fontSize: '11px',
                      fontWeight: rightTab === 'belief' ? 600 : 500,
                      fontFamily: 'inherit',
                      padding: '6px 12px',
                      cursor: 'pointer',
                      letterSpacing: '0.02em',
                      transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                      borderRadius: '8px',
                      boxShadow: rightTab === 'belief' ? '0 2px 8px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.1)' : 'none',
                    }}
                  >
                    Belief State
                  </button>
                  <button
                    onClick={() => socratic.speculativeDraft && setRightTab('draft')}
                    disabled={!socratic.speculativeDraft}
                    title={!socratic.speculativeDraft ? 'No draft available yet' : undefined}
                    style={{
                      flex: 1,
                      background: rightTab === 'draft' ? 'rgba(255, 255, 255, 0.1)' : 'transparent',
                      border: 'none',
                      color: !socratic.speculativeDraft
                        ? 'rgba(255,255,255,0.2)'
                        : rightTab === 'draft'
                          ? 'var(--color-text)'
                          : 'var(--color-text-muted)',
                      fontSize: '11px',
                      fontWeight: rightTab === 'draft' ? 600 : 500,
                      fontFamily: 'inherit',
                      padding: '6px 12px',
                      cursor: !socratic.speculativeDraft ? 'not-allowed' : 'pointer',
                      letterSpacing: '0.02em',
                      transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                      borderRadius: '8px',
                      boxShadow: rightTab === 'draft' ? '0 2px 8px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.1)' : 'none',
                    }}
                  >
                    Draft
                  </button>
                  <button
                    onClick={() => setRightTab('transcript')}
                    style={{
                      flex: 1,
                      background: rightTab === 'transcript' ? 'rgba(255, 255, 255, 0.1)' : 'transparent',
                      border: 'none',
                      color: rightTab === 'transcript' ? 'var(--color-text)' : 'var(--color-text-muted)',
                      fontSize: '11px',
                      fontWeight: rightTab === 'transcript' ? 600 : 500,
                      fontFamily: 'inherit',
                      padding: '6px 12px',
                      cursor: 'pointer',
                      letterSpacing: '0.02em',
                      transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                      borderRadius: '8px',
                      boxShadow: rightTab === 'transcript' ? '0 2px 8px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.1)' : 'none',
                    }}
                  >
                    Transcript
                  </button>
                  <button
                    onClick={() => {
                      setRightTab('events');
                      setEventUnreadCount(0);
                    }}
                    style={{
                      flex: 1,
                      background: rightTab === 'events' ? 'rgba(255, 255, 255, 0.1)' : 'transparent',
                      border: 'none',
                      color: rightTab === 'events' ? 'var(--color-text)' : 'var(--color-text-muted)',
                      fontSize: '11px',
                      fontWeight: rightTab === 'events' ? 600 : 500,
                      fontFamily: 'inherit',
                      padding: '6px 12px',
                      cursor: 'pointer',
                      letterSpacing: '0.02em',
                      transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                      borderRadius: '8px',
                      boxShadow: rightTab === 'events' ? '0 2px 8px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.1)' : 'none',
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      gap: '6px',
                    }}
                  >
                    Events
                    {eventUnreadCount > 0 && rightTab !== 'events' && (
                      <span
                        style={{
                          display: 'inline-flex',
                          minWidth: '16px',
                          justifyContent: 'center',
                          padding: '0 4px',
                          borderRadius: '999px',
                          background: 'var(--color-primary)',
                          color: 'var(--color-bg)',
                          fontSize: '9px',
                          fontWeight: 800,
                          lineHeight: 1.4,
                        }}
                      >
                        {eventUnreadCount}
                      </span>
                    )}
                  </button>
                </div>

                <div style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column', minHeight: 0, padding: '12px 0 0' }}>
                  {rightPanelContent}
                </div>
              </aside>
              </div>
            )}
          </div>
        ) : (
          <div className="split-pane">
            <div className="pane-left">
              <ChatPanel messages={socratic.messages} />

              {isComplete && socratic.pipelineSummary && (
                <div style={{
                  padding: '12px 16px',
                  background: 'color-mix(in srgb, var(--color-success-highlight) 72%, transparent)',
                  color: 'var(--color-success)',
                  fontSize: '12px',
                  flexShrink: 0,
                  lineHeight: '1.5',
                }}>
                  <span style={{ fontWeight: 700, letterSpacing: '0.04em' }}>Pipeline complete - </span>
                  {socratic.pipelineSummary}
                </div>
              )}

              {isInterviewing ? (
                <>
                  {displayCategorySnapshot && (
                    <CategoryNavigator
                      snapshot={displayCategorySnapshot}
                      onEnterCategory={socratic.enterCategory}
                      onBack={socratic.backToCategories}
                      onDone={socratic.sendDone}
                      disabled={!socratic.isConnected}
                    />
                  )}
                  {!displayWorkspace && displayPrompt ? (
                    <QuestionCanvas
                      prompt={displayPrompt}
                      onSubmit={(_promptId, answers) => socratic.submitPromptAnswers(answers)}
                      disabled={!socratic.isConnected}
                    />
                  ) : !displayWorkspace && !displayCategorySnapshot ? (
                    <InterviewProgressPanel
                      currentStep={socratic.currentStep}
                      events={socratic.events}
                      isConnected={socratic.isConnected}
                    />
                  ) : null}
                </>
              ) : (
                <MessageInput
                  onSend={() => undefined}
                  intakePhase={effectivePhase}
                  onDone={socratic.sendDone}
                  disabled={true}
                  pipelineRunning={isPipelineRunning}
                  convergencePct={socratic.convergencePct * 100}
                />
              )}
            </div>

            <div className="pane-right" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '0',
                  background: 'var(--color-surface-offset)',
                  flexShrink: 0,
                  overflowX: 'auto',
                  padding: '6px 8px',
                }}
              >
                <button
                  onClick={() => setRightTab('belief')}
                  style={{
                    background: rightTab === 'belief' ? 'var(--color-surface-2)' : 'transparent',
                    border: 'none',
                    color: rightTab === 'belief' ? 'var(--color-primary)' : 'var(--color-text-muted)',
                    fontSize: '11px',
                    fontWeight: rightTab === 'belief' ? 700 : 400,
                    fontFamily: 'inherit',
                    padding: '8px 14px',
                    cursor: 'pointer',
                    letterSpacing: '0.03em',
                    transition: 'color 0.15s, background 0.15s',
                    borderRadius: '999px',
                    boxShadow: rightTab === 'belief' ? 'var(--shadow-sm)' : 'none',
                  }}
                >
                  Belief State
                </button>
                <button
                  onClick={() => socratic.speculativeDraft && setRightTab('draft')}
                  disabled={!socratic.speculativeDraft}
                  title={!socratic.speculativeDraft ? 'No draft available yet' : undefined}
                  style={{
                    background: rightTab === 'draft' ? 'var(--color-surface-2)' : 'transparent',
                    border: 'none',
                    color: !socratic.speculativeDraft
                      ? 'var(--color-text-muted)'
                      : rightTab === 'draft'
                        ? 'var(--color-primary)'
                        : 'var(--color-text-muted)',
                    fontSize: '11px',
                    fontWeight: rightTab === 'draft' ? 700 : 400,
                    fontFamily: 'inherit',
                    padding: '8px 14px',
                    cursor: !socratic.speculativeDraft ? 'not-allowed' : 'pointer',
                    letterSpacing: '0.03em',
                    opacity: !socratic.speculativeDraft ? 0.4 : 1,
                    transition: 'color 0.15s, background 0.15s, opacity 0.15s',
                    borderRadius: '999px',
                    boxShadow: rightTab === 'draft' ? 'var(--shadow-sm)' : 'none',
                  }}
                >
                  Draft
                </button>
                <button
                  onClick={() => setRightTab('transcript')}
                  style={{
                    background: rightTab === 'transcript' ? 'var(--color-surface-2)' : 'transparent',
                    border: 'none',
                    color: rightTab === 'transcript' ? 'var(--color-primary)' : 'var(--color-text-muted)',
                    fontSize: '11px',
                    fontWeight: rightTab === 'transcript' ? 700 : 400,
                    fontFamily: 'inherit',
                    padding: '8px 14px',
                    cursor: 'pointer',
                    letterSpacing: '0.03em',
                    transition: 'color 0.15s, background 0.15s',
                    borderRadius: '999px',
                    boxShadow: rightTab === 'transcript' ? 'var(--shadow-sm)' : 'none',
                  }}
                >
                  Transcript
                </button>
                <button
                  onClick={() => {
                    setRightTab('events');
                    setEventUnreadCount(0);
                  }}
                  style={{
                    background: rightTab === 'events' ? 'var(--color-surface-2)' : 'transparent',
                    border: 'none',
                    color: rightTab === 'events' ? 'var(--color-primary)' : 'var(--color-text-muted)',
                    fontSize: '11px',
                    fontWeight: rightTab === 'events' ? 700 : 400,
                    fontFamily: 'inherit',
                    padding: '8px 14px',
                    cursor: 'pointer',
                    letterSpacing: '0.03em',
                    transition: 'color 0.15s, background 0.15s',
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '6px',
                    whiteSpace: 'nowrap',
                    borderRadius: '999px',
                    boxShadow: rightTab === 'events' ? 'var(--shadow-sm)' : 'none',
                  }}
                >
                  Events
                  {eventUnreadCount > 0 && rightTab !== 'events' && (
                    <span
                      style={{
                        display: 'inline-flex',
                        minWidth: '16px',
                        justifyContent: 'center',
                        padding: '0 4px',
                        borderRadius: '999px',
                        background: 'var(--color-primary-highlight)',
                        color: 'var(--color-primary)',
                        fontSize: '10px',
                        fontWeight: 700,
                        lineHeight: 1.4,
                      }}
                    >
                      {eventUnreadCount}
                    </span>
                  )}
                </button>
              </div>

              <div style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
                {rightPanelContent}
              </div>
            </div>
          </div>
        )}
      </div>
    </Layout>
  );
}
