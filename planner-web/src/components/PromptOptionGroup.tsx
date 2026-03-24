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
      className="socratic-option-group"
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
            className={[
              'socratic-option-group__option',
              isSelected ? 'is-selected' : '',
              isHovered ? 'is-hovered' : '',
            ].filter(Boolean).join(' ')}
          >
            <span
              aria-hidden="true"
              className="socratic-option-group__dot"
            />
            <span className="socratic-option-group__label">{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}
