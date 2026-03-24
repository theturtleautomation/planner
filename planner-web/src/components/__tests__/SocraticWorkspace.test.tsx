import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SocraticWorkspace from '../SocraticWorkspace.tsx';
import { resetSocraticDocumentGraph } from '../../stores/socraticDocumentStore.ts';
import { useSocraticDraftStore } from '../../stores/useSocraticDraftStore.ts';
import type { PromptEnvelope, SocraticWorkspaceSnapshot } from '../../types.ts';

function makeWorkspace(overrides: Partial<SocraticWorkspaceSnapshot> = {}): SocraticWorkspaceSnapshot {
  return {
    focused_category_id: 'root-security-auth',
    branch_notice: null,
    category_snapshot: {
      revision: 'category-1',
      root_category_ids: ['root-discovery'],
      nodes: [
        {
          category_id: 'root-discovery',
          parent_category_id: null,
          title: 'Explore missing areas',
          summary: 'Start with the broad discovery branch.',
          status: 'ready',
          depth: 0,
          mapped_dimensions: ['Discovery'],
          has_children: true,
          has_prompt_ready: false,
          item_count_hint: 2,
        },
        {
          category_id: 'root-platform',
          parent_category_id: 'root-discovery',
          title: 'Platform',
          summary: 'Clarify delivery surface.',
          status: 'ready',
          depth: 1,
          mapped_dimensions: ['Platform'],
          has_children: true,
          has_prompt_ready: false,
          item_count_hint: 2,
        },
        {
          category_id: 'root-security-auth',
          parent_category_id: 'root-platform',
          title: 'Authentication model',
          summary: 'Clarify sign-in and authorization.',
          status: 'active',
          depth: 2,
          mapped_dimensions: ['Security'],
          has_children: false,
          has_prompt_ready: true,
          item_count_hint: 1,
        },
      ],
      active_category_path: [
        { category_id: 'root-discovery', title: 'Explore missing areas' },
        { category_id: 'root-platform', title: 'Platform' },
        { category_id: 'root-security-auth', title: 'Authentication model' },
      ],
      newly_available_category_ids: [],
      build_ready: false,
      build_readiness_message: 'Build is blocked until the remaining interview work is complete.',
    },
    groups: [
      {
        category_id: 'root-security-auth',
        title: 'Authentication model',
        summary: 'Clarify sign-in and authorization.',
        status: 'active',
        question_count: 1,
        is_focused: true,
        is_new: false,
        preview_items: [],
      },
    ],
    ...overrides,
  };
}

function makePrompt(overrides: Partial<PromptEnvelope> = {}): PromptEnvelope {
  return {
    prompt_id: 'prompt-1',
    kind: 'question_batch',
    title: 'Clarify authentication',
    instructions: 'Answer the focused security question.',
    origin_category_id: 'root-security-auth',
    category_path: [
      { category_id: 'root-discovery', title: 'Explore missing areas' },
      { category_id: 'root-platform', title: 'Platform' },
      { category_id: 'root-security-auth', title: 'Authentication model' },
    ],
    items: [
      {
        item_id: 'item-1',
        kind: 'discovery',
        target_dimension: 'Security',
        section_ref: null,
        text: 'How should authentication work?',
        options: [],
        response_mode: 'single_select_with_custom_text',
        required: false,
        priority: 100,
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
    created_at: '2026-03-22T00:00:00Z',
    ...overrides,
  };
}

describe('SocraticWorkspace', () => {
  beforeEach(() => {
    useSocraticDraftStore.setState((state) => ({ ...state, prompts: {} }));
    resetSocraticDocumentGraph();
  });

  it('renders a permanent thread index and active consultant desk', () => {
    render(
      <SocraticWorkspace
        workspace={makeWorkspace()}
        currentPrompt={makePrompt()}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByLabelText('Thread index')).toBeInTheDocument();
    expect(screen.getByLabelText('Consultant desk')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Authentication model' })).toBeInTheDocument();
    expect(screen.getAllByText('Explore missing areas').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Platform').length).toBeGreaterThan(0);
    expect(screen.queryByText('Thread')).not.toBeInTheDocument();
    expect(screen.queryByText('Live question')).not.toBeInTheDocument();
  });

  it('updates sidebar telemetry live while the active answer draft changes', async () => {
    const user = userEvent.setup();

    render(
      <SocraticWorkspace
        workspace={makeWorkspace()}
        currentPrompt={makePrompt()}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByRole('button', { name: 'Authentication model [ 0/1 ]' })).toBeInTheDocument();

    await user.type(screen.getByLabelText('Custom text for item-1'), 'Use email magic links.');

    expect(screen.getByRole('button', { name: 'Authentication model [ 1/1 ]' })).toBeInTheDocument();
  });

  it('uses arrow traversal to preview another thread in the document without changing server focus', async () => {
    const user = userEvent.setup();
    const onFocusCategory = vi.fn();

    render(
      <SocraticWorkspace
        workspace={makeWorkspace()}
        currentPrompt={makePrompt()}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={onFocusCategory}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    const activeRow = screen.getByRole('button', { name: 'Authentication model [ 0/1 ]' });
    activeRow.focus();
    await user.keyboard('{ArrowUp}');

    expect(onFocusCategory).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: /Platform \[/i })).toHaveAttribute('aria-current', 'true');
  });

  it('pressing Enter on an index row focuses the first answerable item when it exists', async () => {
    const user = userEvent.setup();

    render(
      <SocraticWorkspace
        workspace={makeWorkspace()}
        currentPrompt={makePrompt()}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    const activeRow = screen.getByRole('button', { name: 'Authentication model [ 0/1 ]' });
    activeRow.focus();
    await user.keyboard('{Enter}');

    expect(screen.getByLabelText('Custom text for item-1')).toHaveFocus();
  });

  it('keeps the actively edited section highlighted even if visible focus shifts elsewhere', async () => {
    const user = userEvent.setup();
    const { rerender } = render(
      <SocraticWorkspace
        workspace={makeWorkspace()}
        currentPrompt={makePrompt()}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    await user.click(screen.getByLabelText('Custom text for item-1'));

    rerender(
      <SocraticWorkspace
        workspace={makeWorkspace({
          focused_category_id: 'root-platform',
        })}
        currentPrompt={makePrompt()}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByRole('button', { name: 'Authentication model [ 0/1 ]' })).toHaveAttribute('aria-current', 'true');
  });

  it('renders localized preparing feedback for a pending focused category', () => {
    render(
      <SocraticWorkspace
        workspace={makeWorkspace({
          category_snapshot: {
            ...makeWorkspace().category_snapshot,
            active_category_path: [],
          },
        })}
        currentPrompt={null}
        pendingCategoryId="root-security-auth"
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText('Preparing question')).toBeInTheDocument();
    expect(screen.getByText(/planner is generating the next question for this section now/i)).toBeInTheDocument();
  });

  it('keeps known preview content visible instead of showing preparing while server focus catches up', () => {
    render(
      <SocraticWorkspace
        workspace={makeWorkspace({
          groups: [
            {
              category_id: 'root-security-auth',
              title: 'Authentication model',
              summary: 'Clarify sign-in and authorization.',
              status: 'pending',
              question_count: 1,
              is_focused: true,
              is_new: false,
              preview_items: [
                {
                  item_id: 'preview-1',
                  kind: 'discovery',
                  text: 'How should authentication work?',
                },
              ],
            },
          ],
        })}
        currentPrompt={null}
        pendingCategoryId="root-security-auth"
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText('How should authentication work?')).toBeInTheDocument();
    expect(screen.queryByText('Preparing question')).not.toBeInTheDocument();
    expect(screen.queryByText('Preparing')).not.toBeInTheDocument();
  });

  it('offers a return-to-live action when the live question moved elsewhere', () => {
    render(
      <SocraticWorkspace
        workspace={makeWorkspace({
          branch_notice: 'Planner moved the live question to another thread.',
          groups: [
            {
              category_id: 'root-security-auth',
              title: 'Authentication model',
              summary: 'Clarify sign-in and authorization.',
              status: 'ready',
              question_count: 1,
              is_focused: true,
              is_new: false,
              preview_items: [
                {
                  item_id: 'preview-1',
                  kind: 'discovery',
                  text: 'How should authentication work?',
                },
              ],
            },
          ],
        })}
        currentPrompt={null}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByRole('button', { name: 'Go to live question' })).toBeInTheDocument();
    expect(screen.getByText('How should authentication work?')).toBeInTheDocument();
  });

  it('retains previously loaded questions when the live prompt is no longer mounted', () => {
    const { rerender } = render(
      <SocraticWorkspace
        workspace={makeWorkspace()}
        currentPrompt={makePrompt()}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    rerender(
      <SocraticWorkspace
        workspace={makeWorkspace({
          groups: [
            {
              category_id: 'root-security-auth',
              title: 'Authentication model',
              summary: 'Clarify sign-in and authorization.',
              status: 'ready',
              question_count: 1,
              is_focused: true,
              is_new: false,
              preview_items: [],
            },
          ],
        })}
        currentPrompt={null}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText('How should authentication work?')).toBeInTheDocument();
  });

  it('renders the build-ready hero when no groups remain and the workspace is ready', () => {
    render(
      <SocraticWorkspace
        workspace={makeWorkspace({
          focused_category_id: null,
          category_snapshot: {
            ...makeWorkspace().category_snapshot,
            active_category_path: [],
            build_ready: true,
            build_readiness_message: 'The interview has converged.',
          },
          groups: [],
        })}
        currentPrompt={null}
        pendingCategoryId={null}
        workspaceNotice={null}
        onFocusCategory={vi.fn()}
        onShowAll={vi.fn()}
        onSubmitAnswers={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText('Build ready')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /commit plan/i }).length).toBeGreaterThan(0);
  });
});
