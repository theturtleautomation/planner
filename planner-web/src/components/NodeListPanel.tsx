import { useEffect, useMemo, useRef, useState } from 'react';
import type { NodeSummary, EdgePayload, NodeType } from '../types/blueprint.ts';

// ─── Props ──────────────────────────────────────────────────────────────────

interface NodeListPanelProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  nodeType: NodeType | null;
  onSelectNode: (nodeId: string) => void;
  selectedNodeIds?: string[];
  onToggleSelectNode?: (nodeId: string, selected: boolean) => void;
  onToggleSelectAllVisible?: (nodeIds: string[], selected: boolean) => void;
  /** Column configuration per node type. */
  columns?: ColumnDef[];
}

interface ColumnDef {
  key: string;
  label: string;
  render: (node: NodeSummary, edges: EdgePayload[]) => React.ReactNode;
  sortValue?: (node: NodeSummary, edges: EdgePayload[]) => string;
  width?: string;
}

// ─── Defaults ───────────────────────────────────────────────────────────────

const TYPE_LABELS: Record<string, string> = {
  decision: 'Decision',
  technology: 'Technology',
  component: 'Component',
  constraint: 'Constraint',
  pattern: 'Pattern',
  quality_requirement: 'Quality Req.',
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

const STATUS_COLORS: Record<string, string> = {
  // Decision statuses
  proposed: 'var(--color-warning)',
  accepted: 'var(--color-success)',
  superseded: 'var(--color-text-faint)',
  deprecated: 'var(--color-error)',
  // Technology rings
  adopt: 'var(--color-success)',
  trial: 'var(--color-blue)',
  assess: 'var(--color-warning)',
  hold: 'var(--color-error)',
  // Component statuses
  planned: 'var(--color-text-muted)',
  'in progress': 'var(--color-blue)',
  in_progress: 'var(--color-blue)',
  shipped: 'var(--color-success)',
  // Constraint types
  technical: 'var(--color-blue)',
  organizational: 'var(--color-purple)',
  philosophical: 'var(--color-gold)',
  regulatory: 'var(--color-error)',
  // Quality priorities
  critical: 'var(--color-error)',
  high: 'var(--color-warning)',
  medium: 'var(--color-blue)',
  low: 'var(--color-text-muted)',
  // Pattern
  active: 'var(--color-success)',
};

const STALE_THRESHOLD_DAYS = 30;
const LEGACY_ARCHIVED_TAG = 'archived';
const LINEAGE_BRANCH_PREFIX = 'lineage:branch-of:';
const OVERRIDE_PREFIX = 'overrides:';

function isArchivedNode(node: NodeSummary): boolean {
  if (node.lifecycle === 'archived') return true;
  return node.tags.some(tag => tag.trim().toLowerCase() === LEGACY_ARCHIVED_TAG);
}

function branchLineageSource(node: NodeSummary): string | null {
  for (const rawTag of node.tags) {
    const lower = rawTag.trim().toLowerCase();
    if (!lower.startsWith(LINEAGE_BRANCH_PREFIX)) continue;
    const source = rawTag.trim().slice(LINEAGE_BRANCH_PREFIX.length).trim();
    if (source.length > 0) return source;
  }
  return null;
}

function overrideSource(node: NodeSummary): string | null {
  if (node.override_source_id?.trim()) return node.override_source_id.trim();
  for (const rawTag of node.tags) {
    const lower = rawTag.trim().toLowerCase();
    if (!lower.startsWith(OVERRIDE_PREFIX)) continue;
    const source = rawTag.trim().slice(OVERRIDE_PREFIX.length).trim();
    if (source.length > 0) return source;
  }
  return null;
}

function defaultColumns(edges: EdgePayload[]): ColumnDef[] {
  return [
    {
      key: 'name',
      label: 'Name',
      render: (n, edgeList) => {
        // Stale detection: node not updated in STALE_THRESHOLD_DAYS
        const updatedMs = new Date(n.updated_at).getTime();
        const isStale = !isNaN(updatedMs) && (Date.now() - updatedMs) > STALE_THRESHOLD_DAYS * 86400000;
        // Orphan detection: node with no edges
        const isOrphan = !edgeList.some(e => e.source === n.id || e.target === n.id);
        const lineageSource = branchLineageSource(n);
        const override = overrideSource(n);
        return (
          <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <span style={{ fontWeight: 500 }}>{n.name}</span>
            {isArchivedNode(n) && (
              <span className="health-badge" title="Archived lifecycle state">
                archived
              </span>
            )}
            {lineageSource && (
              <span className="health-badge" title={`Branched from ${lineageSource}`}>
                branch
              </span>
            )}
            {override && (
              <span className="health-badge" title={`Overrides shared record ${override}`}>
                override
              </span>
            )}
            {n.has_documentation && (
              <span className="health-badge" title="Documentation attached">
                docs
              </span>
            )}
            {isStale && (
              <span className="health-badge health-stale" title={`Not updated in ${STALE_THRESHOLD_DAYS}+ days`}>
                stale
              </span>
            )}
            {isOrphan && (
              <span className="health-badge health-orphan" title="No edges — isolated node">
                orphan
              </span>
            )}
          </span>
        );
      },
      sortValue: (n) => n.name.toLowerCase(),
    },
    {
      key: 'type',
      label: 'Type',
      render: (n) => (
        <span className={`badge badge-${n.node_type}`} style={{ fontSize: '0.5625rem' }}>
          {TYPE_LABELS[n.node_type] ?? n.node_type}
        </span>
      ),
      sortValue: (n) => n.node_type,
      width: '100px',
    },
    {
      key: 'status',
      label: 'Status',
      render: (n) => {
        const color = STATUS_COLORS[n.status.toLowerCase().replace(/\s+/g, '_')] ?? STATUS_COLORS[n.status.toLowerCase().replace(/\s+/g, ' ')] ?? 'var(--color-text-muted)';
        return (
          <span style={{ color, fontWeight: 500, fontSize: 'var(--text-xs)', textTransform: 'capitalize' }}>
            {n.status}
          </span>
        );
      },
      sortValue: (n) => n.status.toLowerCase(),
      width: '100px',
    },
    {
      key: 'scope',
      label: 'Scope',
      render: (n) => {
        const scopeClass = n.scope_class ?? 'unscoped';
        const scopeVisibility = n.scope_visibility ?? (n.is_shared ? 'shared' : 'unscoped');
        return (
          <div style={{ display: 'flex', gap: '6px', alignItems: 'center', flexWrap: 'wrap' }}>
            <span
              style={{
                fontSize: '0.5625rem',
                padding: '1px 6px',
                borderRadius: 'var(--radius-full)',
                border: '1px solid var(--color-border)',
                color: 'var(--color-text-muted)',
                whiteSpace: 'nowrap',
              }}
            >
              {SCOPE_CLASS_LABELS[scopeClass] ?? scopeClass}
            </span>
            <span
              style={{
                fontSize: '0.5625rem',
                padding: '1px 6px',
                borderRadius: 'var(--radius-full)',
                background:
                  scopeVisibility === 'shared'
                    ? 'rgba(59,130,246,0.14)'
                    : scopeVisibility === 'project_local'
                      ? 'rgba(34,197,94,0.14)'
                      : 'rgba(234,179,8,0.14)',
                color:
                  scopeVisibility === 'shared'
                    ? 'var(--color-blue)'
                    : scopeVisibility === 'project_local'
                      ? 'var(--color-success)'
                      : 'var(--color-warning)',
                whiteSpace: 'nowrap',
              }}
            >
              {SCOPE_VISIBILITY_LABELS[scopeVisibility] ?? scopeVisibility}
            </span>
            {n.project_name && (
              <span style={{ fontSize: '0.5625rem', color: 'var(--color-text-faint)' }}>
                {n.project_name}
              </span>
            )}
          </div>
        );
      },
      sortValue: (n) => `${n.scope_class ?? 'unscoped'}:${n.scope_visibility ?? 'unscoped'}:${n.project_name ?? ''}`,
      width: '190px',
    },
    {
      key: 'connections',
      label: 'Edges',
      render: (n) => {
        const count = edges.filter(e => e.source === n.id || e.target === n.id).length;
        return (
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-xs)', color: count > 0 ? 'var(--color-text-muted)' : 'var(--color-text-faint)' }}>
            {count}
          </span>
        );
      },
      sortValue: (n) => String(edges.filter(e => e.source === n.id || e.target === n.id).length).padStart(4, '0'),
      width: '60px',
    },
    {
      key: 'tags',
      label: 'Tags',
      render: (n) => (
        <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
          {n.tags.slice(0, 3).map((t, i) => (
            <span key={i} style={{
              padding: '0 6px', fontSize: '0.5625rem', fontWeight: 500,
              borderRadius: 'var(--radius-full)', border: '1px solid var(--color-border)',
              color: 'var(--color-text-muted)', whiteSpace: 'nowrap',
            }}>
              {t}
            </span>
          ))}
          {n.tags.length > 3 && (
            <span style={{ fontSize: '0.5625rem', color: 'var(--color-text-faint)' }}>
              +{n.tags.length - 3}
            </span>
          )}
        </div>
      ),
    },
    {
      key: 'updated',
      label: 'Updated',
      render: (n) => {
        const d = new Date(n.updated_at);
        const display = isNaN(d.getTime()) ? n.updated_at : d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
        return <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>{display}</span>;
      },
      sortValue: (n) => n.updated_at,
      width: '80px',
    },
  ];
}

// ─── Completeness score ─────────────────────────────────────────────────────
function completenessScore(node: NodeSummary, edges: EdgePayload[]): number {
  const updatedMs = new Date(node.updated_at).getTime();
  const isFresh = !isNaN(updatedMs) && (Date.now() - updatedMs) < 90 * 86400000;
  const isConnected = edges.some(e => e.source === node.id || e.target === node.id);
  const hasProjectAssignment =
    (node.scope_class === 'project' || node.scope_class === 'project_contextual')
      ? Boolean(node.project_id?.trim())
      : true;
  const hasName = Boolean(node.name.trim());
  const hasStatus = Boolean(node.status.trim());
  const hasDocs = node.has_documentation;
  const hasTags = node.tags.length > 0;

  const criteriaByType: Record<string, boolean[]> = {
    decision: [hasName, hasStatus, hasTags, hasDocs, isConnected, isFresh, hasProjectAssignment],
    technology: [hasName, hasStatus, hasTags, hasDocs, isFresh, hasProjectAssignment],
    component: [hasName, hasStatus, hasTags, hasDocs, isConnected, isFresh, hasProjectAssignment],
    constraint: [hasName, hasStatus, hasTags, hasDocs, isConnected, isFresh, hasProjectAssignment],
    pattern: [hasName, hasStatus, hasTags, hasDocs, isConnected, isFresh, hasProjectAssignment],
    quality_requirement: [hasStatus, hasTags, hasDocs, isFresh, hasProjectAssignment],
  };

  const criteria = criteriaByType[node.node_type] ?? [hasName, hasStatus, hasTags, hasDocs, isFresh];
  const filled = criteria.filter(Boolean).length;
  return Math.round((filled / Math.max(criteria.length, 1)) * 100);
}

// ─── Component ──────────────────────────────────────────────────────────────

export default function NodeListPanel({
  nodes,
  edges,
  nodeType,
  onSelectNode,
  selectedNodeIds = [],
  onToggleSelectNode,
  onToggleSelectAllVisible,
  columns,
}: NodeListPanelProps) {
  const [search, setSearch] = useState('');
  const [sortKey, setSortKey] = useState('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');
  const selectAllRef = useRef<HTMLInputElement | null>(null);

  const cols = columns ?? defaultColumns(edges);
  const selectedNodeSet = useMemo(() => new Set(selectedNodeIds), [selectedNodeIds]);
  const selectionEnabled = Boolean(onToggleSelectNode);

  const filtered = useMemo(() => {
    let data = nodeType ? nodes.filter(n => n.node_type === nodeType) : [...nodes];
    if (search.trim()) {
      const q = search.toLowerCase();
      data = data.filter(n =>
        n.name.toLowerCase().includes(q) ||
        n.id.toLowerCase().includes(q) ||
        n.tags.some(t => t.toLowerCase().includes(q)) ||
        n.status.toLowerCase().includes(q) ||
        (n.project_id ?? '').toLowerCase().includes(q) ||
        (n.project_name ?? '').toLowerCase().includes(q) ||
        (n.scope_class ?? 'unscoped').toLowerCase().includes(q) ||
        (n.scope_visibility ?? 'unscoped').toLowerCase().includes(q) ||
        (n.secondary_scope?.feature ?? '').toLowerCase().includes(q) ||
        (n.secondary_scope?.widget ?? '').toLowerCase().includes(q) ||
        (n.secondary_scope?.artifact ?? '').toLowerCase().includes(q) ||
        (n.secondary_scope?.component ?? '').toLowerCase().includes(q)
      );
    }
    const col = cols.find(c => c.key === sortKey);
    if (col?.sortValue) {
      const sv = col.sortValue;
      data.sort((a, b) => {
        const va = sv(a, edges);
        const vb = sv(b, edges);
        if (va < vb) return sortDir === 'asc' ? -1 : 1;
        if (va > vb) return sortDir === 'asc' ? 1 : -1;
        return 0;
      });
    }
    return data;
  }, [nodes, edges, nodeType, search, sortKey, sortDir, cols]);

  const visibleNodeIds = useMemo(() => filtered.map(node => node.id), [filtered]);
  const allVisibleSelected = visibleNodeIds.length > 0 && visibleNodeIds.every(nodeId => selectedNodeSet.has(nodeId));
  const someVisibleSelected = !allVisibleSelected && visibleNodeIds.some(nodeId => selectedNodeSet.has(nodeId));

  useEffect(() => {
    if (!selectAllRef.current) return;
    selectAllRef.current.indeterminate = someVisibleSelected;
  }, [someVisibleSelected]);

  const handleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
  };

  // Stats
  const totalCount = nodeType ? nodes.filter(n => n.node_type === nodeType).length : nodes.length;
  const avgCompleteness = filtered.length > 0
    ? Math.round(filtered.reduce((sum, n) => sum + completenessScore(n, edges), 0) / filtered.length)
    : 0;

  return (
    <div className="node-list-panel">
      {/* Search + stats bar */}
      <div className="node-list-toolbar">
        <div className="node-list-search">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-faint)" strokeWidth="2" strokeLinecap="round">
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <input
            type="text"
            placeholder="Search by name, ID, or tag…"
            value={search}
            onChange={e => setSearch(e.target.value)}
            className="field-input"
            style={{ border: 'none', background: 'transparent', padding: '4px 0', fontSize: 'var(--text-xs)' }}
          />
        </div>
        <div className="node-list-stats">
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
            {filtered.length} of {totalCount} {nodeType ? TYPE_LABELS[nodeType] : 'nodes'}
          </span>
          <span style={{ fontSize: '0.5625rem', color: 'var(--color-text-faint)', marginLeft: 'var(--space-3)' }}>
            Completeness: {avgCompleteness}%
          </span>
        </div>
      </div>

      {/* Table */}
      <div style={{ flex: 1, overflow: 'auto' }}>
        <table className="data-table" style={{ width: '100%' }}>
          <thead>
            <tr>
              {selectionEnabled && (
                <th style={{ width: '40px' }}>
                  <input
                    ref={selectAllRef}
                    type="checkbox"
                    checked={allVisibleSelected}
                    aria-label="Select all visible nodes"
                    onChange={event => onToggleSelectAllVisible?.(visibleNodeIds, event.target.checked)}
                  />
                </th>
              )}
              {cols.map(col => (
                <th
                  key={col.key}
                  onClick={() => col.sortValue && handleSort(col.key)}
                  className={sortKey === col.key ? 'sorted' : ''}
                  style={{ cursor: col.sortValue ? 'pointer' : 'default', width: col.width }}
                >
                  {col.label}
                  {col.sortValue && (
                    <span className="sort-arrow">
                      {sortKey === col.key ? (sortDir === 'asc' ? '↑' : '↓') : '↕'}
                    </span>
                  )}
                </th>
              ))}
              <th style={{ width: '50px' }}>Score</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map(node => {
              const score = completenessScore(node, edges);
              const scoreColor = score >= 75 ? 'var(--color-success)' : score >= 50 ? 'var(--color-warning)' : 'var(--color-error)';
              return (
                <tr key={node.id} onClick={() => onSelectNode(node.id)}>
                  {selectionEnabled && (
                    <td onClick={event => event.stopPropagation()}>
                      <input
                        type="checkbox"
                        checked={selectedNodeSet.has(node.id)}
                        aria-label={`Select ${node.name}`}
                        onChange={event => onToggleSelectNode?.(node.id, event.target.checked)}
                      />
                    </td>
                  )}
                  {cols.map(col => (
                    <td key={col.key}>{col.render(node, edges)}</td>
                  ))}
                  <td>
                    <span style={{
                      fontSize: '0.5625rem',
                      fontWeight: 600,
                      color: scoreColor,
                      fontFamily: 'var(--font-mono)',
                    }}>
                      {score}%
                    </span>
                  </td>
                </tr>
              );
            })}
            {filtered.length === 0 && (
              <tr>
                <td colSpan={cols.length + (selectionEnabled ? 2 : 1)} style={{ textAlign: 'center', color: 'var(--color-text-faint)', padding: 'var(--space-8)' }}>
                  {search ? 'No nodes match your search' : 'No nodes in this category yet'}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
