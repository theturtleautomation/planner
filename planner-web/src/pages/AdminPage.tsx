import { useEffect, useState, useMemo, useCallback } from 'react';
import { Link } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { AdminStatusResponse, AdminEventsResponse, AdminEventEntry } from '../types.ts';

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
      border: `1px solid ${s.border}`,
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
      border: '1px solid rgba(136,136,160,0.25)',
      background: 'rgba(136,136,160,0.07)',
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
        border: active ? `1px solid ${color}` : '1px solid var(--color-border)',
        color: active ? color : 'var(--color-text-muted)',
        padding: '3px 12px',
        fontSize: '11px',
        fontWeight: 600,
        cursor: 'pointer',
        borderRadius: '2px',
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
  return (
    <div style={{
      display: 'flex',
      alignItems: 'flex-start',
      gap: '8px',
      padding: '7px 12px',
      borderBottom: '1px solid rgba(42,42,62,0.5)',
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
      <div style={{
        flex: 1,
        overflow: 'auto',
        padding: '28px 24px',
        display: 'flex',
        flexDirection: 'column',
        gap: '24px',
        maxWidth: '1000px',
        margin: '0 auto',
        width: '100%',
      }}>

        {/* ── Header ── */}
        <div style={{
          display: 'flex',
          alignItems: 'baseline',
          justifyContent: 'space-between',
          borderBottom: '1px solid var(--color-border)',
          paddingBottom: '12px',
          gap: '16px',
        }}>
          <span style={{
            color: 'var(--color-primary)',
            fontSize: '14px',
            fontWeight: 700,
            fontFamily: 'monospace',
            letterSpacing: '0.08em',
          }}>
            system administration
          </span>
          <a
            href="/"
            style={{
              color: 'var(--color-text-muted)',
              fontSize: '12px',
              textDecoration: 'none',
              fontFamily: 'monospace',
              transition: 'color 0.18s',
              flexShrink: 0,
            }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLAnchorElement).style.color = 'var(--color-primary)'; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLAnchorElement).style.color = 'var(--color-text-muted)'; }}
          >
            ← dashboard
          </a>
        </div>

        {/* ── System Health Card ── */}
        <div style={{
          background: 'var(--color-surface)',
          border: '1px solid var(--color-border)',
          borderRadius: '3px',
          padding: '16px 20px',
          display: 'flex',
          flexDirection: 'column',
          gap: '12px',
        }}>
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            borderBottom: '1px solid var(--color-border)',
            paddingBottom: '10px',
          }}>
            <span style={{ color: 'var(--color-text-muted)', fontSize: '11px', letterSpacing: '0.08em', fontFamily: 'monospace' }}>
              system health
            </span>
            {statusUpdatedAt && (
              <span style={{ color: 'var(--color-text-muted)', fontSize: '10px', opacity: 0.5, marginLeft: 'auto', fontFamily: 'monospace' }}>
                updated {secsAgo(statusUpdatedAt)}
              </span>
            )}
          </div>

          {statusError && (
            <div style={{
              color: 'var(--color-error)',
              fontSize: '12px',
              fontFamily: 'monospace',
              padding: '8px 12px',
              background: 'rgba(255,68,68,0.06)',
              border: '1px solid rgba(255,68,68,0.25)',
              borderRadius: '2px',
            }}>
              {statusError}
            </div>
          )}

          {!status && !statusError && (
            <span style={{ color: 'var(--color-text-muted)', fontSize: '12px' }}>loading…</span>
          )}

          {status && (
            <div style={{ display: 'flex', flexWrap: 'wrap' as const, gap: '20px 40px' }}>
              {/* Status */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '10px', letterSpacing: '0.06em', fontFamily: 'monospace' }}>status</span>
                <div style={{ display: 'flex', alignItems: 'center', gap: '7px' }}>
                  <StatusDot ok={status.status === 'ok'} />
                  <span style={{
                    color: status.status === 'ok' ? 'var(--color-success)' : 'var(--color-gold)',
                    fontSize: '13px',
                    fontFamily: 'monospace',
                    fontWeight: 700,
                  }}>
                    {status.status}
                  </span>
                </div>
              </div>

              {/* Version */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '10px', letterSpacing: '0.06em', fontFamily: 'monospace' }}>version</span>
                <span style={{ color: 'var(--color-text)', fontSize: '13px', fontFamily: 'monospace', fontWeight: 600 }}>
                  {status.version}
                </span>
              </div>

              {/* Uptime */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '10px', letterSpacing: '0.06em', fontFamily: 'monospace' }}>uptime</span>
                <span style={{ color: 'var(--color-text)', fontSize: '13px', fontFamily: 'monospace', fontWeight: 600 }}>
                  {formatUptime(status.uptime_secs)}
                </span>
              </div>

              {/* Active sessions */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '10px', letterSpacing: '0.06em', fontFamily: 'monospace' }}>active sessions</span>
                <span style={{ color: 'var(--color-text)', fontSize: '13px', fontFamily: 'monospace', fontWeight: 600 }}>
                  {status.sessions.active}
                </span>
              </div>

              {/* Total events */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <span style={{ color: 'var(--color-text-muted)', fontSize: '10px', letterSpacing: '0.06em', fontFamily: 'monospace' }}>total events</span>
                <span style={{ color: 'var(--color-text)', fontSize: '13px', fontFamily: 'monospace', fontWeight: 600 }}>
                  {status.sessions.total_events}
                </span>
              </div>
            </div>
          )}
        </div>

        {/* ── Provider Cards ── */}
        {status && status.providers && status.providers.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <span style={{ color: 'var(--color-text-muted)', fontSize: '11px', letterSpacing: '0.08em', fontFamily: 'monospace' }}>
              llm providers
            </span>
            <div style={{ display: 'flex', gap: '12px', flexWrap: 'wrap' as const }}>
              {status.providers.map((provider) => (
                <div
                  key={provider.name}
                  style={{
                    background: 'var(--color-surface)',
                    border: provider.available
                      ? '1px solid rgba(0,255,136,0.25)'
                      : '1px solid rgba(255,68,68,0.25)',
                    borderRadius: '3px',
                    padding: '12px 18px',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '6px',
                    minWidth: '150px',
                    flex: '1 1 150px',
                    maxWidth: '220px',
                  }}
                >
                  {/* Provider name */}
                  <span style={{
                    color: 'var(--color-text)',
                    fontSize: '13px',
                    fontFamily: 'monospace',
                    fontWeight: 700,
                    letterSpacing: '0.04em',
                  }}>
                    {provider.name}
                  </span>

                  {/* Binary */}
                  <span style={{
                    color: 'var(--color-text-muted)',
                    fontSize: '11px',
                    fontFamily: 'monospace',
                    opacity: 0.75,
                  }}>
                    bin: {provider.binary}
                  </span>

                  {/* Status */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginTop: '2px' }}>
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
                      fontFamily: 'monospace',
                      fontWeight: 600,
                    }}>
                      {provider.available ? 'available' : 'unavailable'}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ── Event Log ── */}
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '10px',
          flex: 1,
          minHeight: 0,
        }}>
          {/* Log header */}
          <div style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '12px',
            flexWrap: 'wrap' as const,
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <span style={{ color: 'var(--color-text-muted)', fontSize: '11px', letterSpacing: '0.08em', fontFamily: 'monospace' }}>
                event log
              </span>
              {eventsData && (
                <span style={{
                  color: 'var(--color-text-muted)',
                  fontSize: '10px',
                  fontFamily: 'monospace',
                  opacity: 0.55,
                }}>
                  ({eventsData.total} total)
                </span>
              )}
              {eventsUpdatedAt && (
                <span style={{ color: 'var(--color-text-muted)', fontSize: '10px', opacity: 0.45, fontFamily: 'monospace' }}>
                  · updated {secsAgo(eventsUpdatedAt)}
                </span>
              )}
            </div>

            {/* Filter buttons */}
            <div style={{ display: 'flex', gap: '6px' }}>
              <FilterBtn label="all" active={levelFilter === 'all'} color="var(--color-text-muted)" onClick={() => setLevelFilter('all')} />
              <FilterBtn label="error" active={levelFilter === 'error'} color="#ff4444" onClick={() => setLevelFilter('error')} />
              <FilterBtn label="warn" active={levelFilter === 'warn'} color="#ffd700" onClick={() => setLevelFilter('warn')} />
              <FilterBtn label="info" active={levelFilter === 'info'} color="#00d4ff" onClick={() => setLevelFilter('info')} />
            </div>
          </div>

          {/* Error banner */}
          {eventsError && (
            <div style={{
              color: 'var(--color-error)',
              fontSize: '12px',
              fontFamily: 'monospace',
              padding: '8px 12px',
              background: 'rgba(255,68,68,0.06)',
              border: '1px solid rgba(255,68,68,0.25)',
              borderRadius: '2px',
            }}>
              {eventsError}
            </div>
          )}

          {/* Scrollable list */}
          <div style={{
            background: 'var(--color-surface)',
            border: '1px solid var(--color-border)',
            borderRadius: '3px',
            overflow: 'auto',
            maxHeight: '480px',
            fontFamily: 'monospace',
          }}>
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

            {filteredEvents.map((event) => (
              <EventRow key={event.id} event={event} />
            ))}
          </div>
        </div>

      </div>
    </Layout>
  );
}
