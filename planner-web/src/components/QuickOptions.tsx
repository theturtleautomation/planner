import { useState } from 'react';
import type { QuickOption } from '../types.ts';

interface QuickOptionsProps {
  options: QuickOption[];
  selectedValues?: string[];
  onToggle: (value: string) => void;
  disabled?: boolean;
}

export default function QuickOptions({
  options,
  selectedValues = [],
  onToggle,
  disabled = false,
}: QuickOptionsProps) {
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
        const isSelected = selectedValues.includes(option.value);

        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={isSelected}
            onClick={() => !disabled && onToggle(option.value)}
            onMouseEnter={() => !disabled && setHovered(option.value)}
            onMouseLeave={() => setHovered(null)}
            disabled={disabled}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              padding: '5px 12px',
              background: isSelected
                ? 'rgba(0,212,255,0.14)'
                : isHovered
                  ? 'rgba(0,212,255,0.08)'
                  : 'var(--color-surface-2)',
              border: `1px solid ${
                isSelected || isHovered ? 'var(--color-primary)' : 'var(--color-border)'
              }`,
              borderRadius: '3px',
              color: isSelected || isHovered ? 'var(--color-primary)' : 'var(--color-text)',
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
            {isSelected ? '✓ ' : ''}
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
