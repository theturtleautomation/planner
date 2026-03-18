import { useState, useCallback, useEffect } from 'react';

interface CreateProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string, description?: string) => Promise<void>;
}

export default function CreateProjectModal({ isOpen, onClose, onCreate }: CreateProjectModalProps) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setName('');
      setDescription('');
      setError(null);
      setSaving(false);
    }
  }, [isOpen]);

  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  const handleSubmit = useCallback(async () => {
    const trimmedName = name.trim();
    if (!trimmedName) {
      setError('Project name is required');
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await onCreate(trimmedName, description.trim() || undefined);
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSaving(false);
    }
  }, [name, description, onCreate, handleClose]);

  if (!isOpen) return null;

  return (
    <div className="modal-backdrop" onClick={handleClose}>
      <div
        className="modal"
        onClick={e => e.stopPropagation()}
        style={{ maxWidth: '400px' }}
      >
        <div className="modal-header">
          <div className="modal-title">Create Project</div>
          <button className="modal-close" onClick={handleClose}>&times;</button>
        </div>
        <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          <label className="field-label">
            Project Name
            <input
              className="field-input"
              value={name}
              onChange={e => setName(e.target.value)}
              placeholder="Enter project name..."
              autoFocus
            />
          </label>
          <label className="field-label">
            Description (Optional)
            <textarea
              className="field-input"
              value={description}
              onChange={e => setDescription(e.target.value)}
              placeholder="Short description of the project..."
              rows={3}
            />
          </label>

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
          <button className="btn btn-ghost" onClick={handleClose} disabled={saving}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={handleSubmit} disabled={saving}>
            {saving ? 'Creating…' : 'Create Project'}
          </button>
        </div>
      </div>
    </div>
  );
}
