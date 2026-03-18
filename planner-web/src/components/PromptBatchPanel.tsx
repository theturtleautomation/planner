import { useEffect, useMemo, useState } from 'react';
import PromptCard, { type PromptCardAnswerDraft } from './PromptCard.tsx';
import type { PromptAnswer, PromptEnvelope } from '../types.ts';

interface PromptBatchPanelProps {
  prompt: PromptEnvelope;
  onSubmit: (promptId: string, answers: PromptAnswer[]) => void;
  onDone?: () => void;
  disabled?: boolean;
}

type DraftAnswerMap = Record<string, PromptCardAnswerDraft>;

function blankDraftAnswer(): PromptCardAnswerDraft {
  return {
    selectedOptionId: null,
    customText: '',
  };
}

function isAnswered(draft: PromptCardAnswerDraft | undefined): boolean {
  if (!draft) return false;
  if (draft.selectedOptionId) return true;
  return draft.customText.trim().length > 0;
}

function collectAnswers(prompt: PromptEnvelope, drafts: DraftAnswerMap): PromptAnswer[] {
  const answers: PromptAnswer[] = [];

  for (const item of prompt.items) {
    const draft = drafts[item.item_id];
    if (!isAnswered(draft)) continue;

    const customText = draft.customText.trim();
    answers.push({
      item_id: item.item_id,
      selected_option_id: draft.selectedOptionId ?? undefined,
      custom_text: customText.length > 0 ? customText : undefined,
    });
  }

  return answers;
}

export default function PromptBatchPanel({
  prompt,
  onSubmit,
  onDone,
  disabled = false,
}: PromptBatchPanelProps) {
  const [drafts, setDrafts] = useState<DraftAnswerMap>({});

  useEffect(() => {
    setDrafts({});
  }, [prompt.prompt_id]);

  const requiredItemIds = useMemo(() => {
    const explicit = prompt.required_item_ids ?? [];
    if (explicit.length > 0) {
      return explicit;
    }
    return prompt.items.filter((item) => item.required).map((item) => item.item_id);
  }, [prompt.items, prompt.required_item_ids]);

  const answeredCount = useMemo(
    () => prompt.items.filter((item) => isAnswered(drafts[item.item_id])).length,
    [drafts, prompt.items],
  );

  const missingRequiredCount = useMemo(
    () => requiredItemIds.filter((itemId) => !isAnswered(drafts[itemId])).length,
    [drafts, requiredItemIds],
  );

  const answers = useMemo(() => collectAnswers(prompt, drafts), [drafts, prompt]);

  const canSubmit = !disabled
    && answers.length > 0
    && (prompt.allow_partial_submit || missingRequiredCount === 0);

  const submit = (): void => {
    if (!canSubmit) return;

    const nextAnswers = collectAnswers(prompt, drafts);
    if (nextAnswers.length === 0) return;

    onSubmit(prompt.prompt_id, nextAnswers);

    setDrafts((previous) => {
      const next = { ...previous };
      for (const answer of nextAnswers) {
        delete next[answer.item_id];
      }
      return next;
    });
  };

  return (
    <div
      style={{
        borderTop: '1px solid var(--color-border)',
        background: 'var(--color-surface)',
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        padding: '12px 16px',
        flexShrink: 0,
      }}
    >
      <header style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '10px' }}>
          <span
            style={{
              fontSize: '11px',
              fontWeight: 700,
              letterSpacing: '0.08em',
              textTransform: 'uppercase',
              color: 'var(--color-primary)',
            }}
          >
            {prompt.title}
          </span>
          <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
            {answeredCount}/{prompt.items.length} answered
          </span>
        </div>

        {prompt.instructions && (
          <p
            style={{
              margin: 0,
              fontSize: '12px',
              color: 'var(--color-text-muted)',
              lineHeight: 1.45,
            }}
          >
            {prompt.instructions}
          </p>
        )}
      </header>

      <div
        style={{
          display: 'grid',
          gap: '10px',
          gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
          maxHeight: '40vh',
          overflowY: 'auto',
          paddingRight: '2px',
        }}
      >
        {prompt.items.map((item) => (
          <PromptCard
            key={item.item_id}
            item={item}
            answer={drafts[item.item_id] ?? blankDraftAnswer()}
            onChange={(next) => {
              setDrafts((previous) => ({
                ...previous,
                [item.item_id]: next,
              }));
            }}
            disabled={disabled}
          />
        ))}
      </div>

      <footer
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '10px',
          flexWrap: 'wrap',
        }}
      >
        <span style={{ fontSize: '11px', color: 'var(--color-text-muted)' }}>
          {prompt.allow_partial_submit
            ? 'Submit any answered cards. Unanswered cards can be sent later.'
            : missingRequiredCount > 0
              ? `Answer ${missingRequiredCount} required card${missingRequiredCount === 1 ? '' : 's'} before submitting.`
              : 'All required cards are answered.'}
        </span>

        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {onDone && (
            <button
              type="button"
              onClick={onDone}
              disabled={disabled}
              style={{
                background: 'transparent',
                border: '1px solid var(--color-success)',
                borderRadius: '3px',
                color: 'var(--color-success)',
                fontSize: '11px',
                fontFamily: 'inherit',
                letterSpacing: '0.04em',
                padding: '6px 12px',
                cursor: disabled ? 'not-allowed' : 'pointer',
              }}
            >
              Done
            </button>
          )}

          <button
            type="button"
            onClick={submit}
            disabled={!canSubmit}
            style={{
              background: canSubmit ? 'var(--color-primary)' : 'transparent',
              border: `1px solid ${canSubmit ? 'var(--color-primary)' : 'var(--color-border)'}`,
              borderRadius: '3px',
              color: canSubmit ? 'var(--color-bg)' : 'var(--color-text-muted)',
              fontSize: '12px',
              fontWeight: 700,
              fontFamily: 'inherit',
              letterSpacing: '0.03em',
              padding: '7px 14px',
              cursor: canSubmit ? 'pointer' : 'not-allowed',
            }}
            aria-label="Submit prompt answers"
          >
            Submit answered items
          </button>
        </div>
      </footer>
    </div>
  );
}

export { collectAnswers };
