import { useEffect, useMemo } from 'react';
import type { BlueprintNode, EdgePayload, DecisionNode, TechnologyNode, ComponentNode, ConstraintNode, PatternNode, QualityRequirementNode } from '../types/blueprint.ts';
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
  onRequestDelete?: (nodeId: string) => void;
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
  onRequestDelete,
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

  // Derive display name from the typed union
  const nodeName = (() => {
    if (!node) return '';
    switch (node.node_type) {
      case 'decision': return (node as DecisionNode).title;
      case 'technology': return (node as TechnologyNode).name;
      case 'component': return (node as ComponentNode).name;
      case 'constraint': return (node as ConstraintNode).title;
      case 'pattern': return (node as PatternNode).name;
      case 'quality_requirement': return (node as QualityRequirementNode).scenario;
      default: return nodeId ?? '';
    }
  })();

  // Derive status display
  const nodeStatus = (() => {
    if (!node) return '';
    switch (node.node_type) {
      case 'decision': return (node as DecisionNode).status;
      case 'technology': return (node as TechnologyNode).ring;
      case 'component': return (node as ComponentNode).status;
      case 'constraint': return (node as ConstraintNode).constraint_type;
      case 'pattern': return 'active';
      case 'quality_requirement': return (node as QualityRequirementNode).priority;
      default: return '';
    }
  })();

  const nodeType = node?.node_type ?? '';
  const typeBadge = `badge-${nodeType}`;
  const typeLabel = TYPE_LABELS[nodeType] ?? nodeType;

  // ─── Type-specific detail sections ──────────────────────────────────────

  const renderDecisionDetails = (n: DecisionNode) => (
    <>
      <div className="drawer-section">
        <div className="drawer-section-title">Context</div>
        <div className="drawer-description">{n.context}</div>
      </div>
      {n.options.length > 0 && (
        <div className="drawer-section">
          <div className="drawer-section-title">Options Considered</div>
          {n.options.map((opt, i) => (
            <div key={i} className={`option-item${opt.chosen ? ' chosen' : ''}`}>
              {opt.chosen ? '✓ ' : ''}{opt.name}
              {opt.chosen && (
                <span style={{
                  fontSize: '0.5625rem', fontWeight: 700,
                  textTransform: 'uppercase', letterSpacing: '0.1em',
                  marginLeft: 'var(--space-2)',
                }}>
                  CHOSEN
                </span>
              )}
              {opt.pros.length > 0 && (
                <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-success)', marginTop: '2px' }}>
                  + {opt.pros.join(', ')}
                </div>
              )}
              {opt.cons.length > 0 && (
                <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-error)', marginTop: '2px' }}>
                  − {opt.cons.join(', ')}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
      {n.consequences.length > 0 && (
        <div className="drawer-section">
          <div className="drawer-section-title">Consequences</div>
          {n.consequences.map((c, i) => (
            <div key={i} style={{
              fontSize: 'var(--text-xs)', padding: '2px 0',
              color: c.positive ? 'var(--color-success)' : 'var(--color-error)',
            }}>
              {c.positive ? '✓' : '✗'} {c.description}
            </div>
          ))}
        </div>
      )}
      {n.assumptions.length > 0 && (
        <div className="drawer-section">
          <div className="drawer-section-title">Assumptions</div>
          {n.assumptions.map((a, i) => (
            <div key={i} style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', padding: '2px 0' }}>
              {a.description}
              <span style={{
                fontSize: '0.5625rem', marginLeft: 'var(--space-2)',
                color: 'var(--color-text-faint)', textTransform: 'uppercase',
              }}>
                [{a.confidence}]
              </span>
            </div>
          ))}
        </div>
      )}
      {n.supersedes && (
        <div className="drawer-section">
          <div className="drawer-section-title">Supersedes</div>
          <div
            className="drawer-relation"
            onClick={() => onNavigateNode(n.supersedes!)}
            style={{ cursor: 'pointer' }}
          >
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.625rem', color: 'var(--color-text-faint)' }}>↩</span>
            <span style={{ color: 'var(--color-text)', fontWeight: 500 }}>{getNodeName(n.supersedes)}</span>
          </div>
        </div>
      )}
    </>
  );

  const renderTechnologyDetails = (n: TechnologyNode) => (
    <>
      <div className="drawer-section">
        <div className="drawer-section-title">Rationale</div>
        <div className="drawer-description">{n.rationale}</div>
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">Category</div>
        <div className="drawer-description" style={{ textTransform: 'capitalize' }}>{n.category}</div>
      </div>
      {n.version && (
        <div className="drawer-section">
          <div className="drawer-section-title">Version</div>
          <div className="drawer-description" style={{ fontFamily: 'var(--font-mono)' }}>{n.version}</div>
        </div>
      )}
      {n.license && (
        <div className="drawer-section">
          <div className="drawer-section-title">License</div>
          <div className="drawer-description">{n.license}</div>
        </div>
      )}
    </>
  );

  const renderComponentDetails = (n: ComponentNode) => (
    <>
      <div className="drawer-section">
        <div className="drawer-section-title">Description</div>
        <div className="drawer-description">{n.description}</div>
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">Component Type</div>
        <div className="drawer-description" style={{ textTransform: 'capitalize' }}>{n.component_type}</div>
      </div>
      {n.provides.length > 0 && (
        <div className="drawer-section">
          <div className="drawer-section-title">Provides</div>
          <ul style={{ margin: 0, paddingLeft: 'var(--space-5)' }}>
            {n.provides.map((p, i) => (
              <li key={i} style={{ fontSize: 'var(--text-xs)', color: 'var(--color-success)', padding: '2px 0' }}>{p}</li>
            ))}
          </ul>
        </div>
      )}
      {n.consumes.length > 0 && (
        <div className="drawer-section">
          <div className="drawer-section-title">Consumes</div>
          <ul style={{ margin: 0, paddingLeft: 'var(--space-5)' }}>
            {n.consumes.map((c, i) => (
              <li key={i} style={{ fontSize: 'var(--text-xs)', color: 'var(--color-warning)', padding: '2px 0' }}>{c}</li>
            ))}
          </ul>
        </div>
      )}
    </>
  );

  const renderConstraintDetails = (n: ConstraintNode) => (
    <>
      <div className="drawer-section">
        <div className="drawer-section-title">Description</div>
        <div className="drawer-description">{n.description}</div>
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">Constraint Type</div>
        <div className="drawer-description" style={{ textTransform: 'capitalize' }}>{n.constraint_type}</div>
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">Source</div>
        <div className="drawer-description">{n.source}</div>
      </div>
    </>
  );

  const renderPatternDetails = (n: PatternNode) => (
    <>
      <div className="drawer-section">
        <div className="drawer-section-title">Description</div>
        <div className="drawer-description">{n.description}</div>
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">Rationale</div>
        <div className="drawer-description">{n.rationale}</div>
      </div>
    </>
  );

  const renderQualityDetails = (n: QualityRequirementNode) => (
    <>
      <div className="drawer-section">
        <div className="drawer-section-title">Quality Attribute</div>
        <div className="drawer-description" style={{ textTransform: 'capitalize' }}>{n.attribute}</div>
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">Scenario</div>
        <div className="drawer-description">{n.scenario}</div>
      </div>
      <div className="drawer-section">
        <div className="drawer-section-title">Priority</div>
        <div className="drawer-description" style={{ textTransform: 'capitalize' }}>{n.priority}</div>
      </div>
    </>
  );

  const renderNodeDetails = () => {
    if (!node) return null;
    switch (node.node_type) {
      case 'decision': return renderDecisionDetails(node as DecisionNode);
      case 'technology': return renderTechnologyDetails(node as TechnologyNode);
      case 'component': return renderComponentDetails(node as ComponentNode);
      case 'constraint': return renderConstraintDetails(node as ConstraintNode);
      case 'pattern': return renderPatternDetails(node as PatternNode);
      case 'quality_requirement': return renderQualityDetails(node as QualityRequirementNode);
      default: return null;
    }
  };

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
                {nodeStatus && (
                  <span className={`status-badge status-${nodeStatus.toLowerCase().replace(/\s+/g, '-')}`}>
                    {nodeStatus}
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

              {/* Type-specific details */}
              {renderNodeDetails()}

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
              {node && 'tags' in node && (node as { tags: string[] }).tags.length > 0 && (
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
                  <div>Created: {(node as { created_at: string }).created_at}</div>
                  {'updated_at' in node && <div>Updated: {(node as { updated_at: string }).updated_at}</div>}
                </div>
              )}
            </>
          )}
        </div>

        <div className="drawer-footer">
          <button className="btn btn-outline" onClick={() => {
            // TODO: Wire to inline editing mode in Phase C
            alert('Editing coming soon — Phase C');
          }}>
            Edit
          </button>
          {onRequestDelete && (
            <button
              className="btn btn-outline"
              onClick={() => nodeId && onRequestDelete(nodeId)}
              style={{ color: 'var(--color-error)', borderColor: 'var(--color-error)' }}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
              </svg>
              Delete
            </button>
          )}
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
