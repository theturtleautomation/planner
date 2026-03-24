import { memo, useEffect, useMemo, useRef, useState } from 'react';
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
  const [customText, setLocalCustomText] = useState(draft.customText);
  const customTextRef = useRef(customText);
  const selectedOptionRef = useRef(draft.selectedOptionId);
  const lastFlushedTextRef = useRef(draft.customText);
  const isDirtyRef = useRef(false);

  customTextRef.current = customText;
  selectedOptionRef.current = draft.selectedOptionId;

  const flushDraftText = (nextText: string): void => {
    if (lastFlushedTextRef.current === nextText) {
      isDirtyRef.current = false;
      return;
    }
    setCustomText(promptId, item.item_id, nextText);
    syncSocraticDocumentDraft(item.item_id, {
      selectedOptionId: selectedOptionRef.current,
      customText: nextText,
    });
    lastFlushedTextRef.current = nextText;
    isDirtyRef.current = false;
  };

  useEffect(() => {
    setLocalCustomText(draft.customText);
    customTextRef.current = draft.customText;
    selectedOptionRef.current = draft.selectedOptionId;
    lastFlushedTextRef.current = draft.customText;
    isDirtyRef.current = false;
  }, [draft.customText, draft.selectedOptionId, item.item_id, promptId]);

  useEffect(() => {
    if (!isDirtyRef.current) return undefined;
    const timeout = window.setTimeout(() => {
      flushDraftText(customTextRef.current);
    }, 150);
    return () => window.clearTimeout(timeout);
  }, [customText]);

  useEffect(() => (
    () => {
      if (!isDirtyRef.current) return;
      flushDraftText(customTextRef.current);
    }
  ), [item.item_id, promptId]);

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
          selectedOptionRef.current = optionId;
          setSelectedOption(promptId, item.item_id, optionId);
          syncSocraticDocumentDraft(item.item_id, {
            selectedOptionId: optionId,
            customText: customTextRef.current,
          });
        }}
        disabled={disabled}
      />

      <label className="socratic-question-block__input-group">
        <span className="socratic-question-block__input-label">Your answer</span>
        <SeamlessInput
          value={customText}
          onChange={(nextValue) => {
            const wasAnswered = customTextRef.current.trim().length > 0;
            const willBeAnswered = nextValue.trim().length > 0;
            setLocalCustomText(nextValue);
            customTextRef.current = nextValue;
            isDirtyRef.current = true;
            if (wasAnswered !== willBeAnswered) {
              flushDraftText(nextValue);
            }
          }}
          onBlur={() => flushDraftText(customTextRef.current)}
          disabled={disabled}
          autoFocus={autoFocusTextarea}
          ariaLabel={`Custom text for ${item.item_id}`}
        />
      </label>
    </section>
  );
}

export default memo(QuestionBlock);
