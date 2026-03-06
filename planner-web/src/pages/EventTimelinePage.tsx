import { useEffect, useState, useMemo, useCallback } from 'react';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { BlueprintEventPayload, BlueprintEventType } from '../types/blueprint.ts';

// ─── Filter config ───────────────────────────────────────────────────────────

const EVENT_TYPES: { key: BlueprintEventType | 'all'; label: string }[] = [
  { key: 'all',            label: 'All Events' },
  { key: 'node_created',   label: 'Created' },
  { key: 'node_updated',   label: 'Updated' },
  { key: 'node_deleted',   label: 'Deleted' },
  { key: 'edge_created',   label: 'Edge Created' },
  { key: 'edges_deleted',  label: 'Edges Deleted' },
];

// ─── Page Component ──────────────────────────────────────────────────────────

export default function EventTimelinePage() {
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [events, setEvents] = useState<BlueprintEventPayload[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterType, setFilterType] = useState<BlueprintEventType | 'all'>('all');
  const [limit, setLimit] = useState(100);

  // ─── Data loading ──────────────────────────────────────────────────────────

  const loadEvents = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.listBlueprintEvents({ limit });
      setEvents(res.events);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api, limit]);

  useEffect(() => { loadEvents(); }, [loadEvents]);

  // ─── Filtering ─────────────────────────────────────────────────────────────

  const filtered = useMemo(() => {
    if (filterType === 'all') return events;
    return events.filter(e => e.event_type === filterType);
  }, [events, filterType]);

  // ─── Relative time display ─────────────────────────────────────────────────

  const relativeTime = (ts: string) => {
    const diff = Date.now() - new Date(ts).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    if (days < 30) return `${days}d ago`;
    return new Date(ts).toLocaleDateString();
  };

  // ─── Render ────────────────────────────────────────────────────────────────

  return (
    <Layout>
      <div className="page-header">
        <div>
          <h1 className="page-title">Event Timeline</h1>
          <p className="page-subtitle">
            All changes across the blueprint — {filtered.length} event{filtered.length !== 1 ? 's' : ''}
          </p>
        </div>
        <div style={{ display: 'flex', gap: 'var(--space-2)', alignItems: 'center' }}>
          <button className="btn btn-outline" onClick={loadEvents} disabled={loading}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/>
            </svg>
            Refresh
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="event-filters">
        <div className="event-filter-group">
          {EVENT_TYPES.map(t => (
            <button
              key={t.key}
              className={`event-filter-chip${filterType === t.key ? ' active' : ''}`}
              onClick={() => setFilterType(t.key)}
            >
              {t.label}
            </button>
          ))}
        </div>
        <select
          className="event-limit-select"
          value={limit}
          onChange={e => setLimit(Number(e.target.value))}
        >
          <option value={50}>Last 50</option>
          <option value={100}>Last 100</option>
          <option value={250}>Last 250</option>
          <option value={500}>Last 500</option>
        </select>
      </div>

      {/* Error */}
      {error && (
        <div className="empty-state" style={{ color: 'var(--color-error)' }}>
          <p>Failed to load events: {error}</p>
          <button className="btn btn-outline" onClick={loadEvents}>Retry</button>
        </div>
      )}

      {/* Loading */}
      {loading && (
        <div style={{ display: 'flex', justifyContent: 'center', padding: 'var(--space-8)' }}>
          <div className="skeleton-pulse" />
        </div>
      )}

      {/* Empty */}
      {!loading && !error && filtered.length === 0 && (
        <div className="empty-state">
          <p style={{ color: 'var(--color-text-faint)' }}>
            {filterType === 'all' ? 'No events recorded yet.' : `No ${filterType.replace(/_/g, ' ')} events found.`}
          </p>
        </div>
      )}

      {/* Event list */}
      {!loading && !error && filtered.length > 0 && (
        <div className="global-event-timeline">
          {filtered.map((evt, i) => (
            <div key={i} className="global-event-item">
              <div className="global-event-left">
                <div className={`event-timeline-dot event-dot-${evt.event_type}`} />
                {i < filtered.length - 1 && <div className="event-timeline-line" />}
              </div>
              <div className="global-event-body">
                <div className="global-event-meta">
                  <span className={`event-type-badge event-badge-${evt.event_type}`}>
                    {evt.event_type.replace(/_/g, ' ')}
                  </span>
                  <span className="event-timeline-time" title={new Date(evt.timestamp).toLocaleString()}>
                    {relativeTime(evt.timestamp)}
                  </span>
                </div>
                <div className="global-event-summary">{evt.summary}</div>
                {evt.data && Object.keys(evt.data).length > 0 && (
                  <details className="event-timeline-data">
                    <summary>Details</summary>
                    <pre>{JSON.stringify(evt.data, null, 2)}</pre>
                  </details>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </Layout>
  );
}
