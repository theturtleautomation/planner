import { useState } from 'react';
import type { BeliefState, Classification } from '../types.ts';
import ClassificationBadge from './ClassificationBadge.tsx';

interface BeliefStatePanelProps {
  beliefState: BeliefState | null;
  classification: Classification | null;
  onDimensionClick?: (dimension: string) => void;
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

export default function BeliefStatePanel({
  beliefState,
  classification,
  onDimensionClick,
}: BeliefStatePanelProps) {
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

  const filledEntries = Object.entries(beliefState.filled);
  const uncertainEntries = Object.entries(beliefState.uncertain);

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
            <div
              key={dim}
              onClick={() => onDimensionClick?.(dim)}
              style={{
                fontSize: '12px',
                color: 'var(--accent-green)',
                cursor: onDimensionClick ? 'pointer' : 'default',
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
              </span>
            </div>
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
            <div
              key={dim}
              onClick={() => onDimensionClick?.(dim)}
              style={{
                fontSize: '12px',
                color: 'var(--accent-yellow)',
                cursor: onDimensionClick ? 'pointer' : 'default',
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
                <span style={{ color: 'var(--text-primary)' }}>{slot.value}</span>
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
              onClick={() => onDimensionClick?.(dim)}
              style={{
                fontSize: '12px',
                color: 'var(--text-secondary)',
                cursor: onDimensionClick ? 'pointer' : 'default',
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
