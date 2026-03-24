import type { PromptAnswer, PromptEnvelope } from "./types";

export type DraftEntry = {
  selectedOptionId?: string | null;
  customText?: string;
};

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
  return prompt.items.map((item) => {
    const draft = promptDrafts?.[item.item_id];
    const text = draft?.customText?.trim();
    const hasSelection = !!draft?.selectedOptionId;
    const hasText = !!text;

    if (!hasSelection && !hasText) {
      return { item_id: item.item_id, skipped: true };
    }

    return {
      item_id: item.item_id,
      selected_option_id: draft?.selectedOptionId ?? null,
      custom_text: text || null,
    };
  });
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
