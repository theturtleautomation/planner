import { useEffect, useState, useMemo, useCallback } from 'react';
import { Link } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { AdminStatusResponse, AdminEventsResponse, AdminEventEntry } from '../types.ts';
import { buildKnowledgeDeepLink } from '../lib/knowledgeDeepLinks.ts';

// ─── Helpers ──────────────────────────────────────────────────────────────────

function formatUptime(secs: number): string {
  if (secs <= 0) return '0m';
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const parts: string[] = [];
  if (d > 0) parts.push(`${d}d`);
  if (h > 0) parts.push(`${h}h`);
  if (m > 0 || parts.length === 0) parts.push(`${m}m`);
  return parts.join(' ');
}

function formatEventTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    });
  } catch {
    return iso;
  }
}

function secsAgo(date: Date): string {
  const diff = Math.floor((Date.now() - date.getTime()) / 1000);
  if (diff < 60) return `${diff}s ago`;
  return `${Math.floor(diff / 60)}m ${diff % 60}s ago`;
}

// ─── Level badge styles ───────────────────────────────────────────────────────

const LEVEL_STYLE: Record<string, { color: string; bg: string; border: string }> = {
  error: { color: '#ff4444', bg: 'rgba(255,68,68,0.10)', border: 'rgba(255,68,68,0.35)' },
  warn:  { color: '#ffd700', bg: 'rgba(255,215,0,0.08)', border: 'rgba(255,215,0,0.35)' },
  info:  { color: '#00d4ff', bg: 'rgba(0,212,255,0.08)', border: 'rgba(0,212,255,0.3)' },
};

function levelStyle(level: string) {
  return LEVEL_STYLE[level] ?? LEVEL_STYLE.info;
}

// ─── Sub-components ───────────────────────────────────────────────────────────

interface StatusDotProps { ok: boolean }
function StatusDot({ ok }: StatusDotProps) {
  return (
    <span style={{
      display: 'inline-block',
      width: '9px',
      height: '9px',
      borderRadius: '50%',
      background: ok ? 'var(--color-success)' : 'var(--color-gold)',
      flexShrink: 0,
      boxShadow: ok
        ? '0 0 6px rgba(0,255,136,0.45)'
        : '0 0 6px rgba(255,215,0,0.45)',
    }} />
  );
}

interface LevelBadgeProps { level: string }
function LevelBadge({ level }: LevelBadgeProps) {
  const s = levelStyle(level);
  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      padding: '1px 7px',
      borderRadius: '9px',
      background: s.bg,
      color: s.color,
      fontSize: '10px',
      fontWeight: 700,
      letterSpacing: '0.06em',
      textTransform: 'uppercase' as const,
      fontFamily: 'monospace',
      whiteSpace: 'nowrap' as const,
      flexShrink: 0,
    }}>
      {level}
    </span>
  );
}

interface SourceBadgeProps { source: string }
function SourceBadge({ source }: SourceBadgeProps) {
  return (
    <span style={{
      display: 'inline-flex',
      alignItems: 'center',
      padding: '1px 7px',
      borderRadius: '9px',
      background: 'color-mix(in srgb, var(--color-surface-offset) 82%, transparent)',
      color: 'var(--color-text-muted)',
      fontSize: '10px',
      fontWeight: 600,
      letterSpacing: '0.04em',
      fontFamily: 'monospace',
      whiteSpace: 'nowrap' as const,
      flexShrink: 0,
    }}>
      {source}
    </span>
  );
}

// ─── Filter buttons ────────────────────────────────────────────────────────────

type LevelFilter = 'all' | 'error' | 'warn' | 'info';

interface FilterBtnProps {
  label: string;
  active: boolean;
  color: string;
  onClick: () => void;
}
function FilterBtn({ label, active, color, onClick }: FilterBtnProps) {
  return (
    <button
      onClick={onClick}
      style={{
        background: active ? `${color}22` : 'transparent',
        border: 'none',
        boxShadow: active
          ? `inset 0 0 0 1px ${color}`
          : 'inset 0 0 0 1px var(--color-ghost-border)',
        color: active ? color : 'var(--color-text-muted)',
        padding: '6px 12px',
        fontSize: '11px',
        fontWeight: 600,
        cursor: 'pointer',
        borderRadius: '999px',
        fontFamily: 'inherit',
        letterSpacing: '0.04em',
        transition: 'all 0.18s',
      }}
    >
      {label}
    </button>
  );
}

// ─── Event row ────────────────────────────────────────────────────────────────

interface EventRowProps { event: AdminEventEntry }
function EventRow({ event }: EventRowProps) {
  const relatedKnowledgeLink = event.project_id
    ? buildKnowledgeDeepLink({
        projectId: event.project_id,
        originPath: '/admin',
        originLabel: 'Admin',
      })
    : null;

  const s = levelStyle(event.level);

  return (
    <div style={{
      display: 'flex',
      alignItems: 'flex-start',
      gap: '8px',
      padding: '10px 12px',
      borderRadius: '16px',
      background: `linear-gradient(180deg, color-mix(in srgb, ${s.bg} 88%, var(--color-surface-2)), color-mix(in srgb, var(--color-surface) 92%, transparent))`,
      fontSize: '12px',
      lineHeight: 1.5,
    }}>
      {/* Timestamp */}
      <span style={{
        color: 'var(--color-text-muted)',
        fontSize: '11px',
        fontFamily: 'monospace',
        whiteSpace: 'nowrap',
        flexShrink: 0,
        paddingTop: '1px',
        minWidth: '64px',
      }}>
        {formatEventTime(event.timestamp)}
      </span>

      {/* Level */}
      <LevelBadge level={event.level} />

      {/* Source */}
      <SourceBadge source={event.source} />

      {/* Session ID (linked) */}
      {event.session_id && (
        <Link
          to={`/session/${event.session_id}`}
          style={{
            color: 'var(--color-primary)',
            fontSize: '11px',
            fontFamily: 'monospace',
            textDecoration: 'none',
            whiteSpace: 'nowrap',
            flexShrink: 0,
            paddingTop: '1px',
            opacity: 0.8,
          }}
          onMouseEnter={(e) => { (e.currentTarget as HTMLAnchorElement).style.opacity = '1'; }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLAnchorElement).style.opacity = '0.8'; }}
        >
          {event.session_id.slice(0, 8)}
        </Link>
      )}

      {relatedKnowledgeLink && (
        <Link
          to={relatedKnowledgeLink}
          title={event.project_name ? `Open Knowledge for ${event.project_name}` : 'Open project knowledge'}
          style={{
            color: 'var(--color-primary)',
            fontSize: '11px',
            fontFamily: 'monospace',
            textDecoration: 'none',
            whiteSpace: 'nowrap',
            flexShrink: 0,
            paddingTop: '1px',
            opacity: 0.8,
          }}
          onMouseEnter={(e) => { (e.currentTarget as HTMLAnchorElement).style.opacity = '1'; }}
          onMouseLeave={(e) => { (e.currentTarget as HTMLAnchorElement).style.opacity = '0.8'; }}
        >
          Knowledge
        </Link>
      )}

      {/* Step */}
      {event.step && (
        <span style={{
          color: 'var(--color-text-muted)',
          fontSize: '11px',
          fontFamily: 'monospace',
          whiteSpace: 'nowrap',
          flexShrink: 0,
          paddingTop: '1px',
          opacity: 0.7,
        }}>
          ·&nbsp;{event.step}
        </span>
      )}

      {/* Message */}
      <span style={{
        color: 'var(--color-text)',
        fontSize: '12px',
        flex: 1,
        minWidth: 0,
        wordBreak: 'break-word' as const,
      }}>
        {event.message}
      </span>

      {/* Duration */}
      {event.duration_ms !== undefined && event.duration_ms !== null && (
        <span style={{
          color: 'var(--color-text-muted)',
          fontSize: '11px',
          fontFamily: 'monospace',
          whiteSpace: 'nowrap',
          flexShrink: 0,
          paddingTop: '1px',
          opacity: 0.65,
        }}>
          {event.duration_ms}ms
        </span>
      )}
    </div>
  );
}

// ─── Main Page ────────────────────────────────────────────────────────────────

export default function AdminPage() {
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  // Status state
  const [status, setStatus] = useState<AdminStatusResponse | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusUpdatedAt, setStatusUpdatedAt] = useState<Date | null>(null);

  // Events state
  const [eventsData, setEventsData] = useState<AdminEventsResponse | null>(null);
  const [eventsError, setEventsError] = useState<string | null>(null);
  const [eventsUpdatedAt, setEventsUpdatedAt] = useState<Date | null>(null);

  // Filter
  const [levelFilter, setLevelFilter] = useState<LevelFilter>('all');

  // Ticker for "X seconds ago"
  const [, setTick] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setTick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, []);

  // Fetch status
  const fetchStatus = useCallback(async () => {
    try {
      const data = await api.adminStatus();
      setStatus(data);
      setStatusError(null);
      setStatusUpdatedAt(new Date());
    } catch (err) {
      setStatusError(err instanceof Error ? err.message : String(err));
    }
  }, [api]);

  // Fetch events
  const fetchEvents = useCallback(async () => {
    try {
      const data = await api.adminEvents({ limit: 200 });
      setEventsData(data);
      setEventsError(null);
      setEventsUpdatedAt(new Date());
    } catch (err) {
      setEventsError(err instanceof Error ? err.message : String(err));
    }
  }, [api]);

  // Initial fetch + polling
  useEffect(() => {
    void fetchStatus();
    const t = setInterval(() => void fetchStatus(), 10_000);
    return () => clearInterval(t);
  }, [fetchStatus]);

  useEffect(() => {
    void fetchEvents();
    const t = setInterval(() => void fetchEvents(), 5_000);
    return () => clearInterval(t);
  }, [fetchEvents]);

  // Filtered events (newest first)
  const filteredEvents = useMemo(() => {
    if (!eventsData) return [];
    const sorted = [...eventsData.events].reverse();
    if (levelFilter === 'all') return sorted;
    return sorted.filter((e) => e.level === levelFilter);
  }, [eventsData, levelFilter]);

  return (
    <Layout>
      <div className="command-page" style={{ maxWidth: '1000px' }}>
        <div className="command-page-header">
          <div className="command-page-header-copy">
            <span className="page-kicker">Operations</span>
            <h1 className="display-heading">Admin</h1>
            <p className="section-copy">
              Monitor provider availability, queue health, and live system events without dropping into a separate legacy console.
            </p>
          </div>
          <div className="command-page-header-actions">
            <a href="/" className="command-link">Home</a>
            <a href="/sessions" className="command-link">Sessions</a>
          </div>
        </div>

        <div className="utility-card utility-card-stack">
          <div className="utility-card-header">
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              <span className="page-kicker" style={{ color: 'var(--color-text-muted)' }}>System health</span>
              <h2 className="section-heading">Runtime status</h2>
            </div>
            {statusUpdatedAt && (
              <span style={{ color: 'var(--color-text-muted)', fontSize: '11px', fontFamily: 'var(--font-mono)' }}>
                updated {secsAgo(statusUpdatedAt)}
              </span>
            )}
          </div>

          {statusError && (
            <div className="utility-note" style={{ background: 'rgba(255,68,68,0.06)', color: 'var(--color-error)', fontFamily: 'var(--font-mono)' }}>
              {statusError}
            </div>
          )}

          {!status && !statusError && (
            <span style={{ color: 'var(--color-text-muted)', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>loading…</span>
          )}

          {status && (
            <div className="utility-stat-grid">
              <div className="utility-stat-card">
                <div className="utility-stat-eyebrow" style={{ color: 'var(--color-text-muted)' }}>status</div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '7px', marginTop: '10px' }}>
                  <StatusDot ok={status.status === 'ok'} />
                  <span style={{
                    color: status.status === 'ok' ? 'var(--color-success)' : 'var(--color-gold)',
                    fontSize: '14px',
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 700,
                    textTransform: 'uppercase',
                  }}>
                    {status.status}
                  </span>
                </div>
              </div>
              <div className="utility-stat-card">
                <div className="utility-stat-eyebrow" style={{ color: 'var(--color-text-muted)' }}>version</div>
                <div className="utility-stat-value" style={{ fontSize: 'clamp(1.15rem, 1rem + 0.5vw, 1.5rem)', fontFamily: 'var(--font-mono)' }}>
                  {status.version}
                </div>
              </div>
              <div className="utility-stat-card">
                <div className="utility-stat-eyebrow" style={{ color: 'var(--color-text-muted)' }}>uptime</div>
                <div className="utility-stat-value" style={{ fontSize: 'clamp(1.15rem, 1rem + 0.5vw, 1.5rem)', fontFamily: 'var(--font-mono)' }}>
                  {formatUptime(status.uptime_secs)}
                </div>
              </div>
              <div className="utility-stat-card">
                <div className="utility-stat-eyebrow" style={{ color: 'var(--color-text-muted)' }}>active sessions</div>
                <div className="utility-stat-value">{status.sessions.active}</div>
              </div>
              <div className="utility-stat-card">
                <div className="utility-stat-eyebrow" style={{ color: 'var(--color-text-muted)' }}>total events</div>
                <div className="utility-stat-value">{status.sessions.total_events}</div>
              </div>
            </div>
          )}
        </div>

        {status && status.providers && status.providers.length > 0 && (
          <div className="utility-card utility-card-stack">
            <div className="utility-card-header">
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <span className="page-kicker" style={{ color: 'var(--color-text-muted)' }}>Provider availability</span>
                <h2 className="section-heading">LLM providers</h2>
              </div>
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(170px, 1fr))', gap: '12px' }}>
              {status.providers.map((provider) => (
                <div
                  key={provider.name}
                  className="utility-stat-card"
                  style={{
                    background: provider.available
                      ? 'rgba(0,255,136,0.07)'
                      : 'rgba(255,68,68,0.07)',
                  }}
                >
                  <span style={{
                    color: 'var(--color-text)',
                    fontSize: '13px',
                    fontFamily: 'var(--font-mono)',
                    fontWeight: 700,
                    letterSpacing: '0.04em',
                  }}>
                    {provider.name}
                  </span>
                  <div className="utility-stat-copy" style={{ marginTop: '6px', fontFamily: 'var(--font-mono)' }}>
                    bin: {provider.binary}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginTop: '10px' }}>
                    <span style={{
                      fontSize: '14px',
                      color: provider.available ? 'var(--color-success)' : 'var(--color-error)',
                      lineHeight: 1,
                    }}>
                      {provider.available ? '✓' : '✗'}
                    </span>
                    <span style={{
                      color: provider.available ? 'var(--color-success)' : 'var(--color-error)',
                      fontSize: '11px',
                      fontFamily: 'var(--font-mono)',
                      fontWeight: 600,
                      textTransform: 'uppercase',
                      letterSpacing: '0.04em',
                    }}>
                      {provider.available ? 'available' : 'unavailable'}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="utility-card utility-card-stack" style={{ flex: 1, minHeight: 0 }}>
          <div className="utility-card-header">
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              <span className="page-kicker" style={{ color: 'var(--color-text-muted)' }}>System events</span>
              <h2 className="section-heading">Event log</h2>
              <p className="section-copy" style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
                {eventsData ? `${eventsData.total} total events` : 'Polling every 5 seconds'}
                {eventsUpdatedAt ? ` · updated ${secsAgo(eventsUpdatedAt)}` : ''}
              </p>
            </div>

            <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
              <FilterBtn label="all" active={levelFilter === 'all'} color="var(--color-text-muted)" onClick={() => setLevelFilter('all')} />
              <FilterBtn label="error" active={levelFilter === 'error'} color="#ff4444" onClick={() => setLevelFilter('error')} />
              <FilterBtn label="warn" active={levelFilter === 'warn'} color="#ffd700" onClick={() => setLevelFilter('warn')} />
              <FilterBtn label="info" active={levelFilter === 'info'} color="#00d4ff" onClick={() => setLevelFilter('info')} />
            </div>
          </div>

          {eventsError && (
            <div className="utility-note" style={{ background: 'rgba(255,68,68,0.06)', color: 'var(--color-error)', fontFamily: 'var(--font-mono)' }}>
              {eventsError}
            </div>
          )}

          <div className="utility-scroll-surface" style={{ overflow: 'auto', maxHeight: '520px', fontFamily: 'var(--font-mono)' }}>
            {!eventsData && !eventsError && (
              <div style={{ padding: '24px', color: 'var(--color-text-muted)', fontSize: '12px', textAlign: 'center' }}>
                loading events…
              </div>
            )}

            {eventsData && filteredEvents.length === 0 && (
              <div style={{ padding: '32px', color: 'var(--color-text-muted)', fontSize: '12px', textAlign: 'center' }}>
                no events{levelFilter !== 'all' ? ` matching "${levelFilter}"` : ''}
              </div>
            )}

            {filteredEvents.length > 0 && (
              <div className="utility-scroll-list">
                {filteredEvents.map((event) => (
                  <EventRow key={event.id} event={event} />
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </Layout>
  );
}
