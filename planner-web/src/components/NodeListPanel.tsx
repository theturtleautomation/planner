import { useState, useMemo } from 'react';
import type { NodeSummary, EdgePayload, NodeType } from '../types/blueprint.ts';

// ─── Props ──────────────────────────────────────────────────────────────────

interface NodeListPanelProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  nodeType: NodeType | null;
  onSelectNode: (nodeId: string) => void;
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

function defaultColumns(edges: EdgePayload[]): ColumnDef[] {
  return [
    {
      key: 'name',
      label: 'Name',
      render: (n) => <span style={{ fontWeight: 500 }}>{n.name}</span>,
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

function completenessScore(node: NodeSummary): number {
  let filled = 0;
  let total = 4; // name, status, type, at least 1 tag
  if (node.name && node.name.trim()) filled++;
  if (node.status && node.status.trim()) filled++;
  if (node.node_type) filled++;
  if (node.tags.length > 0) filled++;
  return Math.round((filled / total) * 100);
}

// ─── Component ──────────────────────────────────────────────────────────────

export default function NodeListPanel({ nodes, edges, nodeType, onSelectNode, columns }: NodeListPanelProps) {
  const [search, setSearch] = useState('');
  const [sortKey, setSortKey] = useState('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  const cols = columns ?? defaultColumns(edges);

  const filtered = useMemo(() => {
    let data = nodeType ? nodes.filter(n => n.node_type === nodeType) : [...nodes];
    if (search.trim()) {
      const q = search.toLowerCase();
      data = data.filter(n =>
        n.name.toLowerCase().includes(q) ||
        n.id.toLowerCase().includes(q) ||
        n.tags.some(t => t.toLowerCase().includes(q)) ||
        n.status.toLowerCase().includes(q)
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
    ? Math.round(filtered.reduce((sum, n) => sum + completenessScore(n), 0) / filtered.length)
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
              const score = completenessScore(node);
              const scoreColor = score >= 75 ? 'var(--color-success)' : score >= 50 ? 'var(--color-warning)' : 'var(--color-error)';
              return (
                <tr key={node.id} onClick={() => onSelectNode(node.id)}>
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
                <td colSpan={cols.length + 1} style={{ textAlign: 'center', color: 'var(--color-text-faint)', padding: 'var(--space-8)' }}>
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
