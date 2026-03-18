import type { SpeculativeDraft } from '../types.ts';

/** Safely coerce serde enum wrappers (e.g. {"custom": "X"}) to plain strings. */
function toDisplayString(v: unknown): string {
  if (typeof v === 'string') return v;
  if (typeof v === 'number' || typeof v === 'boolean') return String(v);
  if (v === null || v === undefined) return '';
  if (typeof v === 'object') {
    const keys = Object.keys(v as Record<string, unknown>);
    if (keys.length === 1) {
      const inner = (v as Record<string, unknown>)[keys[0]];
      if (typeof inner === 'string') return inner;
    }
    return JSON.stringify(v);
  }
  return String(v);
}

interface SpeculativeDraftViewProps {
  draft: SpeculativeDraft;
  onBack: () => void;
}

export default function SpeculativeDraftView({ draft, onBack }: SpeculativeDraftViewProps) {
  return (
    <div
      style={{
        flex: 1,
        background: 'var(--color-surface)',
        overflowY: 'auto',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '10px 16px',
          borderBottom: '1px solid var(--color-border)',
          flexShrink: 0,
          background: 'var(--color-surface)',
          position: 'sticky',
          top: 0,
          zIndex: 1,
        }}
      >
        <span
          style={{
            fontSize: '11px',
            fontWeight: 700,
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            color: 'var(--color-primary)',
          }}
        >
          Draft Spec
          <span
            style={{
              color: 'var(--color-text-muted)',
              fontWeight: 400,
              marginLeft: '8px',
              textTransform: 'none',
              letterSpacing: '0',
            }}
          >
            (review alongside prompt items)
          </span>
        </span>

        <button
          onClick={onBack}
          style={{
            background: 'var(--color-surface-2)',
            border: '1px solid var(--color-border)',
            borderRadius: '3px',
            color: 'var(--color-text-muted)',
            fontSize: '11px',
            fontFamily: 'inherit',
            letterSpacing: '0.04em',
            padding: '4px 10px',
            cursor: 'pointer',
            transition: 'border-color 0.15s ease, color 0.15s ease',
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--color-primary)';
            (e.currentTarget as HTMLButtonElement).style.color = 'var(--color-primary)';
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--color-border)';
            (e.currentTarget as HTMLButtonElement).style.color = 'var(--color-text-muted)';
          }}
        >
          Back to State
        </button>
      </div>

      <div
        style={{
          padding: '16px 20px',
          display: 'flex',
          flexDirection: 'column',
          gap: '20px',
        }}
      >
        {draft.sections.map((section, i) => (
          <div
            key={i}
            className="draft-section-enter"
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
            }}
          >
            <div
              style={{
                fontSize: '12px',
                fontWeight: 700,
                color: 'var(--color-primary)',
                letterSpacing: '0.04em',
                textTransform: 'uppercase',
              }}
            >
              {section.heading}
            </div>

            <div
              style={{
                fontSize: '13px',
                color: 'var(--color-text)',
                lineHeight: '1.7',
                paddingLeft: '10px',
                borderLeft: '2px solid var(--color-border)',
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
              }}
            >
              {section.content}
            </div>
          </div>
        ))}

        {draft.not_discussed.length > 0 && (
          <div
            style={{
              borderTop: '1px solid var(--color-border)',
              paddingTop: '14px',
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
            }}
          >
            <div
              style={{
                fontSize: '11px',
                fontWeight: 700,
                letterSpacing: '0.08em',
                textTransform: 'uppercase',
                color: 'var(--color-text-muted)',
              }}
            >
              Not yet discussed:
            </div>
            <div
              style={{
                display: 'flex',
                flexWrap: 'wrap',
                gap: '6px',
              }}
            >
              {draft.not_discussed.map((dim, i) => (
                <span
                  key={i}
                  style={{
                    fontSize: '11px',
                    color: 'var(--color-text-muted)',
                    background: 'var(--color-surface-2)',
                    border: '1px solid var(--color-border)',
                    borderRadius: '3px',
                    padding: '3px 8px',
                  }}
                >
                  {toDisplayString(dim)}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
