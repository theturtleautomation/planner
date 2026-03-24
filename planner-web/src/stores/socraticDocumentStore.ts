import { atom, getDefaultStore, useAtomValue } from 'jotai';
import { useEffect, useMemo } from 'react';
import type {
  PromptEnvelope,
  PromptItem,
  PromptOption,
  PromptResponseMode,
  SocraticCategoryNode,
  SocraticCategoryPathEntry,
  SocraticCategoryStatus,
  SocraticWorkspaceSnapshot,
} from '../types.ts';
import type { QuestionDraft } from './useSocraticDraftStore.ts';
import { EMPTY_QUESTION_DRAFT, isQuestionDraftAnswered } from './useSocraticDraftStore.ts';

export interface SocraticDocumentQuestion {
  questionId: string;
  categoryId: string;
  promptId: string | null;
  text: string;
  kind: PromptItem['kind'];
  targetDimension?: PromptItem['target_dimension'];
  sectionRef?: string | null;
  options: PromptOption[];
  responseMode: PromptResponseMode;
  required: boolean;
  priority: number;
  dependencyItemIds: string[];
  promptTitle: string | null;
  promptInstructions: string | null;
}

export interface SocraticDocumentCategory {
  categoryId: string;
  parentCategoryId?: string | null;
  title: string;
  summary: string;
  status: SocraticCategoryStatus;
  depth: number;
  mappedDimensions: Array<string | Record<string, unknown>>;
  hasChildren: boolean;
  hasPromptReady: boolean;
  itemCountHint: number;
  isNewlyAvailable: boolean;
  questionIds: string[];
  latestPromptId: string | null;
  latestPromptTitle: string | null;
  latestPromptInstructions: string | null;
}

export interface SocraticDocumentCategoryView extends SocraticDocumentCategory {
  answeredCount: number;
  totalCount: number;
}

export interface SocraticDocumentGraphState {
  revision: string | null;
  categoryOrder: string[];
  categoriesById: Record<string, SocraticDocumentCategory>;
  questionsById: Record<string, SocraticDocumentQuestion>;
  draftsByQuestionId: Record<string, QuestionDraft>;
  focusedCategoryId: string | null;
  activeCategoryId: string | null;
  currentPromptId: string | null;
  activeCategoryPath: SocraticCategoryPathEntry[];
  buildReady: boolean;
  buildReadinessMessage: string;
}

export interface SocraticDocumentHydrationPayload {
  workspace: SocraticWorkspaceSnapshot;
  currentPrompt: PromptEnvelope | null;
}

declare global {
  interface Window {
    __plannerSocraticDocumentTest?: {
      getState: typeof getSocraticDocumentGraphState;
      hydrate: typeof hydrateSocraticDocumentGraph;
      reset: typeof resetSocraticDocumentGraph;
    };
  }
}

export const EMPTY_SOCRATIC_DOCUMENT_GRAPH_STATE: SocraticDocumentGraphState = {
  revision: null,
  categoryOrder: [],
  categoriesById: {},
  questionsById: {},
  draftsByQuestionId: {},
  focusedCategoryId: null,
  activeCategoryId: null,
  currentPromptId: null,
  activeCategoryPath: [],
  buildReady: false,
  buildReadinessMessage: '',
};

function promptCategoryId(prompt: PromptEnvelope | null): string | null {
  if (!prompt) return null;
  return prompt.origin_category_id
    ?? prompt.category_path[prompt.category_path.length - 1]?.category_id
    ?? null;
}

function activePathCategoryId(workspace: SocraticWorkspaceSnapshot): string | null {
  return workspace.category_snapshot.active_category_path[
    workspace.category_snapshot.active_category_path.length - 1
  ]?.category_id ?? null;
}

function fallbackCategoryFromPrompt(prompt: PromptEnvelope): SocraticDocumentCategory {
  const categoryId = promptCategoryId(prompt) ?? prompt.prompt_id;
  const title = prompt.category_path[prompt.category_path.length - 1]?.title ?? prompt.title;

  return {
    categoryId,
    parentCategoryId: prompt.category_path.length > 1
      ? prompt.category_path[prompt.category_path.length - 2]?.category_id ?? null
      : null,
    title,
    summary: prompt.instructions?.trim() ?? 'Current questions are ready.',
    status: 'active',
    depth: Math.max(prompt.category_path.length - 1, 0),
    mappedDimensions: [],
    hasChildren: false,
    hasPromptReady: true,
    itemCountHint: Math.max(prompt.items.length, 1),
    isNewlyAvailable: false,
    questionIds: [],
    latestPromptId: prompt.prompt_id,
    latestPromptTitle: prompt.title,
    latestPromptInstructions: prompt.instructions ?? null,
  };
}

function dedupeOrder(ids: string[]): string[] {
  const seen = new Set<string>();
  return ids.filter((id) => {
    if (seen.has(id)) return false;
    seen.add(id);
    return true;
  });
}

function upsertCategoryFromNode(
  previous: SocraticDocumentCategory | undefined,
  node: SocraticCategoryNode,
  isNewlyAvailable: boolean,
): SocraticDocumentCategory {
  return {
    categoryId: node.category_id,
    parentCategoryId: node.parent_category_id ?? null,
    title: node.title,
    summary: node.summary,
    status: node.status,
    depth: node.depth,
    mappedDimensions: node.mapped_dimensions,
    hasChildren: node.has_children,
    hasPromptReady: node.has_prompt_ready,
    itemCountHint: node.item_count_hint,
    isNewlyAvailable,
    questionIds: previous?.questionIds ?? [],
    latestPromptId: previous?.latestPromptId ?? null,
    latestPromptTitle: previous?.latestPromptTitle ?? null,
    latestPromptInstructions: previous?.latestPromptInstructions ?? null,
  };
}

function mergeQuestionIds(previousIds: string[], nextIds: string[]): string[] {
  return dedupeOrder([...previousIds, ...nextIds]);
}

export function buildSocraticDocumentCategoryViews(
  state: SocraticDocumentGraphState,
): SocraticDocumentCategoryView[] {
  return state.categoryOrder.flatMap((categoryId) => {
    const category = state.categoriesById[categoryId];
    if (!category) return [];

    const answeredCount = category.questionIds.reduce((count, questionId) => (
      count + (isQuestionDraftAnswered(state.draftsByQuestionId[questionId]) ? 1 : 0)
    ), 0);
    const totalCount = category.questionIds.length > 0
      ? category.questionIds.length
      : Math.max(category.itemCountHint, category.hasPromptReady ? 1 : 0);

    return [{
      ...category,
      answeredCount,
      totalCount,
    }];
  });
}

function mergePromptIntoGraph(
  next: SocraticDocumentGraphState,
  prompt: PromptEnvelope,
): SocraticDocumentGraphState {
  const categoryId = promptCategoryId(prompt) ?? prompt.prompt_id;
  const previousCategory = next.categoriesById[categoryId] ?? fallbackCategoryFromPrompt(prompt);
  const promptQuestionIds: string[] = [];

  next.categoriesById[categoryId] = {
    ...previousCategory,
    latestPromptId: prompt.prompt_id,
    latestPromptTitle: prompt.title,
    latestPromptInstructions: prompt.instructions ?? null,
    hasPromptReady: true,
    itemCountHint: Math.max(previousCategory.itemCountHint, prompt.items.length, 1),
  };

  for (const item of prompt.items) {
    const previousQuestion = next.questionsById[item.item_id];
    if (previousQuestion && previousQuestion.categoryId !== categoryId) {
      const previousOwner = next.categoriesById[previousQuestion.categoryId];
      if (previousOwner) {
        next.categoriesById[previousQuestion.categoryId] = {
          ...previousOwner,
          questionIds: previousOwner.questionIds.filter((questionId) => questionId !== item.item_id),
        };
      }
    }

    next.questionsById[item.item_id] = {
      questionId: item.item_id,
      categoryId,
      promptId: prompt.prompt_id,
      text: item.text,
      kind: item.kind,
      targetDimension: item.target_dimension,
      sectionRef: item.section_ref ?? null,
      options: item.options,
      responseMode: item.response_mode,
      required: item.required,
      priority: item.priority,
      dependencyItemIds: item.dependency_item_ids,
      promptTitle: prompt.title,
      promptInstructions: prompt.instructions ?? null,
    };
    promptQuestionIds.push(item.item_id);
  }

  next.categoriesById[categoryId] = {
    ...next.categoriesById[categoryId],
    questionIds: mergeQuestionIds(next.categoriesById[categoryId].questionIds, promptQuestionIds),
  };

  next.categoryOrder = dedupeOrder([...next.categoryOrder, categoryId]);
  next.currentPromptId = prompt.prompt_id;
  next.activeCategoryId = categoryId;
  if (!next.focusedCategoryId) {
    next.focusedCategoryId = categoryId;
  }

  return next;
}

export function mergeSocraticDocumentGraph(
  state: SocraticDocumentGraphState,
  payload: SocraticDocumentHydrationPayload,
): SocraticDocumentGraphState {
  const { workspace, currentPrompt } = payload;
  const next: SocraticDocumentGraphState = {
    ...state,
    categoriesById: { ...state.categoriesById },
    questionsById: { ...state.questionsById },
    draftsByQuestionId: { ...state.draftsByQuestionId },
    activeCategoryPath: workspace.category_snapshot.active_category_path,
  };

  const snapshot = workspace.category_snapshot;
  const snapshotIds = snapshot.nodes.map((node) => node.category_id);

  next.revision = snapshot.revision;
  next.categoryOrder = dedupeOrder([...snapshotIds, ...state.categoryOrder]);
  next.buildReady = snapshot.build_ready;
  next.buildReadinessMessage = snapshot.build_readiness_message;
  next.focusedCategoryId =
    workspace.focused_category_id
    ?? currentPrompt?.origin_category_id
    ?? activePathCategoryId(workspace)
    ?? state.focusedCategoryId;
  next.activeCategoryId = currentPrompt
    ? promptCategoryId(currentPrompt)
    : activePathCategoryId(workspace) ?? next.focusedCategoryId;
  next.currentPromptId = currentPrompt?.prompt_id ?? state.currentPromptId;

  for (const node of snapshot.nodes) {
    next.categoriesById[node.category_id] = upsertCategoryFromNode(
      state.categoriesById[node.category_id],
      node,
      snapshot.newly_available_category_ids.includes(node.category_id),
    );
  }

  for (const group of workspace.groups) {
    const previous = next.categoriesById[group.category_id];
    if (!previous) continue;
    next.categoriesById[group.category_id] = {
      ...previous,
      summary: group.summary || previous.summary,
      status: group.status,
      itemCountHint: Math.max(previous.itemCountHint, group.question_count),
    };
  }

  if (currentPrompt) {
    mergePromptIntoGraph(next, currentPrompt);
  }

  return next;
}

function syncQuestionDraft(
  state: SocraticDocumentGraphState,
  questionId: string,
  draft: QuestionDraft,
): SocraticDocumentGraphState {
  const previous = state.draftsByQuestionId[questionId] ?? EMPTY_QUESTION_DRAFT;
  if (
    previous.selectedOptionId === draft.selectedOptionId
    && previous.customText === draft.customText
  ) {
    return state;
  }

  return {
    ...state,
    draftsByQuestionId: {
      ...state.draftsByQuestionId,
      [questionId]: draft,
    },
  };
}

function clearQuestionDrafts(
  state: SocraticDocumentGraphState,
  questionIds: string[],
): SocraticDocumentGraphState {
  if (questionIds.length === 0) return state;
  const nextDrafts = { ...state.draftsByQuestionId };
  let changed = false;
  for (const questionId of questionIds) {
    if (!(questionId in nextDrafts)) continue;
    delete nextDrafts[questionId];
    changed = true;
  }
  if (!changed) return state;
  return {
    ...state,
    draftsByQuestionId: nextDrafts,
  };
}

const socraticDocumentGraphAtom = atom<SocraticDocumentGraphState>(EMPTY_SOCRATIC_DOCUMENT_GRAPH_STATE);
const mergeSocraticDocumentGraphAtom = atom(
  null,
  (get, set, payload: SocraticDocumentHydrationPayload) => {
    set(socraticDocumentGraphAtom, mergeSocraticDocumentGraph(get(socraticDocumentGraphAtom), payload));
  },
);
const resetSocraticDocumentGraphAtom = atom(
  null,
  (_get, set) => set(socraticDocumentGraphAtom, EMPTY_SOCRATIC_DOCUMENT_GRAPH_STATE),
);
const syncSocraticDocumentDraftAtom = atom(
  null,
  (get, set, payload: { questionId: string; draft: QuestionDraft }) => {
    set(
      socraticDocumentGraphAtom,
      syncQuestionDraft(get(socraticDocumentGraphAtom), payload.questionId, payload.draft),
    );
  },
);
const clearSocraticDocumentDraftsAtom = atom(
  null,
  (get, set, questionIds: string[]) => {
    set(socraticDocumentGraphAtom, clearQuestionDrafts(get(socraticDocumentGraphAtom), questionIds));
  },
);
const orderedCategoryViewsAtom = atom((get) => buildSocraticDocumentCategoryViews(get(socraticDocumentGraphAtom)));
const knownQuestionCountAtom = atom((get) => Object.keys(get(socraticDocumentGraphAtom).questionsById).length);

const defaultStore = getDefaultStore();

export function resetSocraticDocumentGraph(): void {
  defaultStore.set(resetSocraticDocumentGraphAtom);
}

export function hydrateSocraticDocumentGraph(payload: SocraticDocumentHydrationPayload): void {
  defaultStore.set(mergeSocraticDocumentGraphAtom, payload);
}

export function syncSocraticDocumentDraft(questionId: string, draft: QuestionDraft): void {
  defaultStore.set(syncSocraticDocumentDraftAtom, { questionId, draft });
}

export function clearSocraticDocumentDrafts(questionIds: string[]): void {
  defaultStore.set(clearSocraticDocumentDraftsAtom, questionIds);
}

export function getSocraticDocumentGraphState(): SocraticDocumentGraphState {
  return defaultStore.get(socraticDocumentGraphAtom);
}

export function getSocraticDocumentPromptDrafts(prompt: PromptEnvelope): Record<string, QuestionDraft> {
  const state = getSocraticDocumentGraphState();
  return prompt.items.reduce<Record<string, QuestionDraft>>((drafts, item) => {
    const draft = state.draftsByQuestionId[item.item_id];
    if (!draft) return drafts;
    drafts[item.item_id] = draft;
    return drafts;
  }, {});
}

export function useHydrateSocraticDocumentGraph(
  workspace: SocraticWorkspaceSnapshot,
  currentPrompt: PromptEnvelope | null,
): void {
  useEffect(() => {
    hydrateSocraticDocumentGraph({ workspace, currentPrompt });
  }, [workspace, currentPrompt]);
}

export function useSocraticDocumentCategoryViews(): SocraticDocumentCategoryView[] {
  return useAtomValue(orderedCategoryViewsAtom);
}

export function useSocraticDocumentKnownQuestionCount(): number {
  return useAtomValue(knownQuestionCountAtom);
}

export function useSocraticDocumentQuestions(categoryId: string | null): SocraticDocumentQuestion[] {
  const categoryQuestionsAtom = useMemo(() => atom((get) => {
    const state = get(socraticDocumentGraphAtom);
    if (!categoryId) return [] as SocraticDocumentQuestion[];
    const questionIds = state.categoriesById[categoryId]?.questionIds ?? [];
    return questionIds.flatMap((questionId) => {
      const question = state.questionsById[questionId];
      return question ? [question] : [];
    });
  }), [categoryId]);

  return useAtomValue(categoryQuestionsAtom);
}

export function useSocraticDocumentQuestionDraft(questionId: string): QuestionDraft {
  const questionDraftAtom = useMemo(() => atom((get) => {
    const state = get(socraticDocumentGraphAtom);
    return state.draftsByQuestionId[questionId] ?? EMPTY_QUESTION_DRAFT;
  }), [questionId]);

  return useAtomValue(questionDraftAtom);
}

if (typeof window !== 'undefined' && import.meta.env.DEV) {
  window.__plannerSocraticDocumentTest = {
    getState: getSocraticDocumentGraphState,
    hydrate: hydrateSocraticDocumentGraph,
    reset: resetSocraticDocumentGraph,
  };
}
