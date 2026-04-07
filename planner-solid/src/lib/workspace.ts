import type { PromptAnswer, PromptEnvelope, PromptItem, SavedPromptAnswerDraft } from "./types";

export type DraftEntry = {
  selectedOptionId?: string | null;
  customText?: string;
  structuredPayload?: PromptAnswer["structured_payload"];
};

export function draftEntryFromSavedDraft(
  draft: SavedPromptAnswerDraft | undefined,
): DraftEntry | undefined {
  if (!draft) return undefined;
  const entry: DraftEntry = {
    selectedOptionId: draft.selected_option_id ?? null,
    customText: draft.custom_text ?? "",
  };
  if (draft.structured_payload) {
    entry.structuredPayload = draft.structured_payload;
  }
  return entry;
}

export function draftHasContent(draft: DraftEntry | undefined): boolean {
  if (!draft) return false;
  return Boolean(
    draft.selectedOptionId
      || draft.customText?.trim()
      || draft.structuredPayload?.ordered_option_ids?.length
      || Object.keys(draft.structuredPayload?.field_values ?? {}).length
      || draft.structuredPayload?.scalar_value !== undefined
      || draft.structuredPayload?.selected_path?.trim(),
  );
}

export function countAnsweredPromptItems(
  prompt: PromptEnvelope,
  draftsByItemId: Record<string, DraftEntry | undefined>,
): number {
  return prompt.items.reduce((count, item) => {
    if (draftHasContent(draftsByItemId[item.item_id])) {
      return count + 1;
    }
    return count;
  }, 0);
}

export function countProcessedPromptItems(
  prompt: PromptEnvelope,
  processedByItemId: Record<string, boolean | undefined>,
): number {
  return prompt.items.reduce((count, item) => (processedByItemId[item.item_id] ? count + 1 : count), 0);
}

export function firstUnprocessedPromptItemId(
  prompt: PromptEnvelope,
  processedByItemId: Record<string, boolean | undefined>,
): string | null {
  return prompt.items.find((item) => !processedByItemId[item.item_id])?.item_id ?? null;
}

export function describePromptItemProjection(
  item: PromptItem,
  draft: DraftEntry | undefined,
): string[] {
  if (!draftHasContent(draft)) return [];

  const fragments: string[] = [];
  if (draft?.selectedOptionId) {
    const option = item.options.find((candidate) => candidate.option_id === draft.selectedOptionId);
    if (option?.label) {
      fragments.push(option.label);
    }
  }

  const text = draft?.customText?.trim();
  if (text) {
    fragments.push(text);
  }

  return [...new Set(fragments)];
}

export function presentSessionTitle(session: {
  title?: string | null;
  project_description?: string | null;
  id: string;
}) {
  const title = session.title?.trim();
  if (title) return title;
  const description = session.project_description?.trim();
  if (description) return description.slice(0, 96);
  return `Session ${session.id.slice(0, 8)}`;
}

export function buildPromptAnswers(
  prompt: PromptEnvelope,
  promptDrafts: Record<string, DraftEntry> | undefined,
): PromptAnswer[] {
  return prompt.items.map((item) => buildPromptAnswer(item.item_id, promptDrafts?.[item.item_id]));
}

export function buildPromptAnswer(itemId: string, draft: DraftEntry | undefined): PromptAnswer {
  const text = draft?.customText?.trim();
  const hasSelection = !!draft?.selectedOptionId;
  const hasText = !!text;
  const hasStructuredPayload = Boolean(
    draft?.structuredPayload?.ordered_option_ids?.length
      || Object.keys(draft?.structuredPayload?.field_values ?? {}).length
      || draft?.structuredPayload?.scalar_value !== undefined
      || draft?.structuredPayload?.selected_path?.trim(),
  );

  if (!hasSelection && !hasText && !hasStructuredPayload) {
    return { item_id: itemId, skipped: true };
  }

  const answer: PromptAnswer = {
    item_id: itemId,
    selected_option_id: draft?.selectedOptionId ?? null,
    custom_text: text || null,
  };
  if (draft?.structuredPayload) {
    answer.structured_payload = draft.structuredPayload;
  }
  return answer;
}

export function buildSessionExportFilename(session: {
  id: string;
  title?: string | null;
  project_description?: string | null;
}) {
  const base = presentSessionTitle(session)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
  return `${base || `session-${session.id.slice(0, 8)}`}.json`;
}
