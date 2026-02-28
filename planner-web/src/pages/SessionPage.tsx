import { useEffect, useState, useCallback, useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import ChatPanel from '../components/ChatPanel.tsx';
import PipelineBar from '../components/PipelineBar.tsx';
import MessageInput from '../components/MessageInput.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import { useSessionWebSocket } from '../hooks/useSessionWebSocket.ts';
import type { ChatMessage, Session } from '../types.ts';

export default function SessionPage() {
  const { id: routeId } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const getToken = useGetAccessToken();

  const api = useMemo(() => createApiClient(getToken), [getToken]);

  // Core session state
  const [session, setSession] = useState<Session | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(
    routeId && routeId !== 'new' ? routeId : null,
  );
  const [initError, setInitError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  // WebSocket
  const { stages, messages: wsMessages, isConnected, pipelineComplete } =
    useSessionWebSocket({ sessionId, getToken });

  // Merge REST messages from session load + WebSocket messages
  const [restMessages, setRestMessages] = useState<ChatMessage[]>([]);
  const allMessages = useMemo(
    () => [...restMessages, ...wsMessages],
    [restMessages, wsMessages],
  );

  // ── Init: create or load session ──
  useEffect(() => {
    let cancelled = false;

    const init = async (): Promise<void> => {
      try {
        if (!routeId || routeId === 'new') {
          // Create a new session
          const resp = await api.createSession();
          if (cancelled) return;
          setSession(resp.session);
          setSessionId(resp.session.id);
          setRestMessages(resp.session.messages ?? []);
          void navigate(`/session/${resp.session.id}`, { replace: true });
        } else {
          // Load existing session
          const resp = await api.getSession(routeId);
          if (cancelled) return;
          setSession(resp.session);
          setSessionId(resp.session.id);
          setRestMessages(resp.session.messages ?? []);
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
  }, [routeId]);

  // Keep session pipeline_running in sync with pipelineComplete
  useEffect(() => {
    if (pipelineComplete && session) {
      setSession((prev) => prev ? { ...prev, pipeline_running: false } : prev);
    }
  }, [pipelineComplete, session]);

  // ── Send message ──
  const handleSend = useCallback(async (content: string): Promise<void> => {
    if (!sessionId) return;
    setIsLoading(true);
    try {
      const resp = await api.sendMessage(sessionId, content);
      setRestMessages((prev) => [...prev, resp.user_message, resp.planner_message]);
      setSession(resp.session);
    } catch (err) {
      console.error('[SessionPage] sendMessage error:', err);
    } finally {
      setIsLoading(false);
    }
  }, [sessionId, api]);

  const pipelineRunning = session?.pipeline_running ?? false;

  // ── Error state ──
  if (initError) {
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
          <span style={{ color: 'var(--accent-red)', fontSize: '14px' }}>[ ERROR ]</span>
          <span style={{ color: 'var(--text-secondary)', fontSize: '13px', textAlign: 'center', maxWidth: '500px' }}>
            {initError}
          </span>
          <button
            onClick={() => void navigate('/')}
            style={{
              marginTop: '8px',
              background: 'transparent',
              border: '1px solid var(--border)',
              color: 'var(--text-secondary)',
              padding: '7px 16px',
              fontSize: '12px',
              cursor: 'pointer',
              fontFamily: 'inherit',
              borderRadius: '2px',
            }}
          >
            ← back to dashboard
          </button>
        </div>
      </Layout>
    );
  }

  // ── Loading state ──
  if (!sessionId && !initError) {
    return (
      <Layout>
        <div style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--text-secondary)',
          fontSize: '13px',
        }}>
          initializing session…
        </div>
      </Layout>
    );
  }

  return (
    <Layout sessionId={sessionId} isConnected={isConnected}>
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        overflow: 'hidden',
      }}>
        {/* Chat area */}
        <ChatPanel messages={allMessages} />

        {/* Pipeline bar */}
        <PipelineBar stages={stages} />

        {/* Input */}
        <MessageInput
          onSend={(content) => { void handleSend(content); }}
          disabled={!sessionId}
          pipelineRunning={pipelineRunning}
          isLoading={isLoading}
        />
      </div>
    </Layout>
  );
}
