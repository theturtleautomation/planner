import { memo, useMemo } from 'react';
import PromptOptionGroup from './PromptOptionGroup.tsx';
import SeamlessInput from './SeamlessInput.tsx';
import { syncSocraticDocumentDraft } from '../stores/socraticDocumentStore.ts';
import { selectQuestionDraft, useSocraticDraftStore } from '../stores/useSocraticDraftStore.ts';
import type { PromptItem } from '../types.ts';

interface QuestionBlockProps {
  promptId: string;
  item: PromptItem;
  index: number;
  disabled?: boolean;
  autoFocusTextarea?: boolean;
}

export interface QuestionBlockAnswerDraft {
  selectedOptionId: string | null;
  customText: string;
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

function QuestionBlock({
  promptId,
  item,
  index,
  disabled = false,
  autoFocusTextarea = false,
}: QuestionBlockProps) {
  const selector = useMemo(
    () => (state: Parameters<typeof selectQuestionDraft>[0]) => selectQuestionDraft(state, promptId, item.item_id),
    [item.item_id, promptId],
  );
  const draft = useSocraticDraftStore(selector);
  const setSelectedOption = useSocraticDraftStore((state) => state.setSelectedOption);
  const setCustomText = useSocraticDraftStore((state) => state.setCustomText);
  const targetLabel = formatDimensionLabel(item.target_dimension);

  return (
    <section className="socratic-question-block" aria-label={`Question ${index + 1}`}>
      <header className="socratic-question-block__header">
        <div className="socratic-question-block__meta">
          <span className="socratic-question-block__index">Q{index + 1}</span>
          <span className="socratic-question-block__kind">{itemKindLabel(item.kind)}</span>
          {item.required && (
            <span className="socratic-question-block__required">Required</span>
          )}
        </div>

        <p className="socratic-question-block__text">{item.text}</p>

        {(targetLabel || item.section_ref) && (
          <div className="socratic-question-block__tags">
            {targetLabel && (
              <span className="socratic-question-block__tag">{targetLabel}</span>
            )}
            {item.section_ref && (
              <span className="socratic-question-block__tag">{item.section_ref}</span>
            )}
          </div>
        )}
      </header>

      <PromptOptionGroup
        ariaLabel={`Prompt options for ${item.item_id}`}
        options={item.options}
        selectedOptionId={draft.selectedOptionId}
        onSelect={(optionId) => {
          setSelectedOption(promptId, item.item_id, optionId);
          syncSocraticDocumentDraft(item.item_id, {
            selectedOptionId: optionId,
            customText: draft.customText,
          });
        }}
        disabled={disabled}
      />

      <label className="socratic-question-block__input-group">
        <span className="socratic-question-block__input-label">Your answer</span>
        <SeamlessInput
          value={draft.customText}
          onChange={(nextValue) => {
            setCustomText(promptId, item.item_id, nextValue);
            syncSocraticDocumentDraft(item.item_id, {
              selectedOptionId: draft.selectedOptionId,
              customText: nextValue,
            });
          }}
          disabled={disabled}
          autoFocus={autoFocusTextarea}
          ariaLabel={`Custom text for ${item.item_id}`}
        />
      </label>
    </section>
  );
}

export default memo(QuestionBlock);
