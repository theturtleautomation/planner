import { useEffect, useMemo } from 'react';
import { useShallow } from 'zustand/react/shallow';
import QuestionBlock from './QuestionBlock.tsx';
import { getSocraticDocumentPromptDrafts } from '../stores/socraticDocumentStore.ts';
import {
  collectPromptAnswers,
  selectPromptProgress,
  useSocraticDraftStore,
} from '../stores/useSocraticDraftStore.ts';
import type { PromptAnswer, PromptEnvelope } from '../types.ts';

export interface QuestionCanvasProps {
  prompt: PromptEnvelope;
  onSubmit: (promptId: string, answers: PromptAnswer[]) => void;
  onDone?: () => void;
  disabled?: boolean;
  autoFocusFirstField?: boolean;
  onDraftStateChange?: (next: { answeredCount: number; totalCount: number }) => void;
  onAnswerFocus?: () => void;
}

function statusCopy(
  allowPartialSubmit: boolean,
  missingRequiredCount: number,
  highlightDoneCta: boolean,
): string {
  if (highlightDoneCta) {
    return 'No more draft changes to send? Finish intake and start building.';
  }
  if (allowPartialSubmit) {
    return 'Submit any answered questions. Unanswered questions can be sent later.';
  }
  if (missingRequiredCount > 0) {
    return `Answer ${missingRequiredCount} required question${missingRequiredCount === 1 ? '' : 's'} before submitting.`;
  }
  return 'All required questions are answered.';
}

export default function QuestionCanvas({
  prompt,
  onSubmit,
  onDone,
  disabled = false,
  autoFocusFirstField = false,
  onDraftStateChange,
  onAnswerFocus,
}: QuestionCanvasProps) {
  const primePrompt = useSocraticDraftStore((state) => state.primePrompt);
  const clearItems = useSocraticDraftStore((state) => state.clearItems);
  const progress = useSocraticDraftStore(useShallow((state) => selectPromptProgress(state, prompt)));

  useEffect(() => {
    primePrompt(prompt, getSocraticDocumentPromptDrafts(prompt));
  }, [primePrompt, prompt]);

  useEffect(() => {
    onDraftStateChange?.({
      answeredCount: progress.answeredCount,
      totalCount: progress.totalCount,
    });
  }, [onDraftStateChange, progress.answeredCount, progress.totalCount]);

  const canSubmit = !disabled
    && progress.hasAnyAnswer
    && (prompt.allow_partial_submit || progress.missingRequiredCount === 0);
  const highlightDoneCta = Boolean(onDone)
    && prompt.kind === 'draft_review'
    && prompt.allow_partial_submit
    && (prompt.required_item_ids?.length ?? 0) === 0
    && !progress.hasAnyAnswer;
  const doneButtonClassName = [
    'socratic-question-canvas__button',
    'socratic-question-canvas__button--done',
    highlightDoneCta ? 'is-primary' : '',
  ].filter(Boolean).join(' ');
  const submitButtonClassName = [
    'socratic-question-canvas__button',
    'socratic-question-canvas__button--submit',
    canSubmit ? 'is-primary' : '',
  ].filter(Boolean).join(' ');

  const submit = (): void => {
    if (!canSubmit) return;

    const answers = collectPromptAnswers(prompt, useSocraticDraftStore.getState());
    if (answers.length === 0) return;

    onSubmit(prompt.prompt_id, answers);
    clearItems(prompt.prompt_id, answers.map((answer) => answer.item_id));
  };

  const footerStatus = useMemo(
    () => statusCopy(prompt.allow_partial_submit, progress.missingRequiredCount, highlightDoneCta),
    [highlightDoneCta, progress.missingRequiredCount, prompt.allow_partial_submit],
  );

  return (
    <div
      className="socratic-question-canvas"
      data-kind={prompt.kind}
      onFocusCapture={(event) => {
        const target = event.target;
        if (!(target instanceof HTMLElement)) return;
        if (target.tagName === 'TEXTAREA' || target.getAttribute('role') === 'radio') {
          onAnswerFocus?.();
        }
      }}
    >
      <header className="socratic-question-canvas__header">
        <div className="socratic-question-canvas__summary">
          <span className="socratic-question-canvas__title">{prompt.title}</span>
          <span className="socratic-question-canvas__count">
            [ {progress.answeredCount}/{progress.totalCount} ]
          </span>
        </div>

        {prompt.instructions && (
          <p className="socratic-question-canvas__instructions">{prompt.instructions}</p>
        )}
      </header>

      <div className="socratic-question-canvas__stack">
        {prompt.items.map((item, index) => (
          <QuestionBlock
            key={item.item_id}
            promptId={prompt.prompt_id}
            item={item}
            index={index}
            disabled={disabled}
            autoFocusTextarea={autoFocusFirstField && index === 0}
          />
        ))}
      </div>

      <footer className="socratic-question-canvas__footer">
        <span className="socratic-question-canvas__status">{footerStatus}</span>

        <div className="socratic-question-canvas__actions">
          {onDone && (
            <button
              type="button"
              onClick={onDone}
              disabled={disabled}
              aria-label="Done with interview"
              className={doneButtonClassName}
            >
              Done - start building
            </button>
          )}

          <button
            type="button"
            onClick={submit}
            disabled={!canSubmit}
            className={submitButtonClassName}
            aria-label="Submit prompt answers"
          >
            Submit answered items
          </button>
        </div>
      </footer>
    </div>
  );
}

export { collectPromptAnswers as collectAnswers };
