import { useEffect, useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeSanitize from 'rehype-sanitize';
import type { BlueprintNode, EdgePayload, DecisionNode, TechnologyNode, ComponentNode, ConstraintNode, PatternNode, QualityRequirementNode, BlueprintEventPayload, NodeScope } from '../types/blueprint.ts';
import type { ApiClient } from '../api/client.ts';
import { useState, useCallback } from 'react';
import EditNodeForm from './EditNodeForm.tsx';

// ─── Type → badge class mapping ────────────────────────────────────────────

const TYPE_LABELS: Record<string, string> = {
  decision: 'Decision',
  technology: 'Technology',
  component: 'Component',
  constraint: 'Constraint',
  pattern: 'Pattern',
  quality_requirement: 'Quality',
};

const SCOPE_CLASS_LABELS: Record<string, string> = {
  global: 'Global',
  project: 'Project',
  project_contextual: 'Project Contextual',
  unscoped: 'Unscoped',
};

const SCOPE_VISIBILITY_LABELS: Record<string, string> = {
  shared: 'Shared',
  project_local: 'Project Local',
  unscoped: 'Unscoped',
};

const DEFAULT_SCOPE: NodeScope = {
  scope_class: 'unscoped',
  secondary: {},
  is_shared: false,
  lifecycle: 'active',
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
  onNodeUpdated?: () => void;
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
  onNodeUpdated,
}: DetailDrawerProps) {
  const [node, setNode] = useState<BlueprintNode | null>(null);
  const [loading, setLoading] = useState(false);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [activeTab, setActiveTab] = useState<'details' | 'history' | 'docs'>('details');
  const [events, setEvents] = useState<BlueprintEventPayload[]>([]);
  const [eventsLoading, setEventsLoading] = useState(false);

  const isOpen = nodeId !== null;

  // Reset edit mode and tab when node changes
  useEffect(() => {
    setEditing(false);
    setSaving(false);
    setActiveTab('details');
    setEvents([]);
  }, [nodeId]);

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

  // Fetch events when history tab is activated
  useEffect(() => {
    if (activeTab !== 'history' || !nodeId) return;
    let cancelled = false;
    setEventsLoading(true);
    api.listBlueprintEvents({ nodeId, limit: 50 })
      .then(res => { if (!cancelled) setEvents(res.events); })
      .catch(() => { if (!cancelled) setEvents([]); })
      .finally(() => { if (!cancelled) setEventsLoading(false); });
    return () => { cancelled = true; };
  }, [activeTab, nodeId, api]);

  // Connected edges
  const connections = useMemo(() => {
    if (!nodeId) return { upstream: [], downstream: [] };
    const upstream = edges.filter(e => e.target === nodeId).map(e => ({
      id: e.source, type: e.edge_type, direction: 'upstream' as const, metadata: e.metadata,
    }));
    const downstream = edges.filter(e => e.source === nodeId).map(e => ({
      id: e.target, type: e.edge_type, direction: 'downstream' as const, metadata: e.metadata,
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

  // Handle save from edit form
  const handleSave = useCallback(async (updated: BlueprintNode) => {
    if (!nodeId) return;
    setSaving(true);
    try {
      await api.updateBlueprintNode(nodeId, updated);
      setNode(updated);
      setEditing(false);
      onNodeUpdated?.();
    } finally {
      setSaving(false);
    }
  }, [nodeId, api, onNodeUpdated]);

  const handleCancelEdit = useCallback(() => {
    setEditing(false);
  }, []);

  const nodeType = node?.node_type ?? '';
  const typeBadge = `badge-${nodeType}`;
  const typeLabel = TYPE_LABELS[nodeType] ?? nodeType;
  const documentation = node?.documentation?.trim() ?? '';
  const effectiveNodeScope = (node as { scope?: NodeScope } | null)?.scope ?? DEFAULT_SCOPE;

  const renderScopeDetails = (n: BlueprintNode) => {
    const scope = (n as { scope?: NodeScope }).scope ?? DEFAULT_SCOPE;
    const scopeVisibility =
      scope.scope_class === 'unscoped'
        ? 'unscoped'
        : scope.is_shared
          ? 'shared'
          : 'project_local';
    return (
      <div className="drawer-section">
        <div className="drawer-section-title">Scope</div>
        <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap', marginBottom: 'var(--space-2)' }}>
          <span className="health-badge" style={{ color: 'var(--color-text-muted)', background: 'var(--color-surface-offset)' }}>
            {SCOPE_CLASS_LABELS[scope.scope_class] ?? scope.scope_class}
          </span>
          <span
            className="health-badge"
            style={{
              color:
                scopeVisibility === 'shared'
                  ? 'var(--color-blue)'
                  : scopeVisibility === 'project_local'
                    ? 'var(--color-success)'
                    : 'var(--color-warning)',
              background:
                scopeVisibility === 'shared'
                  ? 'rgba(59,130,246,0.14)'
                  : scopeVisibility === 'project_local'
                    ? 'rgba(34,197,94,0.14)'
                    : 'rgba(234,179,8,0.14)',
            }}
          >
            {SCOPE_VISIBILITY_LABELS[scopeVisibility] ?? scopeVisibility}
          </span>
          <span
            className="health-badge"
            style={{
              color: scope.lifecycle === 'archived' ? 'var(--color-warning)' : 'var(--color-success)',
              background: scope.lifecycle === 'archived'
                ? 'rgba(234,179,8,0.14)'
                : 'rgba(34,197,94,0.14)',
            }}
          >
            {scope.lifecycle === 'archived' ? 'Archived' : 'Active'}
          </span>
        </div>
        {scope.project && (
          <div className="drawer-description">
            <strong>Project:</strong> {scope.project.project_name ?? scope.project.project_id} ({scope.project.project_id})
          </div>
        )}
        {(scope.secondary.feature || scope.secondary.widget || scope.secondary.artifact || scope.secondary.component) && (
          <div className="drawer-description" style={{ marginTop: '4px' }}>
            <strong>Context:</strong>{' '}
            {[scope.secondary.feature, scope.secondary.widget, scope.secondary.artifact, scope.secondary.component]
              .filter(Boolean)
              .join(' · ')}
          </div>
        )}
        {scope.is_shared && (
          <div className="drawer-description" style={{ marginTop: '4px' }}>
            <strong>Linked Projects:</strong>{' '}
            {(scope.shared?.linked_project_ids ?? []).join(', ') || 'none'}
          </div>
        )}
        {scope.override_scope && (
          <div className="drawer-description" style={{ marginTop: '4px' }}>
            <strong>Overrides:</strong> {scope.override_scope.shared_source_id}
            {scope.override_scope.override_reason ? ` · ${scope.override_scope.override_reason}` : ''}
            {scope.override_scope.effective_from ? ` · effective ${scope.override_scope.effective_from}` : ''}
          </div>
        )}
      </div>
    );
  };

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
      case 'decision': return <>{renderScopeDetails(node)}{renderDecisionDetails(node as DecisionNode)}</>;
      case 'technology': return <>{renderScopeDetails(node)}{renderTechnologyDetails(node as TechnologyNode)}</>;
      case 'component': return <>{renderScopeDetails(node)}{renderComponentDetails(node as ComponentNode)}</>;
      case 'constraint': return <>{renderScopeDetails(node)}{renderConstraintDetails(node as ConstraintNode)}</>;
      case 'pattern': return <>{renderScopeDetails(node)}{renderPatternDetails(node as PatternNode)}</>;
      case 'quality_requirement': return <>{renderScopeDetails(node)}{renderQualityDetails(node as QualityRequirementNode)}</>;
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
                <span className="status-badge">
                  {SCOPE_CLASS_LABELS[effectiveNodeScope.scope_class] ?? effectiveNodeScope.scope_class}
                </span>
                <span className="status-badge">
                  {SCOPE_VISIBILITY_LABELS[
                    (effectiveNodeScope.scope_class === 'unscoped')
                      ? 'unscoped'
                      : effectiveNodeScope.is_shared
                        ? 'shared'
                        : 'project_local'
                  ]}
                </span>
              </div>
            )}
          </div>
          <button className="drawer-close" onClick={onClose} aria-label="Close drawer">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6L6 18M6 6l12 12"/>
            </svg>
          </button>
        </div>

        {/* Tab bar — only visible in view mode */}
        {!loading && node && !editing && (
          <div className="drawer-tabs">
            <button
              className={`drawer-tab${activeTab === 'details' ? ' active' : ''}`}
              onClick={() => setActiveTab('details')}
            >
              Details
            </button>
            <button
              className={`drawer-tab${activeTab === 'history' ? ' active' : ''}`}
              onClick={() => setActiveTab('history')}
            >
              History
            </button>
            <button
              className={`drawer-tab${activeTab === 'docs' ? ' active' : ''}`}
              onClick={() => setActiveTab('docs')}
            >
              Docs
            </button>
          </div>
        )}

        <div className="drawer-body">
          {loading && (
            <div style={{ display: 'flex', justifyContent: 'center', padding: 'var(--space-8)' }}>
              <div className="skeleton-pulse" />
            </div>
          )}

          {!loading && node && editing && (
            <EditNodeForm
              node={node}
              onSave={handleSave}
              onCancel={handleCancelEdit}
              saving={saving}
            />
          )}

          {/* ─── Details tab ─── */}
          {!loading && node && !editing && activeTab === 'details' && (
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
                      {c.metadata && (
                        <div className="edge-annotation">— {c.metadata}</div>
                      )}
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
                      {c.metadata && (
                        <div className="edge-annotation">— {c.metadata}</div>
                      )}
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

          {/* ─── History tab ─── */}
          {!loading && node && !editing && activeTab === 'history' && (
            <div className="event-timeline">
              {eventsLoading && (
                <div style={{ display: 'flex', justifyContent: 'center', padding: 'var(--space-8)' }}>
                  <div className="skeleton-pulse" />
                </div>
              )}
              {!eventsLoading && events.length === 0 && (
                <div style={{
                  textAlign: 'center', padding: 'var(--space-8)',
                  color: 'var(--color-text-faint)', fontSize: 'var(--text-sm)',
                }}>
                  No events recorded for this node.
                </div>
              )}
              {!eventsLoading && events.map((evt, i) => {
                const before = evt.event_type === 'node_updated' && evt.data?.before
                  ? evt.data.before as Record<string, unknown> : null;
                const after = evt.event_type === 'node_updated' && evt.data?.after
                  ? evt.data.after as Record<string, unknown> : null;
                const hasDiff = before !== null && after !== null;

                // collect changed keys
                const diffKeys = hasDiff
                  ? [...new Set([...Object.keys(before!), ...Object.keys(after!)])].filter(
                      k => JSON.stringify(before![k]) !== JSON.stringify(after![k])
                    )
                  : [];

                return (
                <div key={i} className="event-timeline-item">
                  <div className="event-timeline-dot-container">
                    <div className={`event-timeline-dot event-dot-${evt.event_type}`} />
                    {i < events.length - 1 && <div className="event-timeline-line" />}
                  </div>
                  <div className="event-timeline-content">
                    <div className="event-timeline-header">
                      <span className={`event-type-badge event-badge-${evt.event_type}`}>
                        {evt.event_type.replace(/_/g, ' ')}
                      </span>
                      <span className="event-timeline-time">
                        {new Date(evt.timestamp).toLocaleString()}
                      </span>
                    </div>
                    <div className="event-timeline-summary">{evt.summary}</div>

                    {/* Diff view for node_updated events */}
                    {hasDiff && diffKeys.length > 0 && (
                      <details className="event-timeline-data" open>
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

          {!loading && node && !editing && activeTab === 'docs' && (
            <div className="drawer-section">
              {documentation ? (
                <div className="drawer-description" data-testid="node-documentation">
                  <ReactMarkdown rehypePlugins={[rehypeSanitize]}>
                    {documentation}
                  </ReactMarkdown>
                </div>
              ) : (
                <div
                  style={{
                    textAlign: 'center',
                    padding: 'var(--space-8)',
                    color: 'var(--color-text-faint)',
                    fontSize: 'var(--text-sm)',
                  }}
                >
                  No documentation yet.
                </div>
              )}
            </div>
          )}
        </div>

        {!editing && (
        <div className="drawer-footer">
          <button className="btn btn-outline" onClick={() => setEditing(true)} disabled={!node}>
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
        )}
      </div>
    </>
  );
}
