import { useState, useCallback } from 'react';

interface DeleteNodeDialogProps {
  isOpen: boolean;
  nodeId: string | null;
  nodeName: string | null;
  onClose: () => void;
  onConfirm: (nodeId: string) => Promise<void>;
}

export default function DeleteNodeDialog({
  isOpen,
  nodeId,
  nodeName,
  onClose,
  onConfirm,
}: DeleteNodeDialogProps) {
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleConfirm = useCallback(async () => {
    if (!nodeId) return;
    setDeleting(true);
    setError(null);
    try {
      await onConfirm(nodeId);
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeleting(false);
    }
  }, [nodeId, onConfirm, onClose]);

  const handleClose = useCallback(() => {
    setError(null);
    setDeleting(false);
    onClose();
  }, [onClose]);

  if (!isOpen || !nodeId) return null;

  return (
    <div className="modal-backdrop" onClick={handleClose}>
      <div
        className="modal"
        onClick={e => e.stopPropagation()}
        style={{ maxWidth: '420px' }}
      >
        <div className="modal-header">
          <div className="modal-title" style={{ color: 'var(--color-error)' }}>Delete Node</div>
          <button className="modal-close" onClick={handleClose}>&times;</button>
        </div>

        <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          <p style={{ margin: 0, color: 'var(--color-text)', fontSize: 'var(--text-sm)' }}>
            Are you sure you want to delete <strong>{nodeName ?? nodeId}</strong>?
          </p>
          <p style={{
            margin: 0,
            color: 'var(--color-text-faint)',
            fontSize: 'var(--text-xs)',
            lineHeight: 1.6,
          }}>
            This will permanently remove the node and all edges connected to it.
            This action cannot be undone.
          </p>

          {error && (
            <div style={{
              padding: 'var(--space-2) var(--space-3)',
              background: 'var(--color-error-bg, rgba(255,59,48,0.1))',
              color: 'var(--color-error)',
              borderRadius: 'var(--radius-sm)',
              fontSize: 'var(--text-xs)',
            }}>
              {error}
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={handleClose} disabled={deleting}>
            Cancel
          </button>
          <button
            className="btn"
            onClick={handleConfirm}
            disabled={deleting}
            style={{
              background: 'var(--color-error)',
              color: '#fff',
              border: 'none',
            }}
          >
            {deleting ? 'Deleting…' : 'Delete Node'}
          </button>
        </div>
      </div>
    </div>
  );
}
