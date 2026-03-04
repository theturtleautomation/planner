import type { SpeculativeDraft } from '../types.ts';

interface SpeculativeDraftViewProps {
  draft: SpeculativeDraft;
  onBack: () => void;
}

export default function SpeculativeDraftView({ draft, onBack }: SpeculativeDraftViewProps) {
  return (
    <div
      style={{
        flex: 1,
        background: 'var(--bg-secondary)',
        overflowY: 'auto',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '10px 16px',
          borderBottom: '1px solid var(--border)',
          flexShrink: 0,
          background: 'var(--bg-secondary)',
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
            color: 'var(--accent-cyan)',
          }}
        >
          Draft Spec
          <span
            style={{
              color: 'var(--text-secondary)',
              fontWeight: 400,
              marginLeft: '8px',
              textTransform: 'none',
              letterSpacing: '0',
            }}
          >
            (review and correct)
          </span>
        </span>

        <button
          onClick={onBack}
          style={{
            background: 'var(--bg-tertiary)',
            border: '1px solid var(--border)',
            borderRadius: '3px',
            color: 'var(--text-secondary)',
            fontSize: '11px',
            fontFamily: 'inherit',
            letterSpacing: '0.04em',
            padding: '4px 10px',
            cursor: 'pointer',
            transition: 'border-color 0.15s ease, color 0.15s ease',
          }}
          onMouseEnter={(e) => {
            (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--accent-cyan)';
            (e.currentTarget as HTMLButtonElement).style.color = 'var(--accent-cyan)';
          }}
          onMouseLeave={(e) => {
            (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--border)';
            (e.currentTarget as HTMLButtonElement).style.color = 'var(--text-secondary)';
          }}
        >
          ← Back to State
        </button>
      </div>

      {/* Body */}
      <div
        style={{
          padding: '16px 20px',
          display: 'flex',
          flexDirection: 'column',
          gap: '20px',
        }}
      >
        {/* Draft sections */}
        {draft.sections.map((section, i) => (
          <div
            key={i}
            style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}
          >
            <div
              style={{
                fontSize: '12px',
                fontWeight: 700,
                color: 'var(--accent-cyan)',
                letterSpacing: '0.04em',
                textTransform: 'uppercase',
              }}
            >
              {section.heading}
            </div>
            <div
              style={{
                fontSize: '13px',
                color: 'var(--text-primary)',
                lineHeight: '1.7',
                paddingLeft: '10px',
                borderLeft: '2px solid var(--border)',
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
              }}
            >
              {section.content}
            </div>
          </div>
        ))}

        {/* Assumptions */}
        {draft.assumptions.length > 0 && (
          <div
            style={{
              borderTop: '1px solid var(--border)',
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
                color: 'var(--accent-yellow)',
              }}
            >
              Assumptions (unconfirmed):
            </div>
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: '4px',
              }}
            >
              {draft.assumptions.map((a, i) => (
                <div
                  key={i}
                  style={{
                    fontSize: '12px',
                    color: 'var(--accent-yellow)',
                    display: 'flex',
                    gap: '6px',
                    alignItems: 'baseline',
                  }}
                >
                  <span style={{ flexShrink: 0 }}>?</span>
                  <span>
                    <span style={{ fontWeight: 700 }}>{a.dimension}</span>
                    <span style={{ color: 'var(--text-secondary)', margin: '0 6px' }}>—</span>
                    <span style={{ color: 'var(--text-primary)', fontWeight: 400 }}>
                      {a.assumption}
                    </span>
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Not yet discussed */}
        {draft.not_discussed.length > 0 && (
          <div
            style={{
              borderTop: '1px solid var(--border)',
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
                color: 'var(--text-secondary)',
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
                    color: 'var(--text-secondary)',
                    background: 'var(--bg-tertiary)',
                    border: '1px solid var(--border)',
                    borderRadius: '3px',
                    padding: '3px 8px',
                  }}
                >
                  {dim}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
