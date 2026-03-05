import { useEffect } from 'react';
import type { ImpactReport } from '../types/blueprint.ts';

interface ImpactPreviewModalProps {
  isOpen: boolean;
  report: ImpactReport | null;
  loading: boolean;
  onClose: () => void;
  onApply: () => void;
}

export default function ImpactPreviewModal({
  isOpen,
  report,
  loading,
  onClose,
  onApply,
}: ImpactPreviewModalProps) {
  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [isOpen, onClose]);

  const actionSymbol = (action: string) => {
    switch (action) {
      case 'add': return '+';
      case 'update': return '~';
      case 'reconverge': return '~';
      case 'remove': return '✗';
      case 'invalidate': return '⛔';
      default: return '·';
    }
  };

  const actionClass = (action: string) => {
    switch (action) {
      case 'add': return 'var(--color-success)';
      case 'update':
      case 'reconverge': return 'var(--color-gold)';
      case 'remove': return 'var(--color-error)';
      case 'invalidate': return '#e05555';
      default: return 'var(--color-text-faint)';
    }
  };

  const tagLabel = (action: string) => {
    switch (action) {
      case 'reconverge': return 'RECONVERGE';
      case 'update': return 'UPDATE';
      case 'add': return 'ADD';
      case 'remove': return 'REMOVE';
      case 'invalidate': return 'INVALIDATE';
      default: return action.toUpperCase();
    }
  };

  return (
    <div
      className={`modal-overlay${isOpen ? ' open' : ''}`}
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="modal">
        <div className="modal-header">
          <div className="modal-title">Impact Preview</div>
          <button
            className="drawer-close"
            onClick={onClose}
            aria-label="Close modal"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6L6 18M6 6l12 12"/>
            </svg>
          </button>
        </div>

        <div className="modal-body">
          {loading && (
            <div style={{
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              minHeight: '300px',
            }}>
              <div className="skeleton-pulse" />
            </div>
          )}

          {!loading && !report && (
            <div className="impact-terminal" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--color-text-faint)' }}>
              No impact data available
            </div>
          )}

          {!loading && report && (
            <div className="impact-terminal">
              <div style={{ color: 'var(--color-text)', fontWeight: 600, marginBottom: 'var(--space-1)' }}>
                Impact Plan: {report.source_node_name}
              </div>
              <div style={{ color: 'var(--color-text-faint)', marginBottom: 'var(--space-3)' }}>
                {'━'.repeat(60)}
              </div>

              {/* Summary */}
              <div style={{ marginBottom: 'var(--space-5)', color: 'var(--color-text-muted)' }}>
                <strong style={{ color: 'var(--color-text)' }}>Summary: </strong>
                {Object.entries(report.summary).map(([action, count]) =>
                  `${count} ${action}`
                ).join(', ')}
              </div>

              {/* Entries */}
              {report.entries.map((entry, i) => (
                <div key={i} style={{ marginBottom: 'var(--space-3)' }}>
                  <div style={{ display: 'flex', marginBottom: 'var(--space-1)' }}>
                    <span style={{
                      width: '20px', flexShrink: 0, fontWeight: 600,
                      color: actionClass(entry.action),
                    }}>
                      {actionSymbol(entry.action)}
                    </span>
                    <span style={{
                      width: '320px', flexShrink: 0,
                      whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                    }}>
                      {entry.node_type.toUpperCase()} {entry.node_id}
                    </span>
                    <span style={{
                      fontWeight: 600, marginLeft: 'var(--space-2)',
                      color: actionClass(entry.action),
                    }}>
                      [{tagLabel(entry.action)}]
                    </span>
                    {(entry.action === 'reconverge' || entry.severity === 'deep') && (
                      <span style={{ color: 'var(--color-warning)', marginLeft: 'var(--space-1)' }}>⚠</span>
                    )}
                  </div>
                  <div style={{
                    color: 'var(--color-text-faint)',
                    paddingLeft: '20px', fontSize: '0.75rem',
                  }}>
                    {entry.explanation}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-outline" onClick={onClose}>Cancel</button>
          <button className="btn btn-warning" onClick={onApply}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
            </svg>
            Apply & Reconverge
          </button>
        </div>
      </div>
    </div>
  );
}
