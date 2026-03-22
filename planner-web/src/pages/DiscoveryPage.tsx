import { useEffect, useState, useMemo, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import { buildKnowledgeDeepLink } from '../lib/knowledgeDeepLinks.ts';
import type { ProposedNode, ProposedEdge, ProposalStatus, DiscoverySource } from '../types/blueprint.ts';

// ─── Constants ───────────────────────────────────────────────────────────────

const STATUS_FILTERS: { key: ProposalStatus | 'all'; label: string }[] = [
  { key: 'all',      label: 'All' },
  { key: 'pending',  label: 'Pending' },
  { key: 'accepted', label: 'Accepted' },
  { key: 'rejected', label: 'Rejected' },
  { key: 'merged',   label: 'Merged' },
];

const PROPOSAL_VIEWS: { key: 'nodes' | 'edges'; label: string }[] = [
  { key: 'nodes', label: 'Node Proposals' },
  { key: 'edges', label: 'Edge Proposals' },
];

const SOURCE_ICONS: Record<DiscoverySource, string> = {
  cargo_toml: '📦',
  directory_scan: '📁',
  pipeline_run: '⚡',
  manual: '✏️',
  code_graph_context: '🕸️',
};

const SOURCE_LABELS: Record<DiscoverySource, string> = {
  cargo_toml: 'Cargo.toml',
  directory_scan: 'Directory Scan',
  pipeline_run: 'Pipeline',
  manual: 'Manual',
  code_graph_context: 'Code Graph',
};

// ─── Component ───────────────────────────────────────────────────────────────

export default function DiscoveryPage() {
  const navigate = useNavigate();
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [proposalView, setProposalView] = useState<'nodes' | 'edges'>('nodes');
  const [nodeProposals, setNodeProposals] = useState<ProposedNode[]>([]);
  const [edgeProposals, setEdgeProposals] = useState<ProposedEdge[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filterStatus, setFilterStatus] = useState<ProposalStatus | 'all'>('pending');
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [scanResult, setScanResult] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [proposalNameOverrides, setProposalNameOverrides] = useState<Record<string, string>>({});

  // ─── Load proposals ────────────────────────────────────────────────────

  const loadProposals = useCallback(async () => {
    setLoading(true);
    try {
      const statusParam = filterStatus === 'all' ? undefined : filterStatus;
      if (proposalView === 'nodes') {
        const res = await api.listProposedNodes(statusParam);
        setNodeProposals(res.proposals);
      } else {
        const res = await api.listProposedEdges(statusParam);
        setEdgeProposals(res.proposals);
      }
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api, filterStatus, proposalView]);

  useEffect(() => { loadProposals(); }, [loadProposals]);

  // ─── Actions ───────────────────────────────────────────────────────────

  const handleScan = useCallback(async () => {
    setScanning(true);
    setScanError(null);
    setScanResult(null);
    try {
      const res = await api.runDiscoveryScan({ scanners: ['all'] });
      const summary = res.results
        .map((r) => {
          if (r.proposed_edge_count > 0 || r.skipped_edge_count > 0) {
            return `${r.scanner}: ${r.proposed_count} nodes, ${r.skipped_count} node skips, ${r.proposed_edge_count} edges, ${r.skipped_edge_count} edge skips`;
          }
          return `${r.scanner}: ${r.proposed_count} proposed, ${r.skipped_count} skipped`;
        })
        .join(' · ');
      setScanResult(
        `Scan complete — ${res.total_proposed} node proposals, ${res.total_edge_proposed} edge proposals. ${summary}`,
      );
      loadProposals();
    } catch (err) {
      setScanError(err instanceof Error ? err.message : String(err));
    } finally {
      setScanning(false);
    }
  }, [api, loadProposals]);

  const handleAccept = useCallback(async (proposal: ProposedNode) => {
    const id = proposal.id;
    setActionLoading(id);
    try {
      const suggested = proposal.node.node_type === 'component' ? proposal.node.name.trim() : '';
      const override = proposalNameOverrides[id]?.trim() ?? '';
      const shouldMarkManual = proposal.node.node_type === 'component' && override && override !== suggested;
      await api.acceptProposal(
        id,
        shouldMarkManual
          ? {
              node_patch: {
                name: override,
                naming: {
                  source: 'manual',
                },
              },
            }
          : undefined,
      );

      setNodeProposals(prev => prev.map(p => {
        if (p.id !== id) return p;
        if (!shouldMarkManual || p.node.node_type !== 'component') {
          return { ...p, status: 'accepted' as const, reviewed_at: new Date().toISOString() };
        }
        return {
          ...p,
          status: 'accepted' as const,
          reviewed_at: new Date().toISOString(),
          node: {
            ...p.node,
            name: override,
            naming: {
              ...(p.node.naming ?? {
                origin_key: `manual:${p.node.id}`,
                source: 'generated' as const,
                strategy: 'manual_create' as const,
                generated_name: suggested,
                naming_version: 1,
                last_generated_at: new Date().toISOString(),
              }),
              source: 'manual',
            },
          },
        };
      }));
    } catch { /* keep current state */ }
    finally { setActionLoading(null); }
  }, [api, proposalNameOverrides]);

  const handleReject = useCallback(async (id: string) => {
    setActionLoading(id);
    try {
      await api.rejectProposal(id);
      setNodeProposals(prev => prev.map(p => p.id === id ? { ...p, status: 'rejected' as const, reviewed_at: new Date().toISOString() } : p));
    } catch { /* keep current state */ }
    finally { setActionLoading(null); }
  }, [api]);

  const handleAcceptEdge = useCallback(async (proposal: ProposedEdge) => {
    const id = proposal.id;
    setActionLoading(id);
    try {
      await api.acceptEdgeProposal(id);
      setEdgeProposals(prev => prev.map(p => p.id === id ? { ...p, status: 'merged' as const, reviewed_at: new Date().toISOString() } : p));
    } catch { /* keep current state */ }
    finally { setActionLoading(null); }
  }, [api]);

  const handleRejectEdge = useCallback(async (id: string) => {
    setActionLoading(id);
    try {
      await api.rejectEdgeProposal(id);
      setEdgeProposals(prev => prev.map(p => p.id === id ? { ...p, status: 'rejected' as const, reviewed_at: new Date().toISOString() } : p));
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
    return (n.name ?? n.title ?? n.label ?? n.scenario ?? p.id) as string;
  };

  const getRelatedKnowledgeLink = (proposal: ProposedNode): string | null => {
    const projectId = proposal.node.scope.project?.project_id?.trim();
    if (!projectId) return null;

    const secondary = proposal.node.scope.secondary ?? {};
    const component = secondary.component?.trim()
      || (proposal.node.node_type === 'component' ? proposal.node.name.trim() : undefined);

    return buildKnowledgeDeepLink({
      projectId,
      feature: secondary.feature,
      widget: secondary.widget,
      artifact: secondary.artifact,
      component,
      originPath: '/discovery',
      originLabel: 'Discovery',
    });
  };

  const confidenceColor = (c: number) => {
    if (c >= 0.8) return 'var(--color-success)';
    if (c >= 0.5) return 'var(--color-warning)';
    return 'var(--color-text-faint)';
  };

  const activeProposals = proposalView === 'nodes' ? nodeProposals : edgeProposals;
  const pendingCount = activeProposals.filter(p => p.status === 'pending').length;

  // ─── Render ────────────────────────────────────────────────────────────

  return (
    <Layout>
      <div className="command-page" style={{ maxWidth: '1080px' }}>
      <section className="command-hero-grid">
        <div className="command-surface-strong">
          <div className="command-surface-header">
            <div className="command-surface-copy">
              <span className="page-kicker">Proposal review</span>
              <h1 className="display-heading" style={{ margin: 0 }}>Automated Discovery</h1>
              <p className="section-copy" style={{ margin: 0 }}>
                {proposalView === 'nodes'
                  ? 'Scan project artifacts to discover technologies, components, and patterns.'
                  : 'Review relationship edges imported from code-graph tooling.'}
                {pendingCount > 0 && ` ${pendingCount} pending review.`}
              </p>
            </div>
            <div className="command-pill-matrix">
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
        </div>

        <aside className="command-surface-soft">
          <div className="command-info-grid">
            <div className="command-info-cell">
              <span className="command-info-label">Pending review</span>
              <span className="command-info-value">{pendingCount}</span>
              <span className="command-info-copy">Items still waiting for an accept or reject decision.</span>
            </div>
            <div className="command-info-cell">
              <span className="command-info-label">Current view</span>
              <span className="command-info-value" style={{ fontSize: '1.1rem', lineHeight: 1.2 }}>
                {proposalView === 'nodes' ? 'Nodes' : 'Edges'}
              </span>
              <span className="command-info-copy">Switch between discovered nodes and imported relationship proposals below.</span>
            </div>
          </div>
          <div className="utility-note" style={{ margin: 0 }}>
            Review objects stay dense and decision-oriented. The route should feel like triage, not a gallery of proposal cards.
          </div>
        </aside>
      </section>

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
      <section className="command-surface-soft">
      <div className="event-filters" style={{ marginBottom: 0 }}>
        <div className="event-filter-group" style={{ marginBottom: 'var(--space-2)' }}>
          {PROPOSAL_VIEWS.map(view => (
            <button
              key={view.key}
              className={`event-filter-chip${proposalView === view.key ? ' active' : ''}`}
              onClick={() => {
                setProposalView(view.key);
                setExpandedId(null);
              }}
            >
              {view.label}
            </button>
          ))}
        </div>
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
      </section>

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
      {!loading && !error && activeProposals.length === 0 && (
        <div className="empty-state">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-faint)" strokeWidth="1.5" strokeLinecap="round" style={{ opacity: 0.4 }}>
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <p style={{ color: 'var(--color-text-faint)', marginTop: 'var(--space-3)' }}>
            {filterStatus === 'all'
              ? proposalView === 'nodes'
                ? 'No proposals yet. Run a discovery scan to detect technologies and components from your project.'
                : 'No relationship proposals yet. Import edge proposals from code-graph tooling to review them here.'
              : `No ${filterStatus} proposals.`
            }
          </p>
        </div>
      )}

      {/* Node proposal list */}
      {!loading && !error && proposalView === 'nodes' && nodeProposals.length > 0 && (
        <section className="command-surface-soft">
        <div className="snapshot-list">
          {nodeProposals.map(p => {
            const relatedKnowledgeLink = getRelatedKnowledgeLink(p);
            const displayName = proposalNameOverrides[p.id]?.trim() || getNodeDisplayName(p);
            const suggestedComponentName =
              p.node.node_type === 'component'
                ? p.node.name
                : null;
            const componentNameEdited =
              Boolean(suggestedComponentName)
              && Boolean(proposalNameOverrides[p.id]?.trim())
              && proposalNameOverrides[p.id]!.trim() !== suggestedComponentName;
            const acceptanceNameSource =
              componentNameEdited
                ? 'Manual'
                : (p.node.node_type === 'component' && p.node.naming?.source === 'manual')
                  ? 'Manual'
                  : 'Generated';
            return (
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
                        {displayName}
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
                    <button
                      className="btn btn-outline"
                      style={{
                        fontSize: '0.625rem',
                        padding: 'var(--space-1) var(--space-2)',
                        opacity: relatedKnowledgeLink ? 1 : 0.5,
                      }}
                      title={relatedKnowledgeLink
                        ? 'Open related knowledge in the scoped project context'
                        : 'Scope unavailable for this proposal'}
                      disabled={!relatedKnowledgeLink}
                      onClick={(event) => {
                        event.stopPropagation();
                        if (!relatedKnowledgeLink) return;
                        void navigate(relatedKnowledgeLink);
                      }}
                    >
                      View related knowledge
                    </button>
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
                    {p.node.node_type === 'component' && suggestedComponentName && (
                      <div style={{ display: 'grid', gap: 'var(--space-2)', marginBottom: 'var(--space-3)' }}>
                        <label style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
                          Suggested component name
                          <input
                            className="field-input"
                            value={proposalNameOverrides[p.id] ?? suggestedComponentName}
                            onChange={(event) => {
                              const nextValue = event.target.value;
                              setProposalNameOverrides((prev) => ({ ...prev, [p.id]: nextValue }));
                            }}
                            style={{ marginTop: '4px' }}
                          />
                        </label>
                        <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
                          Name source on accept: <strong>{acceptanceNameSource}</strong>
                          {componentNameEdited ? ' (manual rename will be preserved across regeneration)' : ''}
                        </div>
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
                          onClick={() => handleAccept(p)}
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
              );
          })}
        </div>
        </section>
      )}

      {/* Edge proposal list */}
      {!loading && !error && proposalView === 'edges' && edgeProposals.length > 0 && (
        <section className="command-surface-soft">
        <div className="snapshot-list">
          {edgeProposals.map(p => {
            const displayName = `${p.edge.source} -> ${p.edge.target}`;
            return (
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
                <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)', width: '100%' }}>
                  <span style={{ fontSize: '1.2em' }} title={SOURCE_LABELS[p.source]}>
                    {SOURCE_ICONS[p.source]}
                  </span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
                      <span style={{ fontWeight: 600, fontSize: 'var(--text-sm)', fontFamily: 'var(--font-mono)' }}>
                        {displayName}
                      </span>
                      <span className="badge badge-component" style={{ fontSize: '0.5625rem' }}>
                        {p.edge.edge_type}
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
                    {p.edge.metadata && (
                      <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)', marginBottom: 'var(--space-2)' }}>
                        <strong>Metadata:</strong> {p.edge.metadata}
                      </div>
                    )}
                    <details>
                      <summary style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', cursor: 'pointer' }}>
                        Full edge data
                      </summary>
                      <pre style={{
                        fontSize: '0.625rem', lineHeight: 1.4,
                        padding: 'var(--space-2)', marginTop: 'var(--space-1)',
                        background: 'var(--color-surface-offset)', borderRadius: 'var(--radius-sm)',
                        overflow: 'auto', maxHeight: '200px',
                      }}>
                        {JSON.stringify(p, null, 2)}
                      </pre>
                    </details>

                    {p.status === 'pending' && (
                      <div style={{ display: 'flex', gap: 'var(--space-2)', marginTop: 'var(--space-3)' }}>
                        <button
                          className="btn btn-primary"
                          style={{ fontSize: 'var(--text-xs)', padding: 'var(--space-1) var(--space-3)' }}
                          onClick={() => handleAcceptEdge(p)}
                          disabled={actionLoading === p.id}
                        >
                          {actionLoading === p.id ? '…' : 'Accept Edge'}
                        </button>
                        <button
                          className="btn btn-outline"
                          style={{
                            fontSize: 'var(--text-xs)', padding: 'var(--space-1) var(--space-3)',
                            color: 'var(--color-error)', borderColor: 'var(--color-error)',
                          }}
                          onClick={() => handleRejectEdge(p.id)}
                          disabled={actionLoading === p.id}
                        >
                          {actionLoading === p.id ? '…' : 'Reject Edge'}
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
            );
          })}
        </div>
        </section>
      )}
      </div>
    </Layout>
  );
}
