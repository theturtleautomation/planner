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

// Intake phase badge config
const PHASE_CONFIG: Record<
  'waiting' | 'interviewing' | 'pipeline_running' | 'complete',
  { label: string; color: string; bg: string; borderColor: string; className?: string }
> = {
  waiting: {
    label: 'waiting',
    color: 'var(--text-secondary)',
    bg: 'rgba(136,136,160,0.12)',
    borderColor: 'rgba(136,136,160,0.3)',
  },
  interviewing: {
    label: 'interviewing',
    color: 'var(--accent-cyan)',
    bg: 'rgba(0,212,255,0.08)',
    borderColor: 'var(--accent-cyan)',
    className: 'phase-interviewing',
  },
  pipeline_running: {
    label: 'building',
    color: 'var(--accent-yellow)',
    bg: 'rgba(255,215,0,0.08)',
    borderColor: 'rgba(255,215,0,0.5)',
  },
  complete: {
    label: 'complete',
    color: 'var(--accent-green)',
    bg: 'rgba(0,255,136,0.08)',
    borderColor: 'rgba(0,255,136,0.4)',
  },
};

function SessionCard({ session, onClick }: SessionCardProps) {
  const messageCount = session.messages?.length ?? 0;

  // Use first message timestamp as created-at approximation, or fall back to id prefix
  const createdAt = session.messages?.[0]?.timestamp
    ? formatDate(session.messages[0].timestamp)
    : `id: ${session.id.slice(0, 8)}`;

  // Intake phase
  const phase = session.intake_phase ?? 'waiting';
  const phaseConfig = PHASE_CONFIG[phase] ?? PHASE_CONFIG.waiting;

  // Convergence
  const convergencePct = session.belief_state?.convergence_pct;
  const hasConvergence = convergencePct !== undefined && convergencePct !== null;

  // Classification
  const classification = session.classification;
  const classificationText = classification
    ? `${classification.project_type} · ${classification.complexity}`
    : null;

  // Description snippet
  const descriptionRaw = session.project_description ?? null;
  const descriptionSnippet = descriptionRaw
    ? descriptionRaw.length > 80
      ? descriptionRaw.slice(0, 80) + '…'
      : descriptionRaw
    : null;

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick(); }}
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '6px',
        padding: '12px 16px',
        background: 'var(--bg-secondary)',
        border: '1px solid var(--border)',
        borderRadius: '3px',
        cursor: 'pointer',
        transition: 'border-color 0.18s, background 0.18s',
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
      {/* Row 1: session ID (left) | intake phase badge + convergence (right) */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '8px',
        minWidth: 0,
      }}>
        {/* Session ID + timestamp */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1px', minWidth: 0 }}>
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

        {/* Phase badge + convergence */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
          {hasConvergence && (
            <span style={{
              color: 'var(--text-secondary)',
              fontSize: '10px',
              fontFamily: 'monospace',
            }}>
              {Math.round(convergencePct!)}%
            </span>
          )}
          <span
            className={phaseConfig.className}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              padding: '2px 8px',
              borderRadius: '10px',
              border: `1px solid ${phaseConfig.borderColor}`,
              background: phaseConfig.bg,
              color: phaseConfig.color,
              fontSize: '10px',
              fontWeight: 600,
              letterSpacing: '0.05em',
              textTransform: 'uppercase',
              whiteSpace: 'nowrap',
            }}
          >
            {phaseConfig.label}
          </span>
        </div>
      </div>

      {/* Row 2: description snippet (left) | classification + message count (right) */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '8px',
        minWidth: 0,
      }}>
        {/* Description snippet */}
        <span style={{
          color: 'var(--text-secondary)',
          fontSize: '11px',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          flex: 1,
          minWidth: 0,
        }}>
          {descriptionSnippet ?? <span style={{ fontStyle: 'italic', opacity: 0.6 }}>no description</span>}
        </span>

        {/* Classification + message count */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flexShrink: 0 }}>
          {classificationText && (
            <span style={{
              color: 'var(--text-secondary)',
              fontSize: '10px',
              fontFamily: 'monospace',
              opacity: 0.75,
              whiteSpace: 'nowrap',
            }}>
              {classificationText}
            </span>
          )}
          <span style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>
            {messageCount} {messageCount === 1 ? 'msg' : 'msgs'}
          </span>
          <span style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>→</span>
        </div>
      </div>
    </div>
  );
}
