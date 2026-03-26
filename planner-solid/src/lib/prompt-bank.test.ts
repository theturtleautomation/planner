import { emptyPromptBankGraph, mergePromptBankGraph, revealPromptBankWorkspace } from "./prompt-bank";
import type { PromptBankResponse } from "./types";

const baseResponse = (overrides: Partial<PromptBankResponse> = {}): PromptBankResponse => ({
  session_id: "session-1",
  active_thread_id: "verify-platform",
  banked_threads: [
    {
      category_id: "verify-platform",
      title: "Verify Platform",
      summary: "Confirm the first delivery surface.",
      question_count: 1,
      prompt: {
        prompt_id: "prompt-verify-platform",
        title: "Verify Platform",
        kind: "verification_batch",
        origin_category_id: "verify-platform",
        items: [
          {
            item_id: "platform-choice",
            kind: "verification",
            text: "Should this ship as a web app first?",
            options: [],
            required: true,
          },
        ],
        allow_partial_submit: true,
      },
    },
  ],
  queued_threads: [],
  build_ready: false,
  build_readiness_message: null,
  initial_bank_complete: true,
  ...overrides,
});

describe("prompt bank graph helpers", () => {
  it("normalizes banked prompt threads into a thread and question graph", () => {
    const graph = mergePromptBankGraph(baseResponse());

    expect(graph.activeThreadId).toBe("verify-platform");
    expect(graph.threadOrder).toEqual(["verify-platform"]);
    expect(graph.promptsByThreadId["verify-platform"]?.prompt_id).toBe("prompt-verify-platform");
    expect(graph.questionIdsByThreadId["verify-platform"]).toEqual(["platform-choice"]);
    expect(graph.questionsById["platform-choice"]?.threadId).toBe("verify-platform");
  });

  it("keeps saved drafts keyed by prompt item id", () => {
    const graph = mergePromptBankGraph(
      baseResponse({
        saved_drafts: {
          "platform-choice": {
            prompt_id: "prompt-verify-platform",
            item_id: "platform-choice",
            selected_option_id: "web",
            custom_text: "Start on the web first.",
            skipped: false,
            updated_at: "2026-03-25T00:00:00Z",
          },
        },
      }),
    );

    expect(graph.savedDraftsByItemId["platform-choice"]).toMatchObject({
      prompt_id: "prompt-verify-platform",
      selected_option_id: "web",
      custom_text: "Start on the web first.",
    });
  });

  it("preserves a locally selected active thread when the server omits it", () => {
    const previous = mergePromptBankGraph(
      baseResponse({
        active_thread_id: "queue-first",
        banked_threads: [
          {
            category_id: "queue-first",
            title: "Queue First",
            summary: "Local thread selection",
            question_count: 1,
            prompt: {
              prompt_id: "prompt-queue-first",
              title: "Queue First",
              kind: "question_batch",
              origin_category_id: "queue-first",
              items: [],
              allow_partial_submit: true,
            },
          },
          ...baseResponse().banked_threads,
        ],
      }),
    );

    const graph = mergePromptBankGraph(
      baseResponse({
        active_thread_id: null,
        banked_threads: [
          previous.threadsById["queue-first"],
          ...baseResponse().banked_threads,
        ],
      }),
      previous,
    );

    expect(graph.activeThreadId).toBe("queue-first");
  });

  it("holds first reveal until the prompt bank is complete or build ready", () => {
    const assembling = mergePromptBankGraph(
      baseResponse({
        initial_bank_complete: false,
      }),
      emptyPromptBankGraph(),
    );

    expect(revealPromptBankWorkspace(assembling, "interviewing")).toBe(false);
    expect(
      revealPromptBankWorkspace(
        mergePromptBankGraph(
          baseResponse({
            initial_bank_complete: false,
            build_ready: true,
          }),
        ),
        "interviewing",
      ),
    ).toBe(true);
  });

  it("does not reveal from a complete flag unless a real banked thread exists", () => {
    const legacyOnly = mergePromptBankGraph(
      baseResponse({
        active_thread_id: null,
        banked_threads: [],
        initial_bank_complete: true,
      }),
      emptyPromptBankGraph(),
    );

    expect(revealPromptBankWorkspace(legacyOnly, "interviewing")).toBe(false);
  });
});
