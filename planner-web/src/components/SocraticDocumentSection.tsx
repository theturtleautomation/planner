import QuestionCanvas from './QuestionCanvas.tsx';
import {
  useSocraticDocumentQuestionDraft,
  useSocraticDocumentQuestions,
  type SocraticDocumentCategoryView,
} from '../stores/socraticDocumentStore.ts';
import type {
  PromptAnswer,
  PromptEnvelope,
  SocraticWorkspaceGroup,
} from '../types.ts';

interface SocraticDocumentSectionProps {
  category: SocraticDocumentCategoryView;
  currentPrompt: PromptEnvelope | null;
  pendingCategoryId: string | null;
  branchNotice: string | null;
  group: SocraticWorkspaceGroup | null;
  disabled?: boolean;
  onSubmitAnswers: (answers: PromptAnswer[]) => void;
  onDone?: () => void;
  onShowAll: () => void;
  onAnswerFocus?: (categoryId: string) => void;
  hideHeader?: boolean;
  autoFocusFirstField?: boolean;
}

function formatMappedDimensions(
  dimensions: Array<string | Record<string, unknown>>,
): string | null {
  if (dimensions.length === 0) return null;
  return dimensions
    .map((dimension) => {
      if (typeof dimension === 'string') return dimension;
      const keys = Object.keys(dimension);
      if (keys.length === 1 && typeof dimension[keys[0]] === 'string') {
        return String(dimension[keys[0]]);
      }
      return JSON.stringify(dimension);
    })
    .join(' | ');
}

function promptCategoryId(prompt: PromptEnvelope | null): string | null {
  if (!prompt) return null;
  return prompt.origin_category_id
    ?? prompt.category_path[prompt.category_path.length - 1]?.category_id
    ?? null;
}

function RetainedQuestionBlock({ questionId }: { questionId: string }) {
  const draft = useSocraticDocumentQuestionDraft(questionId);
  const answerText = draft.customText.trim();

  return (
    <div className="socratic-document-question">
      <div className="socratic-document-question__meta">
        <span className="socratic-document-question__label">Saved answer</span>
      </div>
      {draft.selectedOptionId && (
        <div className="socratic-document-question__answer">
          Selected option: {draft.selectedOptionId}
        </div>
      )}
      {answerText && (
        <div className="socratic-document-question__answer">
          {answerText}
        </div>
      )}
      {!draft.selectedOptionId && !answerText && (
        <div className="socratic-document-question__placeholder">
          No draft answer captured yet.
        </div>
      )}
    </div>
  );
}

export default function SocraticDocumentSection({
  category,
  currentPrompt,
  pendingCategoryId,
  branchNotice,
  group,
  disabled = false,
  onSubmitAnswers,
  onDone,
  onShowAll,
  onAnswerFocus,
  hideHeader = false,
  autoFocusFirstField = false,
}: SocraticDocumentSectionProps) {
  const questions = useSocraticDocumentQuestions(category.categoryId);
  const previewItems = group?.preview_items ?? [];
  const mappedDimensions = formatMappedDimensions(category.mappedDimensions);
  const activePromptCategoryId = promptCategoryId(currentPrompt);
  const isPromptSection = Boolean(currentPrompt && activePromptCategoryId === category.categoryId);
  const hasLocalContent = isPromptSection || questions.length > 0 || previewItems.length > 0;
  const isPreparing = Boolean(
    pendingCategoryId === category.categoryId
    && !isPromptSection
    && !hasLocalContent,
  );
  const sectionEyebrow = isPreparing ? 'Preparing' : null;

  return (
    <section
      className={[
        'socratic-document-section',
        isPromptSection ? 'is-live' : '',
        isPreparing ? 'is-preparing' : '',
      ].filter(Boolean).join(' ')}
      data-category-id={category.categoryId}
      aria-label={category.title}
    >
      {!hideHeader && (
        <header className="socratic-document-section__header">
          <div className="socratic-document-section__title-block">
            {sectionEyebrow && (
              <span className="socratic-document-section__eyebrow">{sectionEyebrow}</span>
            )}
            <h3 className="socratic-document-section__title">{category.title}</h3>
          </div>
          <div className="socratic-document-section__meta">
            {isPromptSection && (
              <span className="socratic-document-section__meta-line is-state">
                Current question
              </span>
            )}
            {category.summary && (
              <span className="socratic-document-section__meta-line">
                Planner context: {category.summary}
              </span>
            )}
            {mappedDimensions && (
              <span className="socratic-document-section__meta-line">
                Mapped dimensions: {mappedDimensions}
              </span>
            )}
            <span className="socratic-document-section__meta-line">
              {category.answeredCount}/{category.totalCount} answered
            </span>
          </div>
        </header>
      )}

      <div className="socratic-document-section__body">
        {isPromptSection && currentPrompt ? (
          <QuestionCanvas
            prompt={currentPrompt}
            onSubmit={(_promptId, answers) => onSubmitAnswers(answers)}
            disabled={disabled}
            onDone={onDone}
            autoFocusFirstField={autoFocusFirstField}
            onAnswerFocus={() => onAnswerFocus?.(category.categoryId)}
          />
        ) : isPreparing ? (
          <div className="socratic-document-panel is-preparing" aria-live="polite">
            <span className="socratic-terminal-kicker">Preparing question</span>
            <p className="socratic-terminal-support" style={{ margin: 0 }}>
              Planner is generating the next question for this section now.
            </p>
          </div>
        ) : questions.length > 0 ? (
          <div className="socratic-document-question-list">
            {questions.map((question, index) => (
              <article key={question.questionId} className="socratic-question-block is-retained">
                <header className="socratic-question-block__header">
                  <div className="socratic-question-block__meta">
                    <span className="socratic-question-block__index">Q{index + 1}</span>
                    <span className="socratic-question-block__kind">Retained</span>
                  </div>
                  <p className="socratic-question-block__text">{question.text}</p>
                </header>
                <RetainedQuestionBlock questionId={question.questionId} />
              </article>
            ))}
          </div>
        ) : previewItems.length > 0 ? (
          <div className="socratic-preview-list">
            {previewItems.map((item, index) => (
              <div key={item.item_id} className="socratic-preview-item">
                <span className="socratic-preview-kind">Q{index + 1}</span>
                <span>{item.text}</span>
              </div>
            ))}
          </div>
        ) : (
          <div className="socratic-document-panel">
            <p className="socratic-terminal-support" style={{ margin: 0 }}>
              Awaiting questions...
            </p>
          </div>
        )}

        {branchNotice && !isPromptSection && (
          <div className="socratic-document-section__actions">
            <p className="socratic-terminal-support" style={{ margin: 0 }}>
              The live question moved to another section. You can keep reviewing this section or jump back to the current live question.
            </p>
            <button
              type="button"
              onClick={onShowAll}
              disabled={disabled}
              className="socratic-action-button"
            >
              Go to live question
            </button>
          </div>
        )}
      </div>
    </section>
  );
}
