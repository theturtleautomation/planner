import { useEffect, useState, useMemo, useCallback } from 'react';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { ProposedNode, ProposalStatus, DiscoverySource } from '../types/blueprint.ts';

// ─── Constants ───────────────────────────────────────────────────────────────

const STATUS_FILTERS: { key: ProposalStatus | 'all'; label: string }[] = [
  { key: 'all',      label: 'All' },
  { key: 'pending',  label: 'Pending' },
  { key: 'accepted', label: 'Accepted' },
  { key: 'rejected', label: 'Rejected' },
  { key: 'merged',   label: 'Merged' },
];

const SOURCE_ICONS: Record<DiscoverySource, string> = {
  cargo_toml: '📦',
  directory_scan: '📁',
  pipeline_run: '⚡',
  manual: '✏️',
};

const SOURCE_LABELS: Record<DiscoverySource, string> = {
  cargo_toml: 'Cargo.toml',
  directory_scan: 'Directory Scan',
  pipeline_run: 'Pipeline',
  manual: 'Manual',
};

// ─── Component ───────────────────────────────────────────────────────────────

export default function DiscoveryPage() {
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [proposals, setProposals] = useState<ProposedNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterStatus, setFilterStatus] = useState<ProposalStatus | 'all'>('pending');
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [scanResult, setScanResult] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // ─── Load proposals ────────────────────────────────────────────────────

  const loadProposals = useCallback(async () => {
    setLoading(true);
    try {
      const statusParam = filterStatus === 'all' ? undefined : filterStatus;
      const res = await api.listProposedNodes(statusParam);
      setProposals(res.proposals);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api, filterStatus]);

  useEffect(() => { loadProposals(); }, [loadProposals]);

  // ─── Actions ───────────────────────────────────────────────────────────

  const handleScan = useCallback(async () => {
    setScanning(true);
    setScanError(null);
    setScanResult(null);
    try {
      const res = await api.runDiscoveryScan({ scanners: ['all'] });
      const summary = res.results
        .map(r => `${r.scanner}: ${r.proposed_count} proposed, ${r.skipped_count} skipped`)
        .join(' · ');
      setScanResult(`Scan complete — ${res.total_proposed} new proposals. ${summary}`);
      loadProposals();
    } catch (err) {
      setScanError(err instanceof Error ? err.message : String(err));
    } finally {
      setScanning(false);
    }
  }, [api, loadProposals]);

  const handleAccept = useCallback(async (id: string) => {
    setActionLoading(id);
    try {
      await api.acceptProposal(id);
      setProposals(prev => prev.map(p => p.id === id ? { ...p, status: 'accepted' as const, reviewed_at: new Date().toISOString() } : p));
    } catch { /* keep current state */ }
    finally { setActionLoading(null); }
  }, [api]);

  const handleReject = useCallback(async (id: string) => {
    setActionLoading(id);
    try {
      await api.rejectProposal(id);
      setProposals(prev => prev.map(p => p.id === id ? { ...p, status: 'rejected' as const, reviewed_at: new Date().toISOString() } : p));
    } catch { /* keep current state */ }
    finally { setActionLoading(null); }
  }, [api]);

  // ─── Helpers ───────────────────────────────────────────────────────────

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

  const getNodeDisplayName = (p: ProposedNode): string => {
    const n = p.node as unknown as Record<string, unknown>;
    return (n.name ?? n.title ?? n.scenario ?? p.id) as string;
  };

  const confidenceColor = (c: number) => {
    if (c >= 0.8) return 'var(--color-success)';
    if (c >= 0.5) return 'var(--color-warning)';
    return 'var(--color-text-faint)';
  };

  const pendingCount = proposals.filter(p => p.status === 'pending').length;

  // ─── Render ────────────────────────────────────────────────────────────

  return (
    <Layout>
      <div className="page-header">
        <div>
          <h1 className="page-title">Automated Discovery</h1>
          <p className="page-subtitle">
            Scan project artifacts to discover technologies, components, and patterns.
            {pendingCount > 0 && ` ${pendingCount} pending review.`}
          </p>
        </div>
        <div style={{ display: 'flex', gap: 'var(--space-2)', alignItems: 'center' }}>
          <button className="btn btn-outline" onClick={loadProposals} disabled={loading}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/>
            </svg>
            Refresh
          </button>
          <button className="btn btn-primary" onClick={handleScan} disabled={scanning}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
            </svg>
            {scanning ? 'Scanning…' : 'Run Discovery Scan'}
          </button>
        </div>
      </div>

      {/* Scan feedback */}
      {scanResult && (
        <div style={{
          padding: 'var(--space-3) var(--space-4)',
          background: 'color-mix(in srgb, var(--color-success) 10%, transparent)',
          border: '1px solid color-mix(in srgb, var(--color-success) 30%, transparent)',
          borderRadius: 'var(--radius-md)',
          fontSize: 'var(--text-sm)',
          color: 'var(--color-success)',
          marginBottom: 'var(--space-4)',
        }}>
          {scanResult}
        </div>
      )}
      {scanError && (
        <div style={{
          padding: 'var(--space-3) var(--space-4)',
          background: 'color-mix(in srgb, var(--color-error) 10%, transparent)',
          border: '1px solid color-mix(in srgb, var(--color-error) 30%, transparent)',
          borderRadius: 'var(--radius-md)',
          fontSize: 'var(--text-sm)',
          color: 'var(--color-error)',
          marginBottom: 'var(--space-4)',
        }}>
          Scan failed: {scanError}
        </div>
      )}

      {/* Status filter chips */}
      <div className="event-filters" style={{ marginBottom: 'var(--space-4)' }}>
        <div className="event-filter-group">
          {STATUS_FILTERS.map(f => (
            <button
              key={f.key}
              className={`event-filter-chip${filterStatus === f.key ? ' active' : ''}`}
              onClick={() => setFilterStatus(f.key)}
            >
              {f.label}
            </button>
          ))}
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="empty-state" style={{ color: 'var(--color-error)' }}>
          <p>Failed to load proposals: {error}</p>
          <button className="btn btn-outline" onClick={loadProposals}>Retry</button>
        </div>
      )}

      {/* Loading */}
      {loading && (
        <div style={{ display: 'flex', justifyContent: 'center', padding: 'var(--space-8)' }}>
          <div className="skeleton-pulse" />
        </div>
      )}

      {/* Empty */}
      {!loading && !error && proposals.length === 0 && (
        <div className="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-faint)" strokeWidth="1.5" strokeLinecap="round" style={{ opacity: 0.4 }}>
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <p style={{ color: 'var(--color-text-faint)', marginTop: 'var(--space-3)' }}>
            {filterStatus === 'all'
              ? 'No proposals yet. Run a discovery scan to detect technologies and components from your project.'
              : `No ${filterStatus} proposals.`
            }
          </p>
        </div>
      )}

      {/* Proposal list */}
      {!loading && !error && proposals.length > 0 && (
        <div className="snapshot-list">
          {proposals.map(p => (
            <div
              key={p.id}
              className="snapshot-item"
              style={{
                flexDirection: 'column',
                alignItems: 'stretch',
                cursor: 'pointer',
                borderColor: p.status === 'pending' ? 'color-mix(in srgb, var(--color-warning) 40%, var(--color-border))' : undefined,
              }}
              onClick={() => setExpandedId(expandedId === p.id ? null : p.id)}
            >
              {/* Summary row */}
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)', width: '100%' }}>
                <span style={{ fontSize: '1.2em' }} title={SOURCE_LABELS[p.source]}>
                  {SOURCE_ICONS[p.source]}
                </span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
                    <span style={{ fontWeight: 600, fontSize: 'var(--text-sm)' }}>
                      {getNodeDisplayName(p)}
                    </span>
                    <span className={`badge badge-${p.node.node_type}`} style={{ fontSize: '0.5625rem' }}>
                      {p.node.node_type}
                    </span>
                    <span style={{
                      fontSize: '0.5625rem',
                      fontWeight: 600,
                      textTransform: 'uppercase',
                      letterSpacing: '0.08em',
                      color: p.status === 'pending' ? 'var(--color-warning)' :
                             p.status === 'accepted' ? 'var(--color-success)' :
                             p.status === 'rejected' ? 'var(--color-error)' : 'var(--color-primary)',
                    }}>
                      {p.status}
                    </span>
                  </div>
                  <div style={{
                    fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)',
                    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                  }}>
                    {p.reason}
                  </div>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)', flexShrink: 0 }}>
                  <span style={{
                    fontFamily: 'var(--font-mono)', fontSize: '0.625rem',
                    fontWeight: 600, color: confidenceColor(p.confidence),
                  }}>
                    {Math.round(p.confidence * 100)}%
                  </span>
                  <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)', whiteSpace: 'nowrap' }}>
                    {relativeTime(p.proposed_at)}
                  </span>
                </div>
              </div>

              {/* Expanded details */}
              {expandedId === p.id && (
                <div style={{
                  marginTop: 'var(--space-3)',
                  paddingTop: 'var(--space-3)',
                  borderTop: '1px solid var(--color-divider)',
                }}
                  onClick={e => e.stopPropagation()}
                >
                  {p.source_artifact && (
                    <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)', marginBottom: 'var(--space-2)' }}>
                      <strong>Source:</strong>{' '}
                      <span style={{ fontFamily: 'var(--font-mono)' }}>{p.source_artifact}</span>
                    </div>
                  )}
                  <details>
                    <summary style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', cursor: 'pointer' }}>
                      Full node data
                    </summary>
                    <pre style={{
                      fontSize: '0.625rem', lineHeight: 1.4,
                      padding: 'var(--space-2)', marginTop: 'var(--space-1)',
                      background: 'var(--color-surface-offset)', borderRadius: 'var(--radius-sm)',
                      overflow: 'auto', maxHeight: '200px',
                    }}>
                      {JSON.stringify(p.node, null, 2)}
                    </pre>
                  </details>

                  {p.status === 'pending' && (
                    <div style={{ display: 'flex', gap: 'var(--space-2)', marginTop: 'var(--space-3)' }}>
                      <button
                        className="btn btn-primary"
                        style={{ fontSize: 'var(--text-xs)', padding: 'var(--space-1) var(--space-3)' }}
                        onClick={() => handleAccept(p.id)}
                        disabled={actionLoading === p.id}
                      >
                        {actionLoading === p.id ? '…' : '✓ Accept'}
                      </button>
                      <button
                        className="btn btn-outline"
                        style={{
                          fontSize: 'var(--text-xs)', padding: 'var(--space-1) var(--space-3)',
                          color: 'var(--color-error)', borderColor: 'var(--color-error)',
                        }}
                        onClick={() => handleReject(p.id)}
                        disabled={actionLoading === p.id}
                      >
                        {actionLoading === p.id ? '…' : '✗ Reject'}
                      </button>
                    </div>
                  )}

                  {p.reviewed_at && (
                    <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)', marginTop: 'var(--space-2)' }}>
                      Reviewed {relativeTime(p.reviewed_at)}
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </Layout>
  );
}
