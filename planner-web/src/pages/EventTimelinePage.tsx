import { useEffect, useState, useMemo, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { BlueprintEventPayload, BlueprintEventType } from '../types/blueprint.ts';
import { buildKnowledgeDeepLink } from '../lib/knowledgeDeepLinks.ts';

// ─── Filter config ───────────────────────────────────────────────────────────

const EVENT_TYPES: { key: BlueprintEventType | 'all'; label: string }[] = [
  { key: 'all',            label: 'All Events' },
  { key: 'node_created',   label: 'Created' },
  { key: 'node_updated',   label: 'Updated' },
  { key: 'node_deleted',   label: 'Deleted' },
  { key: 'edge_created',   label: 'Edge Created' },
  { key: 'edges_deleted',  label: 'Edges Deleted' },
  { key: 'export_recorded', label: 'Exports' },
];

// ─── Page Component ──────────────────────────────────────────────────────────

export default function EventTimelinePage() {
  const navigate = useNavigate();
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [events, setEvents] = useState<BlueprintEventPayload[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterType, setFilterType] = useState<BlueprintEventType | 'all'>('all');
  const [limit, setLimit] = useState(100);
  const [activeSection, setActiveSection] = useState<'events' | 'snapshots'>('events');
  const [snapshots, setSnapshots] = useState<{ timestamp: string; filename: string }[]>([]);
  const [snapshotsLoading, setSnapshotsLoading] = useState(false);

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

  // ─── Snapshots loading ────────────────────────────────────────────────

  const loadSnapshots = useCallback(async () => {
    setSnapshotsLoading(true);
    try {
      const res = await api.listBlueprintHistory();
      setSnapshots(res.snapshots);
    } catch { setSnapshots([]); }
    finally { setSnapshotsLoading(false); }
  }, [api]);

  useEffect(() => {
    if (activeSection === 'snapshots') loadSnapshots();
  }, [activeSection, loadSnapshots]);

  // ─── Create snapshot ─────────────────────────────────────────────────

  const [creatingSnapshot, setCreatingSnapshot] = useState(false);

  const handleCreateSnapshot = useCallback(async () => {
    setCreatingSnapshot(true);
    try {
      await api.createBlueprintSnapshot();
      await loadSnapshots();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreatingSnapshot(false);
    }
  }, [api, loadSnapshots]);

  // ─── Filtering ─────────────────────────────────────────────────────────────

  const filtered = useMemo(() => {
    if (filterType === 'all') return events;
    return events.filter(e => e.event_type === filterType);
  }, [events, filterType]);

  const eventKnowledgeLink = useCallback((evt: BlueprintEventPayload): string | null => {
    const readString = (value: unknown): string | null => (
      typeof value === 'string' && value.trim().length > 0 ? value.trim() : null
    );
    const asRecord = (value: unknown): Record<string, unknown> | null => (
      value && typeof value === 'object' && !Array.isArray(value)
        ? value as Record<string, unknown>
        : null
    );

    const data = asRecord(evt.data) ?? {};
    let node: Record<string, unknown> | null = null;
    if (evt.event_type === 'node_created' || evt.event_type === 'node_deleted') {
      node = asRecord(data.node);
    } else if (evt.event_type === 'node_updated') {
      node = asRecord(data.after) ?? asRecord(data.before);
    }

    if (node) {
      const scope = asRecord(node.scope);
      const project = asRecord(scope?.project);
      const secondary = asRecord(scope?.secondary);
      const projectId = readString(project?.project_id);
      if (!projectId) return null;
      const nodeType = readString(node.node_type);
      const nodeName = readString(node.name);
      return buildKnowledgeDeepLink({
        projectId,
        feature: readString(secondary?.feature) ?? undefined,
        widget: readString(secondary?.widget) ?? undefined,
        artifact: readString(secondary?.artifact) ?? undefined,
        component: readString(secondary?.component) ?? (nodeType === 'component' ? nodeName ?? undefined : undefined),
        originPath: '/events',
        originLabel: 'Event Timeline',
      });
    }

    if (evt.event_type === 'export_recorded') {
      const projectId = readString(data.project_id);
      if (!projectId) return null;
      return buildKnowledgeDeepLink({
        projectId,
        originPath: '/events',
        originLabel: 'Event Timeline',
      });
    }

    return null;
  }, []);

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
            {activeSection === 'events'
              ? `All changes across the blueprint — ${filtered.length} event${filtered.length !== 1 ? 's' : ''}`
              : `${snapshots.length} snapshot${snapshots.length !== 1 ? 's' : ''} saved`
            }
          </p>
        </div>
        <div style={{ display: 'flex', gap: 'var(--space-2)', alignItems: 'center' }}>
          {activeSection === 'snapshots' && (
            <button className="btn btn-primary" onClick={handleCreateSnapshot} disabled={creatingSnapshot}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <path d="M19 21H5a2 2 0 01-2-2V5a2 2 0 012-2h11l5 5v11a2 2 0 01-2 2z"/>
                <polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/>
              </svg>
              {creatingSnapshot ? 'Creating…' : 'Create Snapshot'}
            </button>
          )}
          <button className="btn btn-outline" onClick={activeSection === 'events' ? loadEvents : loadSnapshots} disabled={loading || snapshotsLoading}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/>
            </svg>
            Refresh
          </button>
        </div>
      </div>

      {/* Section tabs */}
      <div className="drawer-tabs" style={{ marginBottom: 'var(--space-4)' }}>
        <button
          className={`drawer-tab${activeSection === 'events' ? ' active' : ''}`}
          onClick={() => setActiveSection('events')}
        >
          Events
        </button>
        <button
          className={`drawer-tab${activeSection === 'snapshots' ? ' active' : ''}`}
          onClick={() => setActiveSection('snapshots')}
        >
          Snapshots
        </button>
      </div>

      {/* Events section */}
      {activeSection === 'events' && (
        <>

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
          {filtered.map((evt, i) => {
            const relatedKnowledgeLink = eventKnowledgeLink(evt);
            const before = evt.event_type === 'node_updated' && evt.data?.before
              ? evt.data.before as Record<string, unknown> : null;
            const after = evt.event_type === 'node_updated' && evt.data?.after
              ? evt.data.after as Record<string, unknown> : null;
            const hasDiff = before !== null && after !== null;
            const diffKeys = hasDiff
              ? [...new Set([...Object.keys(before!), ...Object.keys(after!)])].filter(
                  k => JSON.stringify(before![k]) !== JSON.stringify(after![k])
                )
              : [];

            return (
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
                {relatedKnowledgeLink && (
                  <div style={{ marginTop: 'var(--space-2)' }}>
                    <button
                      className="btn btn-outline"
                      style={{ fontSize: '0.625rem', padding: 'var(--space-1) var(--space-2)' }}
                      onClick={() => navigate(relatedKnowledgeLink)}
                    >
                      View related knowledge
                    </button>
                  </div>
                )}

                {/* Diff view for node_updated events */}
                {hasDiff && diffKeys.length > 0 && (
                  <details className="event-timeline-data">
                    <summary>{diffKeys.length} field{diffKeys.length !== 1 ? 's' : ''} changed</summary>
                    <div className="diff-view">
                      <div className="diff-panel diff-panel-before">
                        <div className="diff-panel-header">Before</div>
                        {diffKeys.map(k => (
                          <div key={k} className="diff-row diff-removed">
                            <span className="diff-key">{k}</span>
                            <span className="diff-value" title={String(before![k] ?? '')}>
                              {typeof before![k] === 'object' ? JSON.stringify(before![k]) : String(before![k] ?? '—')}
                            </span>
                          </div>
                        ))}
                      </div>
                      <div className="diff-panel diff-panel-after">
                        <div className="diff-panel-header">After</div>
                        {diffKeys.map(k => (
                          <div key={k} className="diff-row diff-added">
                            <span className="diff-key">{k}</span>
                            <span className="diff-value" title={String(after![k] ?? '')}>
                              {typeof after![k] === 'object' ? JSON.stringify(after![k]) : String(after![k] ?? '—')}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  </details>
                )}

                {/* Fallback raw JSON for non-update events */}
                {!hasDiff && evt.data && Object.keys(evt.data).length > 0 && (
                  <details className="event-timeline-data">
                    <summary>Details</summary>
                    <pre>{JSON.stringify(evt.data, null, 2)}</pre>
                  </details>
                )}
              </div>
            </div>
            );
          })}
        </div>
      )}

        </>
      )}

      {/* Snapshots section */}
      {activeSection === 'snapshots' && (
        <>
          {snapshotsLoading && (
            <div style={{ display: 'flex', justifyContent: 'center', padding: 'var(--space-8)' }}>
              <div className="skeleton-pulse" />
            </div>
          )}
          {!snapshotsLoading && snapshots.length === 0 && (
            <div className="empty-state">
              <p style={{ color: 'var(--color-text-faint)' }}>No snapshots saved yet.</p>
              <p style={{ color: 'var(--color-text-faint)', fontSize: 'var(--text-xs)' }}>
                Snapshots are automatically created when the blueprint is saved.
              </p>
            </div>
          )}
          {!snapshotsLoading && snapshots.length > 0 && (
            <div className="snapshot-list">
              {snapshots.map((snap, i) => (
                <div key={i} className="snapshot-item">
                  <div className="snapshot-icon">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--color-primary)" strokeWidth="2" strokeLinecap="round">
                      <path d="M19 21H5a2 2 0 01-2-2V5a2 2 0 012-2h11l5 5v11a2 2 0 01-2 2z"/>
                      <polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/>
                    </svg>
                  </div>
                  <div className="snapshot-info">
                    <div className="snapshot-timestamp">
                      {new Date(snap.timestamp).toLocaleString()}
                    </div>
                    <div className="snapshot-filename">{snap.filename}</div>
                  </div>
                  <div className="snapshot-age">
                    {relativeTime(snap.timestamp)}
                  </div>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </Layout>
  );
}
