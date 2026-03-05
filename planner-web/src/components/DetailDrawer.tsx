import { useEffect, useMemo } from 'react';
import type { BlueprintNode, EdgePayload } from '../types/blueprint.ts';
import type { ApiClient } from '../api/client.ts';
import { useState } from 'react';

// ─── Type → badge class mapping ────────────────────────────────────────────

const TYPE_LABELS: Record<string, string> = {
  decision: 'Decision',
  technology: 'Technology',
  component: 'Component',
  constraint: 'Constraint',
  pattern: 'Pattern',
  quality_requirement: 'Quality',
};

// ─── Props ──────────────────────────────────────────────────────────────────

interface DetailDrawerProps {
  nodeId: string | null;
  allNodes: { id: string; name: string; node_type: string }[];
  edges: EdgePayload[];
  api: ApiClient;
  onClose: () => void;
  onNavigateNode: (nodeId: string) => void;
  onImpactPreview: (nodeId: string) => void;
}

// ─── Component ──────────────────────────────────────────────────────────────

export default function DetailDrawer({
  nodeId,
  allNodes,
  edges,
  api,
  onClose,
  onNavigateNode,
  onImpactPreview,
}: DetailDrawerProps) {
  const [node, setNode] = useState<BlueprintNode | null>(null);
  const [loading, setLoading] = useState(false);

  const isOpen = nodeId !== null;

  // Fetch full node detail
  useEffect(() => {
    if (!nodeId) { setNode(null); return; }
    let cancelled = false;
    setLoading(true);
    api.getBlueprintNode(nodeId)
      .then(data => { if (!cancelled) setNode(data); })
      .catch(() => { if (!cancelled) setNode(null); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [nodeId, api]);

  // Connected edges
  const connections = useMemo(() => {
    if (!nodeId) return { upstream: [], downstream: [] };
    const upstream = edges.filter(e => e.target === nodeId).map(e => ({
      id: e.source, type: e.edge_type, direction: 'upstream' as const,
    }));
    const downstream = edges.filter(e => e.source === nodeId).map(e => ({
      id: e.target, type: e.edge_type, direction: 'downstream' as const,
    }));
    return { upstream, downstream };
  }, [edges, nodeId]);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [isOpen, onClose]);

  // Get node display name
  const getNodeName = (id: string) => {
    const n = allNodes.find(n => n.id === id);
    return n?.name ?? id;
  };

  const getNodeType = (id: string) => {
    const n = allNodes.find(n => n.id === id);
    return n?.node_type ?? 'unknown';
  };

  // Derive display name from node
  const nodeName = node
    ? ('title' in node ? (node as { title: string }).title
      : 'name' in node ? (node as { name: string }).name
      : 'scenario' in node ? (node as { scenario: string }).scenario
      : nodeId ?? '')
    : '';

  const nodeType = node?.node_type ?? '';
  const typeBadge = `badge-${nodeType}`;
  const typeLabel = TYPE_LABELS[nodeType] ?? nodeType;

  return (
    <>
      {/* Overlay */}
      <div
        className={`drawer-overlay${isOpen ? ' open' : ''}`}
        onClick={onClose}
      />

      {/* Drawer */}
      <div className={`drawer${isOpen ? ' open' : ''}`}>
        <div className="drawer-header">
          <div style={{ flex: 1 }}>
            <div className="drawer-title">{loading ? 'Loading…' : nodeName}</div>
            {node && (
              <div className="drawer-badges">
                <span className={`badge ${typeBadge}`}>{typeLabel}</span>
                {'status' in node && (
                  <span className={`status-badge status-${String((node as { status: string }).status).toLowerCase().replace(/\s+/g, '-')}`}>
                    {(node as { status: string }).status}
                  </span>
                )}
                {'ring' in node && (
                  <span className={`status-badge status-${String((node as { ring: string }).ring).toLowerCase()}`}>
                    {(node as { ring: string }).ring}
                  </span>
                )}
              </div>
            )}
          </div>
          <button className="drawer-close" onClick={onClose} aria-label="Close drawer">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6L6 18M6 6l12 12"/>
            </svg>
          </button>
        </div>

        <div className="drawer-body">
          {loading && (
            <div style={{ display: 'flex', justifyContent: 'center', padding: 'var(--space-8)' }}>
              <div className="skeleton-pulse" />
            </div>
          )}

          {!loading && node && (
            <>
              {/* ID */}
              <div className="drawer-id">{nodeId}</div>

              {/* Description */}
              <div className="drawer-section">
                <div className="drawer-section-title">Description</div>
                <div className="drawer-description">
                  {'description' in node ? (node as { description: string }).description : ''}
                  {'context' in node ? (node as { context: string }).context : ''}
                  {'rationale' in node ? (node as { rationale: string }).rationale : ''}
                  {'scenario' in node ? (node as { scenario: string }).scenario : ''}
                </div>
              </div>

              {/* Decision options */}
              {node.node_type === 'decision' && 'options' in node && (node as { options: { name: string }[] }).options.length > 0 && (
                <div className="drawer-section">
                  <div className="drawer-section-title">Options Considered</div>
                  {(node as { options: { name: string; description: string }[]; chosenOption?: string }).options.map((opt, i) => {
                    const dec = node as { options: { name: string }[]; chosenOption?: string };
                    // Support both API format and mockup format
                    const chosen = 'chosenOption' in dec
                      ? dec.chosenOption === opt.name
                      : false;
                    return (
                      <div key={i} className={`option-item${chosen ? ' chosen' : ''}`}>
                        {chosen ? '✓ ' : ''}{opt.name}
                        {chosen && (
                          <span style={{
                            fontSize: '0.5625rem', fontWeight: 700,
                            textTransform: 'uppercase', letterSpacing: '0.1em',
                            marginLeft: 'var(--space-2)',
                          }}>
                            CHOSEN
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}

              {/* Technology category */}
              {'category' in node && (
                <div className="drawer-section">
                  <div className="drawer-section-title">Category</div>
                  <div className="drawer-description" style={{ textTransform: 'capitalize' }}>
                    {(node as { category: string }).category}
                  </div>
                </div>
              )}

              {/* Quality attribute */}
              {'attribute' in node && (
                <div className="drawer-section">
                  <div className="drawer-section-title">Quality Attribute</div>
                  <div className="drawer-description" style={{ textTransform: 'capitalize' }}>
                    {(node as { attribute: string }).attribute}
                  </div>
                </div>
              )}

              {/* Component responsibilities */}
              {'responsibilities' in node && (node as { responsibilities: string[] }).responsibilities.length > 0 && (
                <div className="drawer-section">
                  <div className="drawer-section-title">Responsibilities</div>
                  <ul style={{ margin: 0, paddingLeft: 'var(--space-5)' }}>
                    {(node as { responsibilities: string[] }).responsibilities.map((r, i) => (
                      <li key={i} style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', padding: '2px 0' }}>{r}</li>
                    ))}
                  </ul>
                </div>
              )}

              {/* Upstream connections */}
              {connections.upstream.length > 0 && (
                <div className="drawer-section">
                  <div className="drawer-section-title">Upstream (depends on this)</div>
                  {connections.upstream.map((c, i) => (
                    <div
                      key={i}
                      className="drawer-relation"
                      onClick={() => onNavigateNode(c.id)}
                    >
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.625rem', color: 'var(--color-text-faint)' }}>←</span>
                      <span className={`badge badge-${getNodeType(c.id)}`} style={{ fontSize: '0.5625rem' }}>
                        {TYPE_LABELS[getNodeType(c.id)] ?? getNodeType(c.id)}
                      </span>
                      <span style={{ color: 'var(--color-text)', fontWeight: 500 }}>{getNodeName(c.id)}</span>
                      <span style={{ color: 'var(--color-text-faint)', fontSize: '0.625rem' }}>{c.type}</span>
                    </div>
                  ))}
                </div>
              )}

              {/* Downstream connections */}
              {connections.downstream.length > 0 && (
                <div className="drawer-section">
                  <div className="drawer-section-title">Downstream (this depends on)</div>
                  {connections.downstream.map((c, i) => (
                    <div
                      key={i}
                      className="drawer-relation"
                      onClick={() => onNavigateNode(c.id)}
                    >
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.625rem', color: 'var(--color-text-faint)' }}>→</span>
                      <span className={`badge badge-${getNodeType(c.id)}`} style={{ fontSize: '0.5625rem' }}>
                        {TYPE_LABELS[getNodeType(c.id)] ?? getNodeType(c.id)}
                      </span>
                      <span style={{ color: 'var(--color-text)', fontWeight: 500 }}>{getNodeName(c.id)}</span>
                      <span style={{ color: 'var(--color-text-faint)', fontSize: '0.625rem' }}>{c.type}</span>
                    </div>
                  ))}
                </div>
              )}

              {/* Tags */}
              {'tags' in node && (node as { tags: string[] }).tags.length > 0 && (
                <div className="drawer-section">
                  <div className="drawer-section-title">Tags</div>
                  <div style={{ display: 'flex', gap: 'var(--space-2)', flexWrap: 'wrap' }}>
                    {(node as { tags: string[] }).tags.map((t, i) => (
                      <span key={i} style={{
                        padding: '1px 8px', fontSize: '0.625rem', fontWeight: 500,
                        borderRadius: 'var(--radius-full)', border: '1px solid var(--color-border)',
                        color: 'var(--color-text-muted)',
                      }}>
                        {t}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {/* Timestamps */}
              {'created_at' in node && (
                <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)', opacity: 0.6, marginTop: 'var(--space-4)' }}>
                  {'created_at' in node && <div>Created: {(node as { created_at: string }).created_at}</div>}
                  {'updated_at' in node && <div>Updated: {(node as { updated_at: string }).updated_at}</div>}
                </div>
              )}
            </>
          )}
        </div>

        <div className="drawer-footer">
          <button className="btn btn-outline" onClick={onClose}>Close</button>
          <button className="btn btn-primary" onClick={() => nodeId && onImpactPreview(nodeId)}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
            </svg>
            Impact Preview
          </button>
        </div>
      </div>
    </>
  );
}
