import { useState } from 'react';
import type { PromptOption } from '../types.ts';

interface PromptOptionGroupProps {
  options: PromptOption[];
  selectedOptionId: string | null;
  onSelect: (optionId: string | null) => void;
  disabled?: boolean;
  ariaLabel: string;
}

export default function PromptOptionGroup({
  options,
  selectedOptionId,
  onSelect,
  disabled = false,
  ariaLabel,
}: PromptOptionGroupProps) {
  const [hoveredOptionId, setHoveredOptionId] = useState<string | null>(null);

  if (options.length === 0) return null;

  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '6px',
      }}
    >
      {options.map((option) => {
        const isSelected = selectedOptionId === option.option_id;
        const isHovered = hoveredOptionId === option.option_id;

        return (
          <button
            key={option.option_id}
            type="button"
            role="radio"
            aria-checked={isSelected}
            onClick={() => {
              if (disabled) return;
              onSelect(isSelected ? null : option.option_id);
            }}
            onMouseEnter={() => {
              if (!disabled) {
                setHoveredOptionId(option.option_id);
              }
            }}
            onMouseLeave={() => setHoveredOptionId(null)}
            disabled={disabled}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              width: '100%',
              textAlign: 'left',
              background: isSelected
                ? 'rgba(0,212,255,0.12)'
                : isHovered
                  ? 'rgba(0,212,255,0.07)'
                  : 'var(--color-surface-2)',
              border: `1px solid ${isSelected || isHovered ? 'var(--color-primary)' : 'var(--color-border)'}`,
              borderRadius: '3px',
              padding: '8px 10px',
              color: isSelected || isHovered ? 'var(--color-primary)' : 'var(--color-text)',
              fontSize: '12px',
              fontFamily: 'inherit',
              lineHeight: 1.4,
              cursor: disabled ? 'not-allowed' : 'pointer',
              opacity: disabled ? 0.6 : 1,
              transition: 'border-color 0.15s ease, background 0.15s ease, color 0.15s ease',
            }}
          >
            <span
              aria-hidden="true"
              style={{
                width: '14px',
                height: '14px',
                borderRadius: '50%',
                border: `1px solid ${isSelected ? 'var(--color-primary)' : 'var(--color-border)'}`,
                background: isSelected ? 'var(--color-primary)' : 'transparent',
                flexShrink: 0,
              }}
            />
            <span>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}
