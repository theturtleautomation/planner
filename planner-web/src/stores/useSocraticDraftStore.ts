import { create } from 'zustand';
import type { PromptAnswer, PromptEnvelope } from '../types.ts';

export interface QuestionDraft {
  selectedOptionId: string | null;
  customText: string;
}

export interface PromptProgress {
  answeredCount: number;
  totalCount: number;
  missingRequiredCount: number;
  hasAnyAnswer: boolean;
}

interface PromptDraftBucket {
  itemIds: string[];
  items: Record<string, QuestionDraft>;
}

interface SocraticDraftState {
  prompts: Record<string, PromptDraftBucket>;
  primePrompt: (prompt: PromptEnvelope, seededItems?: Record<string, QuestionDraft>) => void;
  setSelectedOption: (promptId: string, itemId: string, selectedOptionId: string | null) => void;
  setCustomText: (promptId: string, itemId: string, customText: string) => void;
  clearItems: (promptId: string, itemIds: string[]) => void;
  clearPrompt: (promptId: string) => void;
}

export const EMPTY_QUESTION_DRAFT: QuestionDraft = Object.freeze({
  selectedOptionId: null,
  customText: '',
});

export const EMPTY_PROMPT_PROGRESS: PromptProgress = Object.freeze({
  answeredCount: 0,
  totalCount: 0,
  missingRequiredCount: 0,
  hasAnyAnswer: false,
});

function ensurePromptBucket(
  prompts: Record<string, PromptDraftBucket>,
  promptId: string,
  itemIds: string[],
): PromptDraftBucket {
  return prompts[promptId] ?? { itemIds, items: {} };
}

export function isQuestionDraftAnswered(draft: QuestionDraft | undefined): boolean {
  if (!draft) return false;
  if (draft.selectedOptionId) return true;
  return draft.customText.trim().length > 0;
}

export function selectQuestionDraft(
  state: Pick<SocraticDraftState, 'prompts'>,
  promptId: string,
  itemId: string,
): QuestionDraft {
  return state.prompts[promptId]?.items[itemId] ?? EMPTY_QUESTION_DRAFT;
}

export function collectPromptAnswers(
  prompt: PromptEnvelope,
  state: Pick<SocraticDraftState, 'prompts'>,
): PromptAnswer[] {
  const drafts = state.prompts[prompt.prompt_id]?.items ?? {};

  return prompt.items.flatMap((item) => {
    const draft = drafts[item.item_id];
    if (!isQuestionDraftAnswered(draft)) return [];

    const customText = draft.customText.trim();
    return [{
      item_id: item.item_id,
      selected_option_id: draft.selectedOptionId ?? undefined,
      custom_text: customText.length > 0 ? customText : undefined,
    }];
  });
}

export function selectPromptProgress(
  state: Pick<SocraticDraftState, 'prompts'>,
  prompt: PromptEnvelope | null,
): PromptProgress {
  if (!prompt) return EMPTY_PROMPT_PROGRESS;

  const drafts = state.prompts[prompt.prompt_id]?.items ?? {};
  const requiredItemIds = prompt.required_item_ids.length > 0
    ? prompt.required_item_ids
    : prompt.items.filter((item) => item.required).map((item) => item.item_id);

  const answeredCount = prompt.items.reduce((count, item) => (
    count + (isQuestionDraftAnswered(drafts[item.item_id]) ? 1 : 0)
  ), 0);
  const missingRequiredCount = requiredItemIds.reduce((count, itemId) => (
    count + (isQuestionDraftAnswered(drafts[itemId]) ? 0 : 1)
  ), 0);

  return {
    answeredCount,
    totalCount: prompt.items.length,
    missingRequiredCount,
    hasAnyAnswer: answeredCount > 0,
  };
}

export const useSocraticDraftStore = create<SocraticDraftState>((set) => ({
  prompts: {},
  primePrompt: (prompt, seededItems = {}) => {
    set((state) => {
      const previous = state.prompts[prompt.prompt_id];
      const itemIds = prompt.items.map((item) => item.item_id);
      const mergedItems = {
        ...seededItems,
        ...(previous?.items ?? {}),
      };

      if (
        previous
        && previous.itemIds.length === itemIds.length
        && previous.itemIds.every((itemId, index) => itemId === itemIds[index])
      ) {
        const seedKeys = Object.keys(seededItems);
        const needsSeed = seedKeys.some((itemId) => {
          const seeded = seededItems[itemId];
          const existing = previous.items[itemId];
          return !existing || existing.selectedOptionId !== seeded.selectedOptionId || existing.customText !== seeded.customText;
        });
        if (!needsSeed) {
          return state;
        }
      }

      if (
        previous
        && previous.itemIds.length === itemIds.length
        && previous.itemIds.every((itemId, index) => itemId === itemIds[index])
        && Object.keys(previous.items).length === Object.keys(mergedItems).length
        && Object.entries(previous.items).every(([itemId, draft]) => {
          const candidate = mergedItems[itemId];
          return candidate?.selectedOptionId === draft.selectedOptionId
            && candidate?.customText === draft.customText;
        })
      ) {
        return state;
      }

      return {
        prompts: {
          ...state.prompts,
          [prompt.prompt_id]: {
            itemIds,
            items: mergedItems,
          },
        },
      };
    });
  },
  setSelectedOption: (promptId, itemId, selectedOptionId) => {
    set((state) => {
      const previousBucket = ensurePromptBucket(state.prompts, promptId, []);
      const previousDraft = previousBucket.items[itemId] ?? EMPTY_QUESTION_DRAFT;
      if (previousDraft.selectedOptionId === selectedOptionId) return state;

      return {
        prompts: {
          ...state.prompts,
          [promptId]: {
            ...previousBucket,
            items: {
              ...previousBucket.items,
              [itemId]: {
                ...previousDraft,
                selectedOptionId,
              },
            },
          },
        },
      };
    });
  },
  setCustomText: (promptId, itemId, customText) => {
    set((state) => {
      const previousBucket = ensurePromptBucket(state.prompts, promptId, []);
      const previousDraft = previousBucket.items[itemId] ?? EMPTY_QUESTION_DRAFT;
      if (previousDraft.customText === customText) return state;

      return {
        prompts: {
          ...state.prompts,
          [promptId]: {
            ...previousBucket,
            items: {
              ...previousBucket.items,
              [itemId]: {
                ...previousDraft,
                customText,
              },
            },
          },
        },
      };
    });
  },
  clearItems: (promptId, itemIds) => {
    set((state) => {
      const previousBucket = state.prompts[promptId];
      if (!previousBucket) return state;

      const nextItems = { ...previousBucket.items };
      let changed = false;

      for (const itemId of itemIds) {
        if (!(itemId in nextItems)) continue;
        delete nextItems[itemId];
        changed = true;
      }

      if (!changed) return state;

      return {
        prompts: {
          ...state.prompts,
          [promptId]: {
            ...previousBucket,
            items: nextItems,
          },
        },
      };
    });
  },
  clearPrompt: (promptId) => {
    set((state) => {
      if (!(promptId in state.prompts)) return state;
      const nextPrompts = { ...state.prompts };
      delete nextPrompts[promptId];
      return { prompts: nextPrompts };
    });
  },
}));
