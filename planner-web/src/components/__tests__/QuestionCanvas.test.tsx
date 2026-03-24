import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import QuestionCanvas from '../QuestionCanvas.tsx';
import {
  hydrateSocraticDocumentGraph,
  resetSocraticDocumentGraph,
  syncSocraticDocumentDraft,
} from '../../stores/socraticDocumentStore.ts';
import {
  selectQuestionDraft,
  useSocraticDraftStore,
} from '../../stores/useSocraticDraftStore.ts';
import type { PromptEnvelope, SocraticWorkspaceSnapshot } from '../../types.ts';

function makePrompt(overrides: Partial<PromptEnvelope> = {}): PromptEnvelope {
  return {
    prompt_id: 'prompt-1',
    kind: 'question_batch',
    title: 'Clarify requirements',
    instructions: 'Answer any cards you can.',
    category_path: [],
    items: [
      {
        item_id: 'item-1',
        kind: 'discovery',
        target_dimension: 'platform',
        section_ref: null,
        text: 'Which platform should we prioritize first?',
        options: [
          {
            option_id: 'opt-web',
            label: 'Web app',
            semantic_value: 'web',
          },
          {
            option_id: 'opt-mobile',
            label: 'Mobile app',
            semantic_value: 'mobile',
          },
        ],
        response_mode: 'single_select_with_custom_text',
        required: false,
        priority: 100,
        dependency_item_ids: [],
      },
      {
        item_id: 'item-2',
        kind: 'verification',
        target_dimension: 'auth',
        section_ref: null,
        text: 'Do we need SSO support in v1?',
        options: [
          {
            option_id: 'opt-yes',
            label: 'Yes, include SSO',
            semantic_value: 'yes',
          },
          {
            option_id: 'opt-no',
            label: 'No, defer SSO',
            semantic_value: 'no',
          },
        ],
        response_mode: 'single_select_with_custom_text',
        required: false,
        priority: 90,
        dependency_item_ids: [],
      },
    ],
    draft_snapshot: null,
    required_item_ids: [],
    allow_partial_submit: true,
    ui_hints: {
      preferred_layout: 'cards',
      show_draft_sidebar: false,
    },
    based_on_turn: 4,
    created_at: '2026-03-08T00:00:00Z',
    ...overrides,
  };
}

function makeWorkspace(): SocraticWorkspaceSnapshot {
  return {
    focused_category_id: 'category-platform',
    branch_notice: null,
    category_snapshot: {
      revision: 'category-1',
      root_category_ids: ['category-platform'],
      nodes: [
        {
          category_id: 'category-platform',
          parent_category_id: null,
          title: 'Platform',
          summary: 'Clarify the product surface.',
          status: 'active',
          depth: 0,
          mapped_dimensions: ['Platform'],
          has_children: false,
          has_prompt_ready: true,
          item_count_hint: 2,
        },
      ],
      active_category_path: [
        { category_id: 'category-platform', title: 'Platform' },
      ],
      newly_available_category_ids: [],
      build_ready: false,
      build_readiness_message: 'Build is blocked until the interview converges.',
    },
    groups: [
      {
        category_id: 'category-platform',
        title: 'Platform',
        summary: 'Clarify the product surface.',
        status: 'active',
        question_count: 2,
        is_focused: true,
        is_new: false,
        preview_items: [],
      },
    ],
  };
}

function DraftProbe({
  promptId,
  itemId,
  label,
  renderCounts,
}: {
  promptId: string;
  itemId: string;
  label: string;
  renderCounts: Record<string, number>;
}) {
  renderCounts[label] = (renderCounts[label] ?? 0) + 1;
  const draft = useSocraticDraftStore((state) => selectQuestionDraft(state, promptId, itemId));
  return <div data-testid={`probe-${label}`}>{draft.customText}</div>;
}

describe('QuestionCanvas', () => {
  beforeEach(() => {
    useSocraticDraftStore.setState((state) => ({ ...state, prompts: {} }));
    resetSocraticDocumentGraph();
  });

  it('renders prompt questions from a PromptEnvelope', () => {
    render(<QuestionCanvas prompt={makePrompt()} onSubmit={vi.fn()} />);

    expect(screen.getByText('Clarify requirements')).toBeInTheDocument();
    expect(screen.getByText('Which platform should we prioritize first?')).toBeInTheDocument();
    expect(screen.getByText('Do we need SSO support in v1?')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /submit prompt answers/i })).toBeInTheDocument();
  });

  it('supports single-select plus custom text on the same item', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(<QuestionCanvas prompt={makePrompt()} onSubmit={onSubmit} />);

    await user.click(screen.getByRole('radio', { name: /web app/i }));
    await user.type(screen.getByLabelText('Custom text for item-1'), 'Need responsive breakpoints.');
    await user.click(screen.getByRole('button', { name: /submit prompt answers/i }));

    expect(onSubmit).toHaveBeenCalledWith('prompt-1', [
      {
        item_id: 'item-1',
        selected_option_id: 'opt-web',
        custom_text: 'Need responsive breakpoints.',
      },
    ]);
  });

  it('submits only answered items for partial-submit prompts', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(<QuestionCanvas prompt={makePrompt()} onSubmit={onSubmit} />);

    await user.type(screen.getByLabelText('Custom text for item-2'), 'SSO can wait for enterprise tier.');
    await user.click(screen.getByRole('button', { name: /submit prompt answers/i }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit).toHaveBeenCalledWith('prompt-1', [
      {
        item_id: 'item-2',
        custom_text: 'SSO can wait for enterprise tier.',
      },
    ]);
  });

  it('promotes done as the primary action for untouched draft-review prompts', () => {
    render(
      <QuestionCanvas
        prompt={makePrompt({
          kind: 'draft_review',
          title: 'Review and refine draft',
        })}
        onSubmit={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText(/no more draft changes to send\? finish intake and start building\./i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /done with interview/i })).toHaveTextContent('Done - start building');
    expect(screen.getByRole('button', { name: /done with interview/i })).toHaveClass('is-primary');
  });

  it('keeps submit as the active action once a draft-review answer is selected', async () => {
    const user = userEvent.setup();

    render(
      <QuestionCanvas
        prompt={makePrompt({
          kind: 'draft_review',
          title: 'Review and refine draft',
        })}
        onSubmit={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    await user.click(screen.getByRole('radio', { name: /web app/i }));

    expect(screen.getByText(/submit any answered questions\. unanswered questions can be sent later\./i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /submit prompt answers/i })).toBeEnabled();
    expect(screen.getByRole('button', { name: /done with interview/i })).not.toHaveClass('is-primary');
  });

  it('focuses the first answer field when explicit terminal autofocus is requested', () => {
    render(
      <QuestionCanvas
        prompt={makePrompt()}
        onSubmit={vi.fn()}
        autoFocusFirstField
      />,
    );

    expect(screen.getByLabelText('Custom text for item-1')).toHaveFocus();
  });

  it('reports live draft progress while answers change', async () => {
    const user = userEvent.setup();
    const onDraftStateChange = vi.fn();

    render(
      <QuestionCanvas
        prompt={makePrompt()}
        onSubmit={vi.fn()}
        onDraftStateChange={onDraftStateChange}
      />,
    );

    await user.type(screen.getByLabelText('Custom text for item-1'), 'Web first.');

    expect(onDraftStateChange).toHaveBeenLastCalledWith({
      answeredCount: 1,
      totalCount: 2,
    });
  });

  it('rehydrates previously known document drafts when a prompt returns', () => {
    const prompt = makePrompt();
    hydrateSocraticDocumentGraph({
      workspace: makeWorkspace(),
      currentPrompt: prompt,
    });
    syncSocraticDocumentDraft('item-1', {
      selectedOptionId: 'opt-web',
      customText: 'Restore the last working answer.',
    });

    render(<QuestionCanvas prompt={prompt} onSubmit={vi.fn()} />);

    expect(screen.getByLabelText('Custom text for item-1')).toHaveValue('Restore the last working answer.');
    expect(screen.getByRole('radio', { name: /web app/i })).toBeChecked();
  });

  it('renders a dedicated scrolling item region and footer action row', () => {
    const { container } = render(<QuestionCanvas prompt={makePrompt()} onSubmit={vi.fn()} />);

    expect(container.querySelector('.socratic-question-canvas')).toBeInTheDocument();
    expect(container.querySelector('.socratic-question-canvas__stack')).toBeInTheDocument();
    expect(container.querySelector('.socratic-question-canvas__footer')).toBeInTheDocument();
  });

  it('keeps unrelated draft selectors stable when one answer changes', async () => {
    const user = userEvent.setup();
    const renderCounts: Record<string, number> = {};

    render(
      <>
        <QuestionCanvas prompt={makePrompt()} onSubmit={vi.fn()} />
        <DraftProbe promptId="prompt-1" itemId="item-1" label="item-1" renderCounts={renderCounts} />
        <DraftProbe promptId="prompt-1" itemId="item-2" label="item-2" renderCounts={renderCounts} />
      </>,
    );

    expect(renderCounts['item-1']).toBe(1);
    expect(renderCounts['item-2']).toBe(1);

    await user.type(screen.getByLabelText('Custom text for item-1'), 'x');

    expect(screen.getByTestId('probe-item-1')).toHaveTextContent('x');
    expect(renderCounts['item-1']).toBeGreaterThan(1);
    expect(renderCounts['item-2']).toBe(1);
  });
});
