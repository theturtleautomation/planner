import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import type { BeliefState, Classification, Contradiction } from '../types.ts';
import ClassificationBadge from './ClassificationBadge.tsx';

/**
 * Safely coerce any value to a display string.
 *
 * Serde-serialised Rust enums like `Dimension::Custom("X")` arrive as
 * `{"custom": "X"}` — rendering that object directly in JSX triggers
 * React error #31 ("Objects are not valid as a React child").
 *
 * This helper unwraps one level of nesting and falls back to
 * JSON.stringify so nothing ever reaches React as a raw object.
 */
function toDisplayString(v: unknown): string {
  if (typeof v === 'string') return v;
  if (typeof v === 'number' || typeof v === 'boolean') return String(v);
  if (v === null || v === undefined) return '';
  if (typeof v === 'object') {
    // Unwrap single-key wrapper objects like {"custom": "Browser Support"}
    const keys = Object.keys(v as Record<string, unknown>);
    if (keys.length === 1) {
      const inner = (v as Record<string, unknown>)[keys[0]];
      if (typeof inner === 'string') return inner;
    }
    // Unwrap nested {value: "..."} (e.g. SlotValue objects)
    if ('value' in (v as Record<string, unknown>)) {
      const inner = (v as Record<string, unknown>).value;
      if (typeof inner === 'string') return inner;
      return toDisplayString(inner);
    }
    return JSON.stringify(v);
  }
  return String(v);
}

interface BeliefStatePanelProps {
  beliefState: BeliefState | null;
  classification: Classification | null;
  contradictions?: Contradiction[];
  onDimensionClick?: (dimension: string) => void;
  onDimensionEdit?: (dimension: string, newValue: string) => void;
}

interface SectionProps {
  title: string;
  accentColor: string;
  count: number;
  children: React.ReactNode;
}

function Section({ title, accentColor, count, children }: SectionProps) {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div
      style={{
        borderTop: '1px solid var(--border)',
        paddingTop: '8px',
      }}
    >
      {/* Section header */}
      <button
        onClick={() => setCollapsed((c) => !c)}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          width: '100%',
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          padding: '0 0 6px 0',
          textAlign: 'left',
        }}
      >
        <span
          style={{
            fontSize: '11px',
            fontWeight: 700,
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            color: accentColor,
            fontFamily: 'inherit',
          }}
        >
          {title}
        </span>
        <span
          style={{
            fontSize: '10px',
            color: 'var(--text-secondary)',
            background: 'var(--bg-tertiary)',
            border: '1px solid var(--border)',
            borderRadius: '2px',
            padding: '0px 5px',
            fontFamily: 'inherit',
          }}
        >
          {count}
        </span>
        <span
          style={{
            marginLeft: 'auto',
            fontSize: '10px',
            color: 'var(--text-secondary)',
            fontFamily: 'inherit',
          }}
        >
          {collapsed ? '▸' : '▾'}
        </span>
      </button>

      {/* Items */}
      {!collapsed && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '4px',
            paddingBottom: '8px',
          }}
        >
          {children}
        </div>
      )}
    </div>
  );
}

/** Inline dimension editor — shown when a dimension is clicked. */
function DimensionEditor({
  dimension,
  currentValue,
  onSave,
  onCancel,
}: {
  dimension: string;
  currentValue: string;
  onSave: (newValue: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(currentValue);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent): void => {
    if (e.key === 'Enter') {
      const trimmed = value.trim();
      if (trimmed && trimmed !== currentValue) {
        onSave(trimmed);
      } else {
        onCancel();
      }
    } else if (e.key === 'Escape') {
      onCancel();
    }
  };

  return (
    <div
      style={{
        padding: '6px 8px',
        background: 'var(--bg-tertiary)',
        border: '1px solid var(--accent-cyan)',
        borderRadius: '3px',
        display: 'flex',
        flexDirection: 'column',
        gap: '4px',
      }}
    >
      <span
        style={{
          fontSize: '10px',
          fontWeight: 700,
          color: 'var(--accent-cyan)',
          letterSpacing: '0.06em',
          textTransform: 'uppercase',
        }}
      >
        Edit: {dimension}
      </span>
      <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
        <input
          ref={inputRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          style={{
            flex: 1,
            background: 'var(--bg-primary)',
            border: '1px solid var(--border)',
            borderRadius: '2px',
            color: 'var(--text-primary)',
            fontSize: '12px',
            padding: '4px 8px',
            fontFamily: 'inherit',
            outline: 'none',
          }}
        />
        <button
          onClick={() => {
            const trimmed = value.trim();
            if (trimmed && trimmed !== currentValue) onSave(trimmed);
            else onCancel();
          }}
          style={{
            background: 'var(--accent-cyan)',
            border: 'none',
            borderRadius: '2px',
            color: 'var(--bg-primary)',
            fontSize: '10px',
            fontWeight: 700,
            padding: '4px 10px',
            cursor: 'pointer',
            fontFamily: 'inherit',
          }}
        >
          Save
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
      <span style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>
        Enter to save, Escape to cancel
      </span>
    </div>
  );
}

export default function BeliefStatePanel({
  beliefState,
  classification,
  contradictions = [],
  onDimensionClick,
  onDimensionEdit,
}: BeliefStatePanelProps) {
  const [editingDimension, setEditingDimension] = useState<string | null>(null);
  const [editingCurrentValue, setEditingCurrentValue] = useState('');

  const handleDimensionClick = useCallback((dim: string, currentValue?: string) => {
    if (onDimensionEdit) {
      setEditingDimension(dim);
      setEditingCurrentValue(currentValue ?? '');
    } else if (onDimensionClick) {
      onDimensionClick(dim);
    }
  }, [onDimensionClick, onDimensionEdit]);

  const handleEditSave = useCallback((newValue: string) => {
    if (editingDimension && onDimensionEdit) {
      onDimensionEdit(editingDimension, newValue);
    }
    setEditingDimension(null);
  }, [editingDimension, onDimensionEdit]);

  // Derive entries before hooks that depend on them — safe even when beliefState is null
  const filledEntries = beliefState ? Object.entries(beliefState.filled) : [];
  const uncertainEntries = beliefState ? Object.entries(beliefState.uncertain) : [];

  // ALL hooks must execute on every render — never place hooks after an early return.
  // Track which dimensions just transitioned to filled/uncertain for CSS animations.
  const prevFilledRef = useRef<Set<string>>(new Set());
  const prevUncertainRef = useRef<Set<string>>(new Set());
  const justFilled = useMemo(() => {
    const prev = prevFilledRef.current;
    const current = new Set(filledEntries.map(([dim]) => dim));
    const newlyFilled = new Set<string>();
    for (const dim of current) {
      if (!prev.has(dim)) newlyFilled.add(dim);
    }
    prevFilledRef.current = current;
    return newlyFilled;
  }, [filledEntries]);

  const justUncertain = useMemo(() => {
    const prev = prevUncertainRef.current;
    const current = new Set(uncertainEntries.map(([dim]) => dim));
    const newlyUncertain = new Set<string>();
    for (const dim of current) {
      if (!prev.has(dim)) newlyUncertain.add(dim);
    }
    prevUncertainRef.current = current;
    return newlyUncertain;
  }, [uncertainEntries]);

  // Now safe to early-return — all 8 hooks have executed.
  if (!beliefState) {
    return (
      <div
        style={{
          flex: 1,
          background: 'var(--bg-secondary)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '20px',
        }}
      >
        <span
          style={{
            color: 'var(--text-secondary)',
            fontSize: '12px',
            textAlign: 'center',
            fontStyle: 'italic',
          }}
        >
          Belief state will appear here during the interview
        </span>
      </div>
    );
  }

  const canEdit = !!onDimensionEdit;

  /** Extract display value — handles strings, nested SlotValue objects, and serde enum wrappers. */
  const getSlotDisplayValue = (slot: unknown): string => toDisplayString(slot);

  return (
    <div
      style={{
        flex: 1,
        background: 'var(--bg-secondary)',
        overflowY: 'auto',
        padding: '12px 14px',
        display: 'flex',
        flexDirection: 'column',
        gap: '0',
      }}
    >
      {/* Classification badge */}
      {classification && (
        <div style={{ marginBottom: '12px' }}>
          <ClassificationBadge classification={classification} />
        </div>
      )}

      {/* Contradictions — shown at top when present */}
      {contradictions.length > 0 && (
        <Section
          title="! Contradictions"
          accentColor="var(--accent-red)"
          count={contradictions.length}
        >
          {contradictions.map((c, i) => (
            <div
              key={i}
              style={{
                fontSize: '12px',
                padding: '6px 8px',
                background: 'rgba(255,68,68,0.06)',
                border: '1px solid rgba(255,68,68,0.2)',
                borderRadius: '3px',
                display: 'flex',
                flexDirection: 'column',
                gap: '3px',
              }}
            >
              <div style={{ display: 'flex', gap: '6px', alignItems: 'baseline' }}>
                <span style={{ color: 'var(--accent-red)', flexShrink: 0, fontWeight: 700 }}>!</span>
                <span>
                  <span style={{ fontWeight: 700, color: 'var(--accent-red)' }}>
                    {c.dimension_a}
                  </span>
                  <span style={{ color: 'var(--text-secondary)', margin: '0 4px' }}>=</span>
                  <span style={{ color: 'var(--text-primary)' }}>"{c.value_a}"</span>
                  <span style={{ color: 'var(--text-secondary)', margin: '0 6px' }}>vs</span>
                  <span style={{ fontWeight: 700, color: 'var(--accent-red)' }}>
                    {c.dimension_b}
                  </span>
                  <span style={{ color: 'var(--text-secondary)', margin: '0 4px' }}>=</span>
                  <span style={{ color: 'var(--text-primary)' }}>"{c.value_b}"</span>
                </span>
              </div>
              <div
                style={{
                  fontSize: '11px',
                  color: 'var(--text-secondary)',
                  paddingLeft: '18px',
                  lineHeight: '1.4',
                }}
              >
                {c.explanation}
              </div>
            </div>
          ))}
        </Section>
      )}

      {/* Filled */}
      <Section
        title="✓ Filled"
        accentColor="var(--accent-green)"
        count={filledEntries.length}
      >
        {filledEntries.length === 0 ? (
          <span style={{ fontSize: '11px', color: 'var(--text-secondary)', fontStyle: 'italic' }}>
            none yet
          </span>
        ) : (
          filledEntries.map(([dim, slot]) => (
            editingDimension === dim ? (
              <DimensionEditor
                key={dim}
                dimension={dim}
                currentValue={editingCurrentValue}
                onSave={handleEditSave}
                onCancel={() => setEditingDimension(null)}
              />
            ) : (
              <div
                key={dim}
                className={justFilled.has(dim) ? 'slot-just-filled' : undefined}
                onClick={() => handleDimensionClick(dim, toDisplayString(slot.value))}
                title={canEdit ? 'Click to edit' : slot.source_quote ?? undefined}
                style={{
                  fontSize: '12px',
                  color: 'var(--accent-green)',
                  cursor: canEdit || onDimensionClick ? 'pointer' : 'default',
                  padding: '2px 0',
                  display: 'flex',
                  gap: '6px',
                  alignItems: 'baseline',
                }}
              >
                <span style={{ flexShrink: 0 }}>✓</span>
                <span>
                  <span style={{ fontWeight: 700 }}>{toDisplayString(dim)}</span>
                  <span style={{ color: 'var(--text-secondary)', margin: '0 4px' }}>:</span>
                  <span style={{ color: 'var(--text-primary)' }}>{toDisplayString(slot.value)}</span>
                  {slot.source_quote && (
                    <span
                      style={{
                        color: 'var(--text-secondary)',
                        fontSize: '10px',
                        marginLeft: '6px',
                        fontStyle: 'italic',
                      }}
                    >
                      "{slot.source_quote}"
                    </span>
                  )}
                </span>
              </div>
            )
          ))
        )}
      </Section>

      {/* Uncertain */}
      <Section
        title="? Uncertain"
        accentColor="var(--accent-yellow)"
        count={uncertainEntries.length}
      >
        {uncertainEntries.length === 0 ? (
          <span style={{ fontSize: '11px', color: 'var(--text-secondary)', fontStyle: 'italic' }}>
            none
          </span>
        ) : (
          uncertainEntries.map(([dim, slot]) => (
            editingDimension === dim ? (
              <DimensionEditor
                key={dim}
                dimension={dim}
                currentValue={editingCurrentValue}
                onSave={handleEditSave}
                onCancel={() => setEditingDimension(null)}
              />
            ) : (
              <div
                key={dim}
                className={justUncertain.has(dim) ? 'slot-just-uncertain' : undefined}
                onClick={() => handleDimensionClick(dim, getSlotDisplayValue(slot.value))}
                title={canEdit ? 'Click to edit' : undefined}
                style={{
                  fontSize: '12px',
                  color: 'var(--accent-yellow)',
                  cursor: canEdit || onDimensionClick ? 'pointer' : 'default',
                  padding: '2px 0',
                  display: 'flex',
                  gap: '6px',
                  alignItems: 'baseline',
                }}
              >
                <span style={{ flexShrink: 0 }}>?</span>
                <span>
                  <span style={{ fontWeight: 700 }}>{toDisplayString(dim)}</span>
                  <span style={{ color: 'var(--text-secondary)', margin: '0 4px' }}>:</span>
                  <span style={{ color: 'var(--text-primary)' }}>
                    {getSlotDisplayValue(slot.value)}
                  </span>
                  <span
                    style={{
                      color: 'var(--text-secondary)',
                      fontSize: '10px',
                      marginLeft: '4px',
                    }}
                  >
                    ({Math.round(slot.confidence * 100)}%)
                  </span>
                </span>
              </div>
            )
          ))
        )}
      </Section>

      {/* Missing */}
      <Section
        title="○ Missing"
        accentColor="var(--text-secondary)"
        count={beliefState.missing.length}
      >
        {beliefState.missing.length === 0 ? (
          <span style={{ fontSize: '11px', color: 'var(--text-secondary)', fontStyle: 'italic' }}>
            none
          </span>
        ) : (
          beliefState.missing.map((dim) => (
            <div
              key={toDisplayString(dim)}
              onClick={() => handleDimensionClick(toDisplayString(dim))}
              style={{
                fontSize: '12px',
                color: 'var(--text-secondary)',
                cursor: canEdit || onDimensionClick ? 'pointer' : 'default',
                padding: '2px 0',
                display: 'flex',
                gap: '6px',
                alignItems: 'baseline',
              }}
            >
              <span style={{ flexShrink: 0 }}>○</span>
              <span style={{ fontWeight: 700 }}>{toDisplayString(dim)}</span>
            </div>
          ))
        )}
      </Section>

      {/* Out of Scope */}
      <Section
        title="✗ Out of Scope"
        accentColor="var(--border)"
        count={beliefState.out_of_scope.length}
      >
        {beliefState.out_of_scope.length === 0 ? (
          <span style={{ fontSize: '11px', color: 'var(--text-secondary)', fontStyle: 'italic' }}>
            none
          </span>
        ) : (
          beliefState.out_of_scope.map((dim) => (
            <div
              key={toDisplayString(dim)}
              style={{
                fontSize: '12px',
                color: 'var(--border)',
                padding: '2px 0',
                display: 'flex',
                gap: '6px',
                alignItems: 'baseline',
                opacity: 0.6,
              }}
            >
              <span style={{ flexShrink: 0 }}>✗</span>
              <span>{toDisplayString(dim)}</span>
            </div>
          ))
        )}
      </Section>
    </div>
  );
}
