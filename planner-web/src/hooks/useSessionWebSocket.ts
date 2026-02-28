import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  ChatMessage,
  ClientWsMessage,
  PipelineStage,
  PipelineStageName,
  ServerWsMessage,
  StageStatus,
} from '../types.ts';

const PIPELINE_STAGE_NAMES: PipelineStageName[] = [
  'Intake', 'Chunk', 'Compile', 'Lint', 'AR Review', 'Refine',
  'Scenarios', 'Ralph', 'Graph', 'Factory', 'Validate', 'Git',
];

function buildInitialStages(): PipelineStage[] {
  return PIPELINE_STAGE_NAMES.map((name) => ({ name, status: 'pending' as StageStatus }));
}

type GetTokenFn = () => Promise<string>;

interface UseSessionWebSocketOptions {
  sessionId: string | null;
  getToken: GetTokenFn;
}

interface UseSessionWebSocketResult {
  stages: PipelineStage[];
  messages: ChatMessage[];
  isConnected: boolean;
  pipelineComplete: boolean;
  pipelineSummary: string | null;
  sendWsMessage: (msg: ClientWsMessage) => void;
}

const MAX_RETRIES = 3;
const BASE_DELAY_MS = 1000;

export function useSessionWebSocket({
  sessionId,
  getToken,
}: UseSessionWebSocketOptions): UseSessionWebSocketResult {
  const [stages, setStages] = useState<PipelineStage[]>(buildInitialStages);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [pipelineComplete, setPipelineComplete] = useState(false);
  const [pipelineSummary, setPipelineSummary] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const retryCountRef = useRef(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);
  const sessionIdRef = useRef(sessionId);

  // Keep sessionId ref in sync for use inside async callbacks
  useEffect(() => { sessionIdRef.current = sessionId; }, [sessionId]);

  const clearRetryTimer = (): void => {
    if (retryTimerRef.current !== null) {
      clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
  };

  const connect = useCallback(async (): Promise<void> => {
    const id = sessionIdRef.current;
    if (!id || !mountedRef.current) return;

    // Close any existing socket
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }

    const token = await getToken();

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const url = `${protocol}//${host}/api/sessions/${id}/ws${token ? `?token=${encodeURIComponent(token)}` : ''}`;

    let ws: WebSocket;
    try {
      ws = new WebSocket(url);
    } catch (err) {
      console.error('[WS] failed to construct WebSocket', err);
      return;
    }

    wsRef.current = ws;

    ws.onopen = (): void => {
      if (!mountedRef.current) return;
      setIsConnected(true);
      retryCountRef.current = 0;
    };

    ws.onmessage = (event: MessageEvent): void => {
      if (!mountedRef.current) return;
      let msg: ServerWsMessage;
      try {
        msg = JSON.parse(event.data as string) as ServerWsMessage;
      } catch {
        console.warn('[WS] invalid JSON', event.data);
        return;
      }

      switch (msg.type) {
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
            id: crypto.randomUUID(),
            role: msg.role,
            content: msg.content,
            timestamp: new Date().toISOString(),
          };
          setMessages((prev) => [...prev, cm]);
          break;
        }
        case 'pipeline_complete': {
          setPipelineComplete(true);
          setPipelineSummary(msg.summary);
          break;
        }
        case 'error': {
          console.error('[WS] server error:', msg.message);
          break;
        }
      }
    };

    ws.onerror = (ev: Event): void => {
      console.error('[WS] error', ev);
    };

    ws.onclose = (): void => {
      if (!mountedRef.current) return;
      setIsConnected(false);
      wsRef.current = null;

      if (retryCountRef.current < MAX_RETRIES) {
        const delay = BASE_DELAY_MS * Math.pow(2, retryCountRef.current);
        retryCountRef.current += 1;
        console.info(`[WS] reconnecting in ${delay}ms (attempt ${retryCountRef.current}/${MAX_RETRIES})`);
        retryTimerRef.current = setTimeout(() => {
          void connect();
        }, delay);
      } else {
        console.warn('[WS] max retries reached, giving up');
      }
    };
  }, [getToken]);

  // Connect/disconnect when sessionId changes
  useEffect(() => {
    mountedRef.current = true;
    retryCountRef.current = 0;
    setStages(buildInitialStages());
    setPipelineComplete(false);
    setPipelineSummary(null);

    if (sessionId) {
      void connect();
    }

    return (): void => {
      mountedRef.current = false;
      clearRetryTimer();
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close();
        wsRef.current = null;
      }
      setIsConnected(false);
    };
  }, [sessionId, connect]);

  const sendWsMessage = useCallback((msg: ClientWsMessage): void => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    } else {
      console.warn('[WS] cannot send, socket not open');
    }
  }, []);

  return { stages, messages, isConnected, pipelineComplete, pipelineSummary, sendWsMessage };
}
