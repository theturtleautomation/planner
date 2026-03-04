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
import type {
  BeliefState,
  ChatMessage,
  Classification,
  ClientWsMessage,
  DraftAssumption,
  DraftSection,
  IntakePhase,
  PipelineStage,
  PipelineStageName,
  QuickOption,
  ServerWsMessage,
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

type GetTokenFn = () => Promise<string>;

/** The current question being posed to the user, if any. */
export interface CurrentQuestion {
  text: string;
  targetDimension: string;
  quickOptions: QuickOption[];
  allowSkip: boolean;
}

export interface UseSocraticWebSocketOptions {
  sessionId: string | null;
  getToken: GetTokenFn;
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
  currentQuestion: CurrentQuestion | null;
  speculativeDraft: SpeculativeDraft | null;

  // Pipeline state (active after convergence)
  stages: PipelineStage[];
  pipelineComplete: boolean;
  pipelineSummary: string | null;

  // Actions
  sendDescription: (description: string) => void;
  sendResponse: (content: string) => void;
  skipQuestion: () => void;
  sendDone: () => void;
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
  const [currentQuestion, setCurrentQuestion] = useState<CurrentQuestion | null>(null);
  const [speculativeDraft, setSpeculativeDraft] = useState<SpeculativeDraft | null>(null);

  // Pipeline
  const [stages, setStages] = useState<PipelineStage[]>(buildInitialStages);
  const [pipelineComplete, setPipelineComplete] = useState(false);
  const [pipelineSummary, setPipelineSummary] = useState<string | null>(null);

  // Refs
  const wsRef = useRef<WebSocket | null>(null);
  const retryCountRef = useRef(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);
  const sessionIdRef = useRef(sessionId);

  useEffect(() => { sessionIdRef.current = sessionId; }, [sessionId]);

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
          question_budget: msg.question_budget,
        });
        // Add a planner message for the chat
        setMessages((prev) => [...prev, {
          id: crypto.randomUUID(),
          role: 'planner',
          content: `Classified as: **${msg.project_type}** (${msg.complexity}). I'll ask up to ${msg.question_budget} questions.`,
          timestamp: new Date().toISOString(),
        }]);
        break;
      }

      case 'belief_state_update': {
        const bs: BeliefState = {
          filled: msg.filled as BeliefState['filled'],
          uncertain: msg.uncertain as BeliefState['uncertain'],
          missing: msg.missing,
          out_of_scope: msg.out_of_scope ?? [],
          convergence_pct: msg.convergence_pct,
        };
        setBeliefState(bs);
        setConvergencePct(msg.convergence_pct);
        break;
      }

      case 'question': {
        setCurrentQuestion({
          text: msg.text,
          targetDimension: msg.target_dimension,
          quickOptions: msg.quick_options ?? [],
          allowSkip: msg.allow_skip,
        });
        // Also add the question text as a planner chat message
        setMessages((prev) => [...prev, {
          id: crypto.randomUUID(),
          role: 'planner',
          content: msg.text,
          timestamp: new Date().toISOString(),
        }]);
        break;
      }

      case 'speculative_draft': {
        setSpeculativeDraft({
          sections: msg.sections as DraftSection[],
          assumptions: msg.assumptions as DraftAssumption[],
          not_discussed: msg.not_discussed,
        });
        setMessages((prev) => [...prev, {
          id: crypto.randomUUID(),
          role: 'planner',
          content: 'Here\'s a speculative draft based on what I know so far. Review it in the right panel.',
          timestamp: new Date().toISOString(),
        }]);
        break;
      }

      case 'converged': {
        setConvergencePct(msg.convergence_pct);
        setCurrentQuestion(null);
        setIntakePhase('pipeline_running');
        setMessages((prev) => [...prev, {
          id: crypto.randomUUID(),
          role: 'planner',
          content: `Requirements gathering complete (${Math.round(msg.convergence_pct * 100)}% converged). Starting the planning pipeline\u2026`,
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
        const cm: ChatMessage = {
          id: msg.id ?? crypto.randomUUID(),
          role: msg.role,
          content: msg.content,
          timestamp: msg.timestamp ?? new Date().toISOString(),
        };
        setMessages((prev) => [...prev, cm]);
        break;
      }

      case 'pipeline_complete': {
        setPipelineComplete(true);
        setPipelineSummary(msg.summary);
        setIntakePhase('complete');
        break;
      }

      case 'error': {
        console.error('[Socratic WS] server error:', msg.message);
        setMessages((prev) => [...prev, {
          id: crypto.randomUUID(),
          role: 'system',
          content: `Error: ${msg.message}`,
          timestamp: new Date().toISOString(),
        }]);
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

  // -------------------------------------------------------------------------
  // Lifecycle
  // -------------------------------------------------------------------------

  useEffect(() => {
    mountedRef.current = true;
    retryCountRef.current = 0;

    // Reset all state when session changes
    setIsConnected(false);
    setReconnectFailed(false);
    setIntakePhase('waiting');
    setMessages([]);
    setClassification(null);
    setBeliefState(null);
    setConvergencePct(0);
    setCurrentQuestion(null);
    setSpeculativeDraft(null);
    setStages(buildInitialStages());
    setPipelineComplete(false);
    setPipelineSummary(null);

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
        sendRaw({ type: 'socratic_response', content: description });
        setIntakePhase('interviewing');
      })();
    } else {
      sendRaw({ type: 'socratic_response', content: description });
      setIntakePhase('interviewing');
    }
  }, [connect, sendRaw]);

  /** Send a user response during the interview. */
  const sendResponse = useCallback((content: string): void => {
    setMessages((prev) => [...prev, {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      timestamp: new Date().toISOString(),
    }]);
    setCurrentQuestion(null);
    sendRaw({ type: 'socratic_response', content });
  }, [sendRaw]);

  /** Skip the current question. */
  const skipQuestion = useCallback((): void => {
    setCurrentQuestion(null);
    setMessages((prev) => [...prev, {
      id: crypto.randomUUID(),
      role: 'system',
      content: '(Question skipped)',
      timestamp: new Date().toISOString(),
    }]);
    sendRaw({ type: 'skip_question' });
  }, [sendRaw]);

  /** Signal "done, start building." */
  const sendDone = useCallback((): void => {
    setMessages((prev) => [...prev, {
      id: crypto.randomUUID(),
      role: 'system',
      content: '(Done — starting pipeline)',
      timestamp: new Date().toISOString(),
    }]);
    sendRaw({ type: 'done' });
  }, [sendRaw]);

  return {
    isConnected,
    reconnectFailed,
    intakePhase,
    messages,
    classification,
    beliefState,
    convergencePct,
    currentQuestion,
    speculativeDraft,
    stages,
    pipelineComplete,
    pipelineSummary,
    sendDescription,
    sendResponse,
    skipQuestion,
    sendDone,
  };
}
