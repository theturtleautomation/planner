import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SocraticWorkspace from '../SocraticWorkspace.tsx';
import {
  hydrateSocraticDocumentGraph,
  resetSocraticDocumentGraph,
} from '../../stores/socraticDocumentStore.ts';
import { useSocraticDraftStore } from '../../stores/useSocraticDraftStore.ts';
import type { PromptEnvelope, SocraticWorkspaceSnapshot } from '../../types.ts';

function makePrompt(categoryId: string, title: string): PromptEnvelope {
  return {
    prompt_id: `prompt-${categoryId}`,
    kind: 'question_batch',
    title,
    instructions: `Clarify ${title}.`,
    origin_category_id: categoryId,
    category_path: [
      { category_id: categoryId, title },
    ],
    items: Array.from({ length: 8 }, (_, index) => ({
      item_id: `${categoryId}-question-${index + 1}`,
      kind: 'discovery' as const,
      target_dimension: title,
      section_ref: null,
      text: `${title} question ${index + 1}?`,
      options: [],
      response_mode: 'single_select_with_custom_text' as const,
      required: false,
      priority: 100 - index,
      dependency_item_ids: [],
    })),
    draft_snapshot: null,
    required_item_ids: [],
    allow_partial_submit: true,
    ui_hints: {
      preferred_layout: 'cards',
      show_draft_sidebar: false,
    },
    based_on_turn: 5,
    created_at: '2026-03-23T00:00:00Z',
  };
}

function makeWorkspace(): SocraticWorkspaceSnapshot {
  const nodes = Array.from({ length: 15 }, (_, index) => ({
    category_id: `category-${index + 1}`,
    parent_category_id: null,
    title: `Category ${index + 1}`,
    summary: `Clarify category ${index + 1}.`,
    status: index === 0 ? 'active' as const : 'ready' as const,
    depth: 0,
    mapped_dimensions: [`Dimension ${index + 1}`],
    has_children: false,
    has_prompt_ready: true,
    item_count_hint: 8,
  }));

  return {
    focused_category_id: 'category-1',
    branch_notice: null,
    category_snapshot: {
      revision: 'perf-revision-1',
      root_category_ids: nodes.map((node) => node.category_id),
      nodes,
      active_category_path: [
        { category_id: 'category-1', title: 'Category 1' },
      ],
      newly_available_category_ids: [],
      build_ready: false,
      build_readiness_message: 'Build is blocked until the interview converges.',
    },
    groups: [
      {
        category_id: 'category-1',
        title: 'Category 1',
        summary: 'Clarify category 1.',
        status: 'active',
        question_count: 8,
        is_focused: true,
        is_new: false,
        preview_items: [],
      },
    ],
  };
}

describe('SocraticWorkspace performance guardrail', () => {
  beforeEach(() => {
    useSocraticDraftStore.setState((state) => ({ ...state, prompts: {} }));
    resetSocraticDocumentGraph();
  });

  it('keeps a seeded 15-category / 120-question desk responsive for a single keystroke', async () => {
    const user = userEvent.setup();
    const workspace = makeWorkspace();
    const activePrompt = makePrompt('category-1', 'Category 1');

    for (let index = 0; index < 15; index += 1) {
      const categoryId = `category-${index + 1}`;
      hydrateSocraticDocumentGraph({
        workspace,
        currentPrompt: makePrompt(categoryId, `Category ${index + 1}`),
      });
    }

    render(
      <SocraticWorkspace
        workspace={workspace}
        currentPrompt={activePrompt}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getAllByRole('button').filter((button) => button.getAttribute('aria-label')?.includes('Category')).length)
      .toBeGreaterThanOrEqual(15);

    const input = screen.getByLabelText('Custom text for category-1-question-1');
    const start = performance.now();
    await user.type(input, 'x');
    const elapsed = performance.now() - start;

    expect(screen.getByRole('button', { name: 'Category 1 [ 1/8 ]' })).toBeInTheDocument();
    expect(elapsed).toBeLessThan(1000);
  });
});
