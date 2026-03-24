import type {
  PromptBankResponse,
  PromptBankThread,
  PromptEnvelope,
  PromptItem,
  QueuedPromptThread,
} from "./types";

export interface PromptBankQuestionNode {
  id: string;
  threadId: string;
  promptId: string;
  item: PromptItem;
}

export interface PromptBankGraph {
  activeThreadId: string | null;
  threadOrder: string[];
  threadsById: Record<string, PromptBankThread>;
  promptsByThreadId: Record<string, PromptEnvelope>;
  queuedThreadIds: string[];
  queuedById: Record<string, QueuedPromptThread>;
  questionIdsByThreadId: Record<string, string[]>;
  questionsById: Record<string, PromptBankQuestionNode>;
  buildReady: boolean;
  buildReadinessMessage?: string | null;
  initialBankComplete: boolean;
}

export function emptyPromptBankGraph(): PromptBankGraph {
  return {
    activeThreadId: null,
    threadOrder: [],
    threadsById: {},
    promptsByThreadId: {},
    queuedThreadIds: [],
    queuedById: {},
    questionIdsByThreadId: {},
    questionsById: {},
    buildReady: false,
    buildReadinessMessage: null,
    initialBankComplete: false,
  };
}

export function mergePromptBankGraph(
  response: PromptBankResponse,
  previous: PromptBankGraph = emptyPromptBankGraph(),
): PromptBankGraph {
  const threadsById: Record<string, PromptBankThread> = {};
  const promptsByThreadId: Record<string, PromptEnvelope> = {};
  const questionIdsByThreadId: Record<string, string[]> = {};
  const questionsById: Record<string, PromptBankQuestionNode> = {};

  for (const thread of response.banked_threads) {
    threadsById[thread.category_id] = thread;
    promptsByThreadId[thread.category_id] = thread.prompt;
    questionIdsByThreadId[thread.category_id] = thread.prompt.items.map((item) => item.item_id);
    for (const item of thread.prompt.items) {
      questionsById[item.item_id] = {
        id: item.item_id,
        threadId: thread.category_id,
        promptId: thread.prompt.prompt_id,
        item,
      };
    }
  }

  const queuedById: Record<string, QueuedPromptThread> = {};
  for (const thread of response.queued_threads) {
    queuedById[thread.category_id] = thread;
  }

  const requestedActiveThreadId = response.active_thread_id ?? previous.activeThreadId;
  const activeThreadId = requestedActiveThreadId && threadsById[requestedActiveThreadId]
    ? requestedActiveThreadId
    : response.banked_threads[0]?.category_id ?? null;

  return {
    activeThreadId,
    threadOrder: response.banked_threads.map((thread) => thread.category_id),
    threadsById,
    promptsByThreadId,
    queuedThreadIds: response.queued_threads.map((thread) => thread.category_id),
    queuedById,
    questionIdsByThreadId,
    questionsById,
    buildReady: response.build_ready,
    buildReadinessMessage: response.build_readiness_message ?? null,
    initialBankComplete: response.initial_bank_complete,
  };
}

export function revealPromptBankWorkspace(
  bank: PromptBankGraph,
  intakePhase: string,
): boolean {
  return bank.initialBankComplete || bank.buildReady || intakePhase === "error";
}
