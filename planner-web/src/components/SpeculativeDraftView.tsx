import { useState, useCallback } from 'react';
import type { SpeculativeDraft } from '../types.ts';

interface SpeculativeDraftViewProps {
  draft: SpeculativeDraft;
  onBack: () => void;
  onReact?: (target: string, action: string, correction?: string) => void;
}

/** Small inline correction input shown when "Fix" is clicked. */
function CorrectionInput({
  onSubmit,
  onCancel,
}: {
  onSubmit: (correction: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState('');

  return (
    <div
      style={{
        marginTop: '6px',
        display: 'flex',
        gap: '6px',
        alignItems: 'flex-end',
      }}
    >
      <textarea
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !e.shiftKey && value.trim()) {
            e.preventDefault();
            onSubmit(value.trim());
          } else if (e.key === 'Escape') {
            onCancel();
          }
        }}
        placeholder="Describe what should change…"
        rows={2}
        autoFocus
        style={{
          flex: 1,
          background: 'var(--bg-primary)',
          border: '1px solid var(--accent-yellow)',
          borderRadius: '2px',
          color: 'var(--text-primary)',
          fontSize: '12px',
          padding: '6px 8px',
          fontFamily: 'inherit',
          resize: 'none',
          outline: 'none',
          lineHeight: '1.5',
        }}
      />
      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
        <button
          onClick={() => value.trim() && onSubmit(value.trim())}
          disabled={!value.trim()}
          style={{
            background: value.trim() ? 'var(--accent-yellow)' : 'transparent',
            border: `1px solid ${value.trim() ? 'var(--accent-yellow)' : 'var(--border)'}`,
            borderRadius: '2px',
            color: value.trim() ? 'var(--bg-primary)' : 'var(--text-secondary)',
            fontSize: '10px',
            fontWeight: 700,
            padding: '4px 10px',
            cursor: value.trim() ? 'pointer' : 'not-allowed',
            fontFamily: 'inherit',
          }}
        >
          Submit
        </button>
        <button
          onClick={onCancel}
          style={{
            background: 'transparent',
            border: '1px solid var(--border)',
            borderRadius: '2px',
            color: 'var(--text-secondary)',
            fontSize: '10px',
            padding: '4px 10px',
            cursor: 'pointer',
            fontFamily: 'inherit',
          }}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

export default function SpeculativeDraftView({ draft, onBack, onReact }: SpeculativeDraftViewProps) {
  // Track which section/assumptions has an open correction input.
  const [fixingTarget, setFixingTarget] = useState<string | null>(null);
  // Track which sections/assumptions have been reacted to (for visual feedback).
  const [reacted, setReacted] = useState<Set<string>>(new Set());

  const handleCorrect = useCallback((target: string) => {
    onReact?.(target, 'correct');
    setReacted((prev) => new Set(prev).add(target));
    setFixingTarget(null);
  }, [onReact]);

  const handleFix = useCallback((target: string) => {
    setFixingTarget(target);
  }, []);

  const handleFixSubmit = useCallback((target: string, correction: string) => {
    onReact?.(target, 'fix', correction);
    setReacted((prev) => new Set(prev).add(target));
    setFixingTarget(null);
  }, [onReact]);

  const handleConfirmAllAssumptions = useCallback(() => {
    onReact?.('assumptions', 'confirm_all');
    setReacted((prev) => new Set(prev).add('assumptions'));
    setFixingTarget(null);
  }, [onReact]);

  const handleFixAssumptions = useCallback(() => {
    setFixingTarget('assumptions');
  }, []);

  const handleFixAssumptionsSubmit = useCallback((correction: string) => {
    onReact?.('assumptions', 'fix_these', correction);
    setReacted((prev) => new Set(prev).add('assumptions'));
    setFixingTarget(null);
  }, [onReact]);

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
        {draft.sections.map((section, i) => {
          const target = String(i);
          const isReacted = reacted.has(target);
          const isFixing = fixingTarget === target;

          return (
            <div
              key={i}
              className="draft-section-enter"
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: '6px',
                opacity: isReacted ? 0.6 : 1,
                transition: 'opacity 0.3s',
              }}
            >
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                }}
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

                {/* Reaction buttons */}
                {onReact && !isReacted && (
                  <div style={{ display: 'flex', gap: '4px' }}>
                    <button
                      onClick={() => handleCorrect(target)}
                      style={{
                        background: 'transparent',
                        border: '1px solid var(--accent-green)',
                        borderRadius: '2px',
                        color: 'var(--accent-green)',
                        fontSize: '10px',
                        fontFamily: 'inherit',
                        padding: '2px 8px',
                        cursor: 'pointer',
                        letterSpacing: '0.03em',
                      }}
                    >
                      ✓ Correct
                    </button>
                    <button
                      onClick={() => handleFix(target)}
                      style={{
                        background: 'transparent',
                        border: '1px solid var(--accent-yellow)',
                        borderRadius: '2px',
                        color: 'var(--accent-yellow)',
                        fontSize: '10px',
                        fontFamily: 'inherit',
                        padding: '2px 8px',
                        cursor: 'pointer',
                        letterSpacing: '0.03em',
                      }}
                    >
                      ✎ Fix
                    </button>
                  </div>
                )}

                {isReacted && (
                  <span style={{ fontSize: '10px', color: 'var(--accent-green)' }}>
                    ✓ reviewed
                  </span>
                )}
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

              {isFixing && (
                <CorrectionInput
                  onSubmit={(correction) => handleFixSubmit(target, correction)}
                  onCancel={() => setFixingTarget(null)}
                />
              )}
            </div>
          );
        })}

        {/* Assumptions */}
        {draft.assumptions.length > 0 && (
          <div
            style={{
              borderTop: '1px solid var(--border)',
              paddingTop: '14px',
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
              opacity: reacted.has('assumptions') ? 0.6 : 1,
              transition: 'opacity 0.3s',
            }}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
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

              {/* Assumption reaction buttons */}
              {onReact && !reacted.has('assumptions') && (
                <div style={{ display: 'flex', gap: '4px' }}>
                  <button
                    onClick={handleConfirmAllAssumptions}
                    style={{
                      background: 'transparent',
                      border: '1px solid var(--accent-green)',
                      borderRadius: '2px',
                      color: 'var(--accent-green)',
                      fontSize: '10px',
                      fontFamily: 'inherit',
                      padding: '2px 8px',
                      cursor: 'pointer',
                      letterSpacing: '0.03em',
                    }}
                  >
                    ✓ Confirm All
                  </button>
                  <button
                    onClick={handleFixAssumptions}
                    style={{
                      background: 'transparent',
                      border: '1px solid var(--accent-yellow)',
                      borderRadius: '2px',
                      color: 'var(--accent-yellow)',
                      fontSize: '10px',
                      fontFamily: 'inherit',
                      padding: '2px 8px',
                      cursor: 'pointer',
                      letterSpacing: '0.03em',
                    }}
                  >
                    ✎ Fix These
                  </button>
                </div>
              )}

              {reacted.has('assumptions') && (
                <span style={{ fontSize: '10px', color: 'var(--accent-green)' }}>
                  ✓ reviewed
                </span>
              )}
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

            {fixingTarget === 'assumptions' && (
              <CorrectionInput
                onSubmit={handleFixAssumptionsSubmit}
                onCancel={() => setFixingTarget(null)}
              />
            )}
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
