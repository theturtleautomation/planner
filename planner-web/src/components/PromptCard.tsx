import PromptOptionGroup from './PromptOptionGroup.tsx';
import type { PromptItem } from '../types.ts';

export interface PromptCardAnswerDraft {
  selectedOptionId: string | null;
  customText: string;
}

interface PromptCardProps {
  item: PromptItem;
  answer: PromptCardAnswerDraft;
  onChange: (next: PromptCardAnswerDraft) => void;
  disabled?: boolean;
}

function formatDimensionLabel(value: unknown): string | null {
  if (typeof value === 'string') return value;
  if (value && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>);
    if (entries.length === 1 && typeof entries[0][1] === 'string') {
      return entries[0][1] as string;
    }
    return JSON.stringify(value);
  }
  return null;
}

function itemKindLabel(kind: PromptItem['kind']): string {
  switch (kind) {
    case 'verification':
      return 'Verification';
    case 'contradiction':
      return 'Contradiction';
    case 'draft_section':
      return 'Draft review';
    case 'discovery':
    default:
      return 'Discovery';
  }
}

export default function PromptCard({
  item,
  answer,
  onChange,
  disabled = false,
}: PromptCardProps) {
  const targetLabel = formatDimensionLabel(item.target_dimension);

  return (
    <article
      style={{
        border: '1px solid var(--color-border)',
        background: 'var(--color-surface)',
        borderRadius: '4px',
        padding: '12px',
        display: 'flex',
        flexDirection: 'column',
        gap: '10px',
      }}
    >
      <header style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px' }}>
          <span
            style={{
              fontSize: '10px',
              fontWeight: 700,
              letterSpacing: '0.07em',
              textTransform: 'uppercase',
              color: 'var(--color-primary)',
            }}
          >
            {itemKindLabel(item.kind)}
          </span>
          {item.required && (
            <span
              style={{
                fontSize: '10px',
                color: 'var(--color-text-muted)',
                letterSpacing: '0.04em',
                textTransform: 'uppercase',
              }}
            >
              Required
            </span>
          )}
        </div>

        <p style={{ margin: 0, fontSize: '13px', color: 'var(--color-text)', lineHeight: 1.55 }}>
          {item.text}
        </p>

        {(targetLabel || item.section_ref) && (
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', alignItems: 'center' }}>
            {targetLabel && (
              <span
                style={{
                  fontSize: '10px',
                  color: 'var(--color-text-muted)',
                  border: '1px solid var(--color-border)',
                  borderRadius: '999px',
                  padding: '2px 8px',
                }}
              >
                {targetLabel}
              </span>
            )}
            {item.section_ref && (
              <span
                style={{
                  fontSize: '10px',
                  color: 'var(--color-text-muted)',
                  border: '1px solid var(--color-border)',
                  borderRadius: '999px',
                  padding: '2px 8px',
                }}
              >
                {item.section_ref}
              </span>
            )}
          </div>
        )}
      </header>

      <PromptOptionGroup
        ariaLabel={`Prompt options for ${item.item_id}`}
        options={item.options}
        selectedOptionId={answer.selectedOptionId}
        onSelect={(optionId) => {
          onChange({
            ...answer,
            selectedOptionId: optionId,
          });
        }}
        disabled={disabled}
      />

      <label style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
        <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>Optional details</span>
        <textarea
          value={answer.customText}
          onChange={(event) => {
            onChange({
              ...answer,
              customText: event.target.value,
            });
          }}
          placeholder="Add context or correction"
          rows={2}
          disabled={disabled}
          aria-label={`Custom text for ${item.item_id}`}
          style={{
            width: '100%',
            boxSizing: 'border-box',
            background: 'var(--color-surface-2)',
            border: '1px solid var(--color-border)',
            borderRadius: '3px',
            color: 'var(--color-text)',
            fontSize: '12px',
            lineHeight: 1.45,
            resize: 'vertical',
            minHeight: '56px',
            padding: '8px 10px',
            fontFamily: 'inherit',
            outline: 'none',
          }}
        />
      </label>
    </article>
  );
}
