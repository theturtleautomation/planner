import { useState, useCallback, useEffect } from 'react';
import type { ImportProvider } from '../types.ts';

interface ImportProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  onImport: (provider: ImportProvider, sourceRef: string) => Promise<void>;
}

export default function ImportProjectModal({
  isOpen,
  onClose,
  onImport,
}: ImportProjectModalProps) {
  const [provider, setProvider] = useState<ImportProvider>('github');
  const [sourceRef, setSourceRef] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setProvider('github');
      setSourceRef('');
      setSaving(false);
      setError(null);
    }
  }, [isOpen]);

  const handleClose = useCallback(() => {
    if (!saving) {
      onClose();
    }
  }, [onClose, saving]);

  const handleSubmit = useCallback(async () => {
    const trimmed = sourceRef.trim();
    if (!trimmed) {
      setError('Source reference is required');
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await onImport(provider, trimmed);
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSaving(false);
    }
  }, [handleClose, onImport, provider, sourceRef]);

  if (!isOpen) return null;

  const sourceLabel = provider === 'github' ? 'GitHub URL' : 'Local Absolute Path';
  const placeholder = provider === 'github'
    ? 'https://github.com/org/repo'
    : '/absolute/path/to/repo';
  const helperText = provider === 'github'
    ? 'This slice clones the public GitHub repo default branch, analyzes the checkout, and seeds a draft planning session.'
    : 'This slice validates the absolute local path, analyzes it in place, and seeds the same draft planning session without creating a managed clone.';

  return (
    <div className="modal-backdrop" onClick={handleClose}>
      <div className="modal" onClick={(event) => event.stopPropagation()} style={{ maxWidth: '440px' }}>
        <div className="modal-header">
          <div>
            <div className="modal-title">Import Existing Project</div>
            <p className="modal-copy">
              Seed a planning-ready project from an existing repository without changing the surrounding workspace flow.
            </p>
          </div>
          <button className="modal-close" onClick={handleClose} disabled={saving}>&times;</button>
        </div>
        <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          <label className="field-label">
            Provider
            <select
              className="field-input"
              value={provider}
              onChange={(event) => setProvider(event.target.value as ImportProvider)}
              aria-label="Provider"
            >
              <option value="github">GitHub</option>
              <option value="local">Local</option>
            </select>
          </label>
          <label className="field-label">
            {sourceLabel}
            <input
              className="field-input"
              value={sourceRef}
              onChange={(event) => setSourceRef(event.target.value)}
              placeholder={placeholder}
              aria-label={sourceLabel}
              autoFocus
            />
          </label>
          <div style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-xs)' }}>
            {helperText}
          </div>
          {error && (
            <div
              style={{
                padding: 'var(--space-2) var(--space-3)',
                background: 'var(--color-error-bg, rgba(255,59,48,0.1))',
                color: 'var(--color-error)',
                borderRadius: 'var(--radius-sm)',
                fontSize: 'var(--text-xs)',
              }}
            >
              {error}
            </div>
          )}
        </div>
        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={handleClose} disabled={saving}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={handleSubmit} disabled={saving}>
            {saving ? 'Queueing…' : 'Queue Import'}
          </button>
        </div>
      </div>
    </div>
  );
}
