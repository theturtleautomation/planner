import { useState, useCallback } from 'react';
import type { EdgeType, NodeSummary } from '../types/blueprint.ts';

// ─── Props ──────────────────────────────────────────────────────────────────

interface AddEdgeModalProps {
  isOpen: boolean;
  nodes: NodeSummary[];
  /** Pre-fill the source node (e.g. when adding from the detail drawer). */
  defaultSourceId?: string | null;
  onClose: () => void;
  onCreate: (edge: { source: string; target: string; edge_type: EdgeType; metadata?: string }) => Promise<void>;
}

// ─── Constants ──────────────────────────────────────────────────────────────

const EDGE_TYPES: { value: EdgeType; label: string; description: string }[] = [
  { value: 'decided_by',  label: 'Decided By',  description: 'Tech/Comp/Pattern → Decision' },
  { value: 'supersedes',  label: 'Supersedes',   description: 'Decision → Decision' },
  { value: 'depends_on',  label: 'Depends On',   description: 'Component → Component' },
  { value: 'uses',        label: 'Uses',          description: 'Component → Technology' },
  { value: 'constrains',  label: 'Constrains',    description: 'Constraint → Decision/Comp/Tech' },
  { value: 'implements',  label: 'Implements',     description: 'Component → Pattern' },
  { value: 'satisfies',   label: 'Satisfies',      description: 'Decision/Pattern → QualityRequirement' },
  { value: 'affects',     label: 'Affects',        description: 'Decision → Component/Technology' },
];

const NODE_TYPE_LABELS: Record<string, string> = {
  decision: 'DEC',
  technology: 'TECH',
  component: 'COMP',
  constraint: 'CON',
  pattern: 'PAT',
  quality_requirement: 'QR',
};

// ─── Component ──────────────────────────────────────────────────────────────

export default function AddEdgeModal({ isOpen, nodes, defaultSourceId, onClose, onCreate }: AddEdgeModalProps) {
  const [sourceId, setSourceId] = useState('');
  const [targetId, setTargetId] = useState('');
  const [edgeType, setEdgeType] = useState<EdgeType>('depends_on');
  const [metadata, setMetadata] = useState('');
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Reset form when modal opens
  const prevOpen = useState(isOpen)[0];
  if (isOpen && !prevOpen) {
    // Can't use useEffect for this without a ref; just let state initialize
  }

  // Initialize source from defaultSourceId when opening
  useState(() => {
    if (defaultSourceId) setSourceId(defaultSourceId);
  });

  const handleCreate = useCallback(async () => {
    if (!sourceId || !targetId) {
      setError('Source and target are required');
      return;
    }
    if (sourceId === targetId) {
      setError('Source and target must be different nodes');
      return;
    }
    setError(null);
    setCreating(true);
    try {
      await onCreate({
        source: sourceId,
        target: targetId,
        edge_type: edgeType,
        metadata: metadata.trim() || undefined,
      });
      // Reset and close
      setSourceId('');
      setTargetId('');
      setEdgeType('depends_on');
      setMetadata('');
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create edge');
    } finally {
      setCreating(false);
    }
  }, [sourceId, targetId, edgeType, metadata, onCreate, onClose]);

  const handleClose = useCallback(() => {
    setError(null);
    onClose();
  }, [onClose]);

  if (!isOpen) return null;

  const sortedNodes = [...nodes].sort((a, b) => a.name.localeCompare(b.name));

  return (
    <>
      <div className="modal-backdrop" onClick={handleClose} />
      <div className="modal" style={{ maxWidth: '480px' }}>
        <div className="modal-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 'var(--space-4)' }}>
          <h3 style={{ margin: 0, fontSize: 'var(--text-base)', fontWeight: 600 }}>Add Edge</h3>
          <button className="modal-close" onClick={handleClose}>&times;</button>
        </div>

        <label className="field-label">Source Node</label>
        <select className="field-input" value={sourceId} onChange={e => setSourceId(e.target.value)}>
          <option value="">Select source…</option>
          {sortedNodes.map(n => (
            <option key={n.id} value={n.id}>
              [{NODE_TYPE_LABELS[n.node_type] ?? n.node_type}] {n.name}
            </option>
          ))}
        </select>

        <label className="field-label" style={{ marginTop: 'var(--space-3)' }}>Edge Type</label>
        <select className="field-input" value={edgeType} onChange={e => setEdgeType(e.target.value as EdgeType)}>
          {EDGE_TYPES.map(et => (
            <option key={et.value} value={et.value}>
              {et.label} — {et.description}
            </option>
          ))}
        </select>

        <label className="field-label" style={{ marginTop: 'var(--space-3)' }}>Target Node</label>
        <select className="field-input" value={targetId} onChange={e => setTargetId(e.target.value)}>
          <option value="">Select target…</option>
          {sortedNodes.map(n => (
            <option key={n.id} value={n.id}>
              [{NODE_TYPE_LABELS[n.node_type] ?? n.node_type}] {n.name}
            </option>
          ))}
        </select>

        <label className="field-label" style={{ marginTop: 'var(--space-3)' }}>Why this relationship? (optional)</label>
        <input className="field-input" placeholder="e.g. 'primary data store', 'mandated by compliance'" value={metadata} onChange={e => setMetadata(e.target.value)} />

        {/* Visual preview */}
        {sourceId && targetId && (
          <div style={{
            marginTop: 'var(--space-4)',
            padding: 'var(--space-3)',
            background: 'var(--color-surface-raised)',
            borderRadius: 'var(--radius-md)',
            fontSize: 'var(--text-xs)',
            fontFamily: 'var(--font-mono)',
            textAlign: 'center',
            color: 'var(--color-text-muted)',
          }}>
            {nodes.find(n => n.id === sourceId)?.name ?? sourceId}
            {' '}
            <span style={{ color: 'var(--color-accent)' }}>—[{edgeType}]→</span>
            {' '}
            {nodes.find(n => n.id === targetId)?.name ?? targetId}
          </div>
        )}

        {error && (
          <div style={{ color: 'var(--color-error)', fontSize: 'var(--text-xs)', marginTop: 'var(--space-3)' }}>
            {error}
          </div>
        )}

        <div style={{ display: 'flex', gap: 'var(--space-3)', justifyContent: 'flex-end', marginTop: 'var(--space-5)' }}>
          <button className="btn btn-outline" onClick={handleClose} disabled={creating}>Cancel</button>
          <button className="btn btn-primary" onClick={handleCreate} disabled={creating || !sourceId || !targetId}>
            {creating ? 'Creating…' : 'Create Edge'}
          </button>
        </div>
      </div>
    </>
  );
}
