import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import PromptBatchPanel from '../PromptBatchPanel.tsx';
import type { PromptEnvelope } from '../../types.ts';

function makePrompt(overrides: Partial<PromptEnvelope> = {}): PromptEnvelope {
  return {
    prompt_id: 'prompt-1',
    kind: 'question_batch',
    title: 'Clarify requirements',
    instructions: 'Answer any cards you can.',
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

describe('PromptBatchPanel', () => {
  it('renders prompt cards from a PromptEnvelope', () => {
    render(<PromptBatchPanel prompt={makePrompt()} onSubmit={vi.fn()} />);

    expect(screen.getByText('Clarify requirements')).toBeInTheDocument();
    expect(screen.getByText('Which platform should we prioritize first?')).toBeInTheDocument();
    expect(screen.getByText('Do we need SSO support in v1?')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /submit prompt answers/i })).toBeInTheDocument();
  });

  it('supports single-select plus custom text on the same item', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();

    render(<PromptBatchPanel prompt={makePrompt()} onSubmit={onSubmit} />);

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

    render(<PromptBatchPanel prompt={makePrompt()} onSubmit={onSubmit} />);

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
});
