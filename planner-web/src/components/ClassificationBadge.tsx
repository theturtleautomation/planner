import type { Classification } from '../types.ts';

const TYPE_ICON: Record<string, string> = {
  'Web App': '🌐',
  'Mobile App': '📱',
  'API': '🔌',
  'CLI': '⌨️',
  'Desktop App': '🖥️',
  'Data Pipeline': '🔄',
  'ML Model': '🤖',
};

interface ClassificationBadgeProps {
  classification: Classification;
}

export default function ClassificationBadge({ classification }: ClassificationBadgeProps) {
  const icon = TYPE_ICON[classification.project_type] ?? '📦';

  return (
    <div
      style={{
        display: 'inline-flex',
        flexDirection: 'column',
        gap: '3px',
        background: 'var(--bg-tertiary)',
        border: '1px solid var(--border)',
        borderRadius: '3px',
        padding: '8px 12px',
      }}
    >
      {/* Top row: icon + type · complexity */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
        <span style={{ fontSize: '12px' }}>{icon}</span>
        <span
          style={{
            fontSize: '12px',
            color: 'var(--accent-cyan)',
            fontWeight: 700,
            letterSpacing: '0.02em',
          }}
        >
          {classification.project_type}
        </span>
        <span style={{ color: 'var(--border)', fontSize: '12px' }}>·</span>
        <span
          style={{
            fontSize: '11px',
            color: 'var(--text-secondary)',
            textTransform: 'capitalize',
          }}
        >
          {classification.complexity}
        </span>
      </div>

      {/* Bottom row: budget */}
      <div
        style={{
          fontSize: '10px',
          color: 'var(--text-secondary)',
          letterSpacing: '0.04em',
          textTransform: 'uppercase',
        }}
      >
        Budget:{' '}
        <span style={{ color: 'var(--text-primary)' }}>
          ~{classification.question_budget} questions
        </span>
      </div>
    </div>
  );
}
