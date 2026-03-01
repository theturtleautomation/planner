import { useEffect, useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { Session } from '../types.ts';

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleString([], {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', hour12: false,
    });
  } catch {
    return iso;
  }
}

export default function Dashboard() {
  const navigate = useNavigate();
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(true);
  const [fetchError, setFetchError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async (): Promise<void> => {
      setLoading(true);
      setFetchError(null);
      try {
        const resp = await api.listSessions();
        if (!cancelled) setSessions(resp.sessions);
      } catch (err) {
        if (!cancelled) {
          const msg = err instanceof Error ? err.message : String(err);
          setFetchError(msg);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    };
    void load();
    return () => { cancelled = true; };
  }, [api]);

  const handleNewSession = (): void => {
    void navigate('/session/new');
  };

  return (
    <Layout>
      <div style={{
        flex: 1,
        overflow: 'auto',
        padding: '32px 24px',
        display: 'flex',
        flexDirection: 'column',
        gap: '24px',
        maxWidth: '800px',
        margin: '0 auto',
        width: '100%',
      }}>
        {/* Section header */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderBottom: '1px solid var(--border)',
          paddingBottom: '12px',
        }}>
          <span style={{ color: 'var(--text-primary)', fontSize: '14px', fontWeight: 600 }}>
            sessions
          </span>
          <button
            onClick={handleNewSession}
            style={{
              background: 'var(--accent-cyan)',
              border: 'none',
              color: 'var(--bg-primary)',
              padding: '7px 18px',
              fontSize: '12px',
              fontWeight: 700,
              cursor: 'pointer',
              letterSpacing: '0.05em',
              textTransform: 'uppercase',
              borderRadius: '2px',
              fontFamily: 'inherit',
              transition: 'opacity 0.18s',
            }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.opacity = '0.85'; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.opacity = '1'; }}
          >
            + new session
          </button>
        </div>

        {/* Loading state */}
        {loading && (
          <div style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '40px 24px',
            color: 'var(--text-secondary)',
            fontSize: '13px',
          }}>
            loading sessions…
          </div>
        )}

        {/* Error state */}
        {!loading && fetchError && (
          <div style={{
            padding: '16px',
            border: '1px solid var(--accent-red)',
            borderRadius: '3px',
            background: 'rgba(255,68,68,0.06)',
            color: 'var(--accent-red)',
            fontSize: '13px',
          }}>
            <span style={{ fontWeight: 600 }}>Error loading sessions: </span>
            {fetchError}
          </div>
        )}

        {/* Empty state */}
        {!loading && !fetchError && sessions.length === 0 && (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '60px 24px',
            border: '1px dashed var(--border)',
            borderRadius: '3px',
            gap: '12px',
          }}>
            <span style={{ color: 'var(--text-secondary)', fontSize: '13px' }}>
              no sessions yet
            </span>
            <span style={{ color: 'var(--text-secondary)', fontSize: '12px' }}>
              create a new session to start planning
            </span>
            <button
              onClick={handleNewSession}
              style={{
                marginTop: '8px',
                background: 'transparent',
                border: '1px solid var(--accent-cyan)',
                color: 'var(--accent-cyan)',
                padding: '8px 20px',
                fontSize: '12px',
                cursor: 'pointer',
                borderRadius: '2px',
                fontFamily: 'inherit',
                transition: 'background 0.18s',
              }}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLButtonElement).style.background = 'rgba(0,212,255,0.08)';
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLButtonElement).style.background = 'transparent';
              }}
            >
              start new session →
            </button>
          </div>
        )}

        {/* Session list */}
        {!loading && !fetchError && sessions.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            {sessions.map((session) => (
              <SessionCard
                key={session.id}
                session={session}
                onClick={() => void navigate(`/session/${session.id}`)}
              />
            ))}
          </div>
        )}

        {/* Info box */}
        <div style={{
          padding: '14px 16px',
          background: 'var(--bg-secondary)',
          border: '1px solid var(--border)',
          borderRadius: '3px',
          fontSize: '12px',
          color: 'var(--text-secondary)',
          lineHeight: 1.7,
        }}>
          <span style={{ color: 'var(--accent-cyan)', fontWeight: 600 }}>TIP</span>
          {' '}— Each session maintains its own conversation history and pipeline state.
          Sessions are isolated and can be resumed at any time.
        </div>
      </div>
    </Layout>
  );
}

interface SessionCardProps {
  session: Session;
  onClick: () => void;
}

function SessionCard({ session, onClick }: SessionCardProps) {
  const messageCount = session.messages?.length ?? 0;
  const pipelineStatus = session.pipeline_running ? 'running' : 'idle';
  const pipelineColor = session.pipeline_running ? 'var(--accent-yellow)' : 'var(--text-secondary)';

  // Use first message timestamp as created-at approximation, or fall back to id prefix
  const createdAt = session.messages?.[0]?.timestamp
    ? formatDate(session.messages[0].timestamp)
    : `id: ${session.id.slice(0, 8)}`;

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick(); }}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '12px 16px',
        background: 'var(--bg-secondary)',
        border: '1px solid var(--border)',
        borderRadius: '3px',
        cursor: 'pointer',
        transition: 'border-color 0.18s, background 0.18s',
        gap: '12px',
      }}
      onMouseEnter={(e) => {
        (e.currentTarget as HTMLDivElement).style.borderColor = 'var(--accent-cyan)';
        (e.currentTarget as HTMLDivElement).style.background = 'var(--bg-tertiary)';
      }}
      onMouseLeave={(e) => {
        (e.currentTarget as HTMLDivElement).style.borderColor = 'var(--border)';
        (e.currentTarget as HTMLDivElement).style.background = 'var(--bg-secondary)';
      }}
    >
      {/* Session ID */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', minWidth: 0 }}>
        <span style={{
          color: 'var(--accent-cyan)',
          fontSize: '12px',
          fontWeight: 600,
          letterSpacing: '0.04em',
          fontFamily: 'monospace',
        }}>
          {session.id.slice(0, 8)}…
        </span>
        <span style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>
          {createdAt}
        </span>
      </div>

      {/* Stats */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '16px', flexShrink: 0 }}>
        <span style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>
          {messageCount} {messageCount === 1 ? 'msg' : 'msgs'}
        </span>
        <span style={{
          color: pipelineColor,
          fontSize: '11px',
          fontWeight: 500,
        }}>
          {pipelineStatus}
        </span>
        <span style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>→</span>
      </div>
    </div>
  );
}
