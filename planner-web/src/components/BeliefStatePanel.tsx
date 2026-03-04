import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import type { BeliefState, Classification, Contradiction } from '../types.ts';
import ClassificationBadge from './ClassificationBadge.tsx';

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

  /** Extract display value from uncertain slot (handles both nested and flat formats). */
  const getUncertainValue = (slot: { value: unknown; confidence: number }): string => {
    if (typeof slot.value === 'string') return slot.value;
    if (slot.value && typeof slot.value === 'object' && 'value' in slot.value) {
      return String((slot.value as { value: unknown }).value);
    }
    return String(slot.value);
  };

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
                onClick={() => handleDimensionClick(dim, slot.value)}
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
                  <span style={{ fontWeight: 700 }}>{dim}</span>
                  <span style={{ color: 'var(--text-secondary)', margin: '0 4px' }}>:</span>
                  <span style={{ color: 'var(--text-primary)' }}>{slot.value}</span>
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
                onClick={() => handleDimensionClick(dim, getUncertainValue(slot))}
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
                  <span style={{ fontWeight: 700 }}>{dim}</span>
                  <span style={{ color: 'var(--text-secondary)', margin: '0 4px' }}>:</span>
                  <span style={{ color: 'var(--text-primary)' }}>
                    {getUncertainValue(slot)}
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
              key={dim}
              onClick={() => handleDimensionClick(dim)}
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
              <span style={{ fontWeight: 700 }}>{dim}</span>
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
              key={dim}
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
              <span>{dim}</span>
            </div>
          ))
        )}
      </Section>
    </div>
  );
}
