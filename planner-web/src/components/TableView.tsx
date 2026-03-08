import { useState, useMemo } from 'react';
import type { NodeSummary, EdgePayload, NodeType } from '../types/blueprint.ts';
import { labelNodeType } from '../lib/taxonomy.ts';

const ALL_TYPES: (NodeType | 'all')[] = ['all', 'decision', 'technology', 'component', 'constraint', 'pattern', 'quality_requirement'];

interface TableViewProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  filterType: NodeType | null;
  onSelectNode: (nodeId: string) => void;
}

type SortCol = 'name' | 'type' | 'status' | 'id';
type SortDir = 'asc' | 'desc';

export default function TableView({ nodes, edges, filterType, onSelectNode }: TableViewProps) {
  const [sortCol, setSortCol] = useState<SortCol>('name');
  const [sortDir, setSortDir] = useState<SortDir>('asc');
  const [localFilter, setLocalFilter] = useState<NodeType | 'all'>('all');

  // Effective filter: sidebar filter overrides local filter
  const effectiveFilter = filterType ?? (localFilter === 'all' ? null : localFilter);

  const filteredNodes = useMemo(() => {
    let data = effectiveFilter ? nodes.filter(n => n.node_type === effectiveFilter) : [...nodes];
    data.sort((a, b) => {
      let va: string, vb: string;
      switch (sortCol) {
        case 'name': va = a.name.toLowerCase(); vb = b.name.toLowerCase(); break;
        case 'type': va = a.node_type; vb = b.node_type; break;
        case 'status': va = (a.status ?? '').toLowerCase(); vb = (b.status ?? '').toLowerCase(); break;
        case 'id': va = a.id; vb = b.id; break;
        default: va = ''; vb = '';
      }
      if (va < vb) return sortDir === 'asc' ? -1 : 1;
      if (va > vb) return sortDir === 'asc' ? 1 : -1;
      return 0;
    });
    return data;
  }, [nodes, effectiveFilter, sortCol, sortDir]);

  const getConnections = (nodeId: string) => {
    return edges.filter(e => e.source === nodeId || e.target === nodeId).length;
  };

  const handleSort = (col: SortCol) => {
    if (sortCol === col) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortCol(col);
      setSortDir('asc');
    }
  };

  return (
    <div style={{
      width: '100%', height: '100%', overflowY: 'auto',
      overscrollBehavior: 'contain', padding: 'var(--space-4) var(--space-5)',
    }}>
      {/* Filter chips */}
      <div style={{ display: 'flex', gap: 'var(--space-2)', marginBottom: 'var(--space-4)', flexWrap: 'wrap' }}>
        {ALL_TYPES.map(t => {
          const isActive = effectiveFilter === null ? t === 'all' : t === effectiveFilter;
          return (
            <button
              key={t}
              className={`filter-chip${isActive ? ' active' : ''}`}
              onClick={() => setLocalFilter(t === 'all' ? 'all' : t as NodeType)}
            >
              {t === 'all' ? 'All' : labelNodeType(t, 'short')}
            </button>
          );
        })}
      </div>

      {/* Table */}
      <table className="data-table">
        <thead>
          <tr>
            <th
              onClick={() => handleSort('name')}
              className={sortCol === 'name' ? 'sorted' : ''}
            >
              Name <span className="sort-arrow">↕</span>
            </th>
            <th
              onClick={() => handleSort('type')}
              className={sortCol === 'type' ? 'sorted' : ''}
            >
              Type <span className="sort-arrow">↕</span>
            </th>
            <th
              onClick={() => handleSort('status')}
              className={sortCol === 'status' ? 'sorted' : ''}
            >
              Status <span className="sort-arrow">↕</span>
            </th>
            <th
              onClick={() => handleSort('id')}
              className={sortCol === 'id' ? 'sorted' : ''}
            >
              ID <span className="sort-arrow">↕</span>
            </th>
            <th>Connections</th>
          </tr>
        </thead>
        <tbody>
          {filteredNodes.map(n => (
            <tr
              key={n.id}
              onClick={() => onSelectNode(n.id)}
              tabIndex={0}
              role="button"
              onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelectNode(n.id); } }}
            >
              <td style={{ fontWeight: 500, color: 'var(--color-text)' }}>{n.name}</td>
              <td><span className={`badge badge-${n.node_type}`}>{labelNodeType(n.node_type, 'short')}</span></td>
              <td><span className={`status-badge status-${(n.status ?? '').toLowerCase().replace(/\s+/g, '-')}`}>{n.status ?? ''}</span></td>
              <td style={{ fontFamily: 'var(--font-mono)', fontSize: '0.6875rem', color: 'var(--color-text-faint)' }}>{n.id}</td>
              <td style={{ fontVariantNumeric: 'tabular-nums', color: 'var(--color-text-faint)' }}>{getConnections(n.id)}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {filteredNodes.length === 0 && (
        <div style={{
          textAlign: 'center', padding: 'var(--space-8)', color: 'var(--color-text-faint)',
          fontSize: 'var(--text-sm)',
        }}>
          No nodes match the current filter
        </div>
      )}
    </div>
  );
}
