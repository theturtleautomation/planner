import { beforeEach, describe, expect, it } from 'vitest';
import type { PromptEnvelope, SocraticWorkspaceSnapshot } from '../../types.ts';
import {
  buildSocraticDocumentCategoryViews,
  getSocraticDocumentGraphState,
  hydrateSocraticDocumentGraph,
  resetSocraticDocumentGraph,
  syncSocraticDocumentDraft,
} from '../socraticDocumentStore.ts';

function makeWorkspace(): SocraticWorkspaceSnapshot {
  return {
    focused_category_id: 'category-auth',
    branch_notice: null,
    category_snapshot: {
      revision: 'category-1',
      root_category_ids: ['category-platform'],
      nodes: [
        {
          category_id: 'category-platform',
          parent_category_id: null,
          title: 'Platform',
          summary: 'Clarify the primary platform.',
          status: 'ready',
          depth: 0,
          mapped_dimensions: ['Platform'],
          has_children: true,
          has_prompt_ready: false,
          item_count_hint: 1,
        },
        {
          category_id: 'category-auth',
          parent_category_id: 'category-platform',
          title: 'Authentication model',
          summary: 'Clarify how people sign in.',
          status: 'active',
          depth: 1,
          mapped_dimensions: ['Security'],
          has_children: false,
          has_prompt_ready: true,
          item_count_hint: 2,
        },
      ],
      active_category_path: [
        { category_id: 'category-platform', title: 'Platform' },
        { category_id: 'category-auth', title: 'Authentication model' },
      ],
      newly_available_category_ids: ['category-auth'],
      build_ready: false,
      build_readiness_message: 'Build is blocked until remaining answers land.',
    },
    groups: [
      {
        category_id: 'category-auth',
        title: 'Authentication model',
        summary: 'Clarify how people sign in.',
        status: 'active',
        question_count: 2,
        is_focused: true,
        is_new: true,
        preview_items: [],
      },
    ],
  };
}

function makePrompt(): PromptEnvelope {
  return {
    prompt_id: 'prompt-auth',
    kind: 'question_batch',
    title: 'Clarify authentication',
    instructions: 'Answer the current authentication questions.',
    origin_category_id: 'category-auth',
    category_path: [
      { category_id: 'category-platform', title: 'Platform' },
      { category_id: 'category-auth', title: 'Authentication model' },
    ],
    items: [
      {
        item_id: 'question-auth-1',
        kind: 'discovery',
        target_dimension: 'Security',
        section_ref: null,
        text: 'How should people sign in?',
        options: [],
        response_mode: 'single_select_with_custom_text',
        required: false,
        priority: 100,
        dependency_item_ids: [],
      },
      {
        item_id: 'question-auth-2',
        kind: 'verification',
        target_dimension: 'Security',
        section_ref: null,
        text: 'Do we need password resets in v1?',
        options: [],
        response_mode: 'single_select_with_custom_text',
        required: false,
        priority: 80,
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
    based_on_turn: 3,
    created_at: '2026-03-23T00:00:00Z',
  };
}

describe('socraticDocumentStore', () => {
  beforeEach(() => {
    resetSocraticDocumentGraph();
  });

  it('hydrates a normalized category and question graph from workspace and prompt payloads', () => {
    hydrateSocraticDocumentGraph({
      workspace: makeWorkspace(),
      currentPrompt: makePrompt(),
    });

    const state = getSocraticDocumentGraphState();

    expect(state.categoryOrder).toEqual(['category-platform', 'category-auth']);
    expect(state.categoriesById['category-auth']?.questionIds).toEqual([
      'question-auth-1',
      'question-auth-2',
    ]);
    expect(state.questionsById['question-auth-1']?.text).toBe('How should people sign in?');
    expect(state.categoriesById['category-auth']?.latestPromptTitle).toBe('Clarify authentication');
  });

  it('derives category telemetry from stored document drafts without a mounted active prompt', () => {
    hydrateSocraticDocumentGraph({
      workspace: makeWorkspace(),
      currentPrompt: makePrompt(),
    });
    syncSocraticDocumentDraft('question-auth-1', {
      selectedOptionId: null,
      customText: 'Use magic links first.',
    });

    const categoryViews = buildSocraticDocumentCategoryViews(getSocraticDocumentGraphState());
    const authView = categoryViews.find((category) => category.categoryId === 'category-auth');

    expect(authView?.answeredCount).toBe(1);
    expect(authView?.totalCount).toBe(2);
  });

  it('preserves unrelated drafts when a new category is inserted ahead of an existing thread', () => {
    hydrateSocraticDocumentGraph({
      workspace: makeWorkspace(),
      currentPrompt: makePrompt(),
    });
    syncSocraticDocumentDraft('question-auth-1', {
      selectedOptionId: null,
      customText: 'Use magic links first.',
    });

    hydrateSocraticDocumentGraph({
      workspace: {
        ...makeWorkspace(),
        category_snapshot: {
          ...makeWorkspace().category_snapshot,
          nodes: [
            {
              category_id: 'category-stakeholders',
              parent_category_id: null,
              title: 'Stakeholders',
              summary: 'Clarify who needs the plan.',
              status: 'ready',
              depth: 0,
              mapped_dimensions: ['Stakeholders'],
              has_children: false,
              has_prompt_ready: true,
              item_count_hint: 1,
            },
            ...makeWorkspace().category_snapshot.nodes,
          ],
          root_category_ids: ['category-stakeholders', 'category-platform'],
          newly_available_category_ids: ['category-stakeholders'],
        },
      },
      currentPrompt: {
        ...makePrompt(),
        prompt_id: 'prompt-stakeholders',
        title: 'Clarify stakeholders',
        origin_category_id: 'category-stakeholders',
        category_path: [
          { category_id: 'category-stakeholders', title: 'Stakeholders' },
        ],
        items: [
          {
            item_id: 'question-stakeholders-1',
            kind: 'discovery',
            target_dimension: 'Stakeholders',
            section_ref: null,
            text: 'Who is the primary stakeholder?',
            options: [],
            response_mode: 'single_select_with_custom_text',
            required: false,
            priority: 100,
            dependency_item_ids: [],
          },
        ],
      },
    });

    const state = getSocraticDocumentGraphState();

    expect(state.categoryOrder.slice(0, 3)).toEqual([
      'category-stakeholders',
      'category-platform',
      'category-auth',
    ]);
    expect(state.draftsByQuestionId['question-auth-1']?.customText).toBe('Use magic links first.');
    expect(state.categoriesById['category-auth']?.questionIds).toContain('question-auth-1');
  });
});
