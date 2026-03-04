import { useState } from 'react';
import type { QuickOption } from '../types.ts';

interface QuickOptionsProps {
  options: QuickOption[];
  onSelect: (value: string) => void;
  disabled?: boolean;
}

export default function QuickOptions({ options, onSelect, disabled = false }: QuickOptionsProps) {
  const [hovered, setHovered] = useState<string | null>(null);

  if (options.length === 0) return null;

  return (
    <div
      style={{
        display: 'flex',
        flexWrap: 'wrap',
        gap: '6px',
        padding: '8px 0 4px 0',
      }}
    >
      {options.map((option) => {
        const isHovered = hovered === option.value;

        return (
          <button
            key={option.value}
            onClick={() => !disabled && onSelect(option.value)}
            onMouseEnter={() => !disabled && setHovered(option.value)}
            onMouseLeave={() => setHovered(null)}
            disabled={disabled}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              padding: '5px 12px',
              background: isHovered ? 'rgba(0,212,255,0.08)' : 'var(--bg-tertiary)',
              border: `1px solid ${isHovered ? 'var(--accent-cyan)' : 'var(--border)'}`,
              borderRadius: '3px',
              color: isHovered ? 'var(--accent-cyan)' : 'var(--text-primary)',
              fontSize: '11px',
              fontFamily: 'inherit',
              letterSpacing: '0.03em',
              cursor: disabled ? 'not-allowed' : 'pointer',
              opacity: disabled ? 0.5 : 1,
              transition: 'border-color 0.15s ease, background 0.15s ease, color 0.15s ease',
              outline: 'none',
              whiteSpace: 'nowrap',
            }}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
