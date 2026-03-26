import { useNavigate, useParams } from "@solidjs/router";
import {
  batch,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  onCleanup,
  untrack,
  type Accessor,
} from "solid-js";

import {
  buildSocraticWebSocketUrl,
  duplicateSession,
  exportSession,
  getPromptBank,
  getSession,
  restartSessionFromDescription,
  retrySessionPipeline,
  savePromptDrafts,
} from "~/lib/api";
import {
  emptyPromptBankGraph,
  mergePromptBankGraph,
  revealPromptBankWorkspace,
  type PromptBankGraph,
} from "~/lib/prompt-bank";
import {
  canRetryStartup,
  getSessionStatusCopy,
  needsSavedBriefAction,
  shouldOpenSessionSocket,
  shouldSendStartupHandshake,
} from "~/lib/session-status";
import type {
  ClientPromptResponseMessage,
  ClientStartSocraticMessage,
  GetSessionResponse,
  PromptBankResponse,
  PromptBankThread,
  Session,
} from "~/lib/types";
import {
  buildPromptAnswers,
  buildSessionExportFilename,
  countProcessedPromptItems,
  draftEntryFromSavedDraft,
  draftHasContent,
  firstUnprocessedPromptItemId,
  presentSessionTitle,
  type DraftEntry,
} from "~/lib/workspace";
import {
  type DraftSaveState,
  type SurfaceTab,
  viewportClassFromWidth,
} from "./session-workspace-view";

interface UpcomingTask {
  threadId: string;
  threadTitle: string;
  itemId: string;
  text: string;
}

type SessionActionPending = null | "duplicate" | "export" | "restart" | "retry";

export interface SessionWorkspaceController {
  session: Accessor<GetSessionResponse | undefined>;
  currentSession: Accessor<Session | null>;
  bankedThreads: Accessor<PromptBankThread[]>;
  queuedThreads: Accessor<PromptBankResponse["queued_threads"]>;
  selectedThread: Accessor<PromptBankThread | null>;
  activeItem: Accessor<PromptBankThread["prompt"]["items"][number] | null>;
  activeItemIndex: Accessor<number>;
  activeThreadProgress: Accessor<number>;
  upcomingTasks: Accessor<UpcomingTask[]>;
  promptBankGraph: Accessor<PromptBankGraph>;
  draftsByQuestionId: Accessor<Record<string, DraftEntry>>;
  processedByItemId: Accessor<Record<string, boolean>>;
  socketState: Accessor<"connecting" | "open" | "closed" | "error">;
  isCollapsedLayout: Accessor<boolean>;
  surfaceTab: Accessor<SurfaceTab>;
  submitError: Accessor<string | null>;
  actionNotice: Accessor<string | null>;
  actionError: Accessor<string | null>;
  actionPending: Accessor<SessionActionPending>;
  submittingThreadId: Accessor<string | null>;
  draftSaveState: Accessor<DraftSaveState>;
  draftSaveMessage: Accessor<string | null>;
  sessionStatus: Accessor<ReturnType<typeof getSessionStatusCopy> | null>;
  workspaceReady: Accessor<boolean>;
  needsSavedBriefAction: Accessor<boolean>;
  canRetryStartup: Accessor<boolean>;
  handleSurfaceTabChange: (tab: SurfaceTab) => void;
  setActiveTask: (threadId: string, itemId: string | null, flushCurrent?: boolean) => void;
  handleDraftChange: (thread: PromptBankThread, itemId: string, next: DraftEntry) => void;
  handleCommitAnswer: (thread: PromptBankThread, itemId: string, nextDraft?: DraftEntry) => Promise<void>;
  handleCommitCurrentAnswer: (nextDraft?: DraftEntry) => Promise<void>;
  handleDuplicate: (currentSession: Session) => Promise<void>;
  handleExport: (currentSession: Session) => Promise<void>;
  handleRestart: (currentSession: Session) => Promise<void>;
  handleRetry: (currentSession: Session) => Promise<void>;
  handleRetryStartup: (currentSession: Session) => Promise<void>;
  registerInputRef: (itemId: string, element: HTMLTextAreaElement) => void;
}

export function useSessionWorkspaceController(): SessionWorkspaceController {
  const params = useParams();
  const navigate = useNavigate();
  const [session, { refetch: refetchSession }] = createResource(() => params.sessionId, getSession);
  const [promptBank, { refetch: refetchPromptBank }] = createResource(() => params.sessionId, getPromptBank);
  const [draftsByQuestionId, setDraftsByQuestionId] = createSignal<Record<string, DraftEntry>>({});
  const [processedByItemId, setProcessedByItemId] = createSignal<Record<string, boolean>>({});
  const [promptBankGraph, setPromptBankGraph] = createSignal(emptyPromptBankGraph());
  const [socketState, setSocketState] = createSignal<"connecting" | "open" | "closed" | "error">("closed");
  const [windowWidth, setWindowWidth] = createSignal(typeof window === "undefined" ? 1280 : window.innerWidth);
  const [surfaceTab, setSurfaceTab] = createSignal<SurfaceTab>("interview");
  const [activeItemId, setActiveItemId] = createSignal<string | null>(null);
  const [submitError, setSubmitError] = createSignal<string | null>(null);
  const [actionNotice, setActionNotice] = createSignal<string | null>(null);
  const [actionError, setActionError] = createSignal<string | null>(null);
  const [actionPending, setActionPending] = createSignal<SessionActionPending>(null);
  const [startupRetryNonce, setStartupRetryNonce] = createSignal(0);
  const [submittingThreadId, setSubmittingThreadId] = createSignal<string | null>(null);
  const [draftSaveState, setDraftSaveState] = createSignal<DraftSaveState>("idle");
  const [draftSaveMessage, setDraftSaveMessage] = createSignal<string | null>(null);

  let socket: WebSocket | null = null;
  let draftSaveTimer: number | undefined;
  const lastSavedSignatureByPromptId: Record<string, string> = {};
  const inputRefs = new Map<string, HTMLTextAreaElement>();

  const currentSession = createMemo(() => session()?.session ?? null);
  const isCollapsedLayout = createMemo(() => windowWidth() < 1024);
  const viewportClass = createMemo(() => viewportClassFromWidth(windowWidth()));
  const bankedThreads = createMemo(() =>
    promptBankGraph()
      .threadOrder.map(threadId => promptBankGraph().threadsById[threadId])
      .filter((thread): thread is NonNullable<typeof thread> => !!thread),
  );
  const queuedThreads = createMemo(() =>
    promptBankGraph()
      .queuedThreadIds.map(threadId => promptBankGraph().queuedById[threadId])
      .filter((thread): thread is NonNullable<typeof thread> => !!thread),
  );
  const selectedThread = createMemo(() => {
    const selectedId = promptBankGraph().activeThreadId;
    if (selectedId) {
      const selected = promptBankGraph().threadsById[selectedId];
      if (selected) return selected;
    }
    return bankedThreads()[0] ?? null;
  });
  const activeItem = createMemo(() => {
    const thread = selectedThread();
    if (!thread) return null;
    const requestedId = activeItemId();
    if (requestedId) {
      const existing = thread.prompt.items.find(item => item.item_id === requestedId);
      if (existing) return existing;
    }
    const nextId = firstUnprocessedPromptItemId(thread.prompt, processedByItemId());
    return thread.prompt.items.find(item => item.item_id === nextId) ?? thread.prompt.items[0] ?? null;
  });
  const activeItemIndex = createMemo(() => {
    const thread = selectedThread();
    const item = activeItem();
    if (!thread || !item) return 0;
    return thread.prompt.items.findIndex(candidate => candidate.item_id === item.item_id);
  });
  const activeThreadProgress = createMemo(() => {
    const thread = selectedThread();
    if (!thread) return 0;
    return countProcessedPromptItems(thread.prompt, processedByItemId());
  });
  const upcomingTasks = createMemo<UpcomingTask[]>(() => {
    const currentThread = selectedThread();
    const currentItem = activeItem();
    if (!currentThread || !currentItem) return [];

    const orderedTasks = bankedThreads().flatMap(thread =>
      thread.prompt.items.map(item => ({
        threadId: thread.category_id,
        threadTitle: thread.title,
        itemId: item.item_id,
        text: item.text,
      })),
    );
    const currentIndex = orderedTasks.findIndex(
      task => task.threadId === currentThread.category_id && task.itemId === currentItem.item_id,
    );
    if (currentIndex < 0) return [];

    return orderedTasks
      .slice(currentIndex + 1)
      .filter(task => !processedByItemId()[task.itemId])
      .slice(0, 3);
  });
  const sessionStatus = createMemo(() => {
    const activeSession = currentSession();
    return activeSession ? getSessionStatusCopy(activeSession, socketState()) : null;
  });
  const workspaceReady = createMemo(() => {
    const activeSession = currentSession();
    return activeSession ? revealPromptBankWorkspace(promptBankGraph(), activeSession.intake_phase) : false;
  });
  const needsSavedBriefActionState = createMemo(() => {
    const activeSession = currentSession();
    return activeSession ? needsSavedBriefAction(activeSession) : false;
  });
  const canRetryStartupState = createMemo(() => {
    const activeSession = currentSession();
    return activeSession ? canRetryStartup(activeSession, socketState()) : false;
  });

  const promptDrafts = (itemIds: string[]) =>
    itemIds.reduce<Record<string, DraftEntry>>((drafts, itemId) => {
      const draft = draftsByQuestionId()[itemId];
      if (draft) drafts[itemId] = draft;
      return drafts;
    }, {});

  const promptDraftsWithOverride = (
    prompt: PromptBankThread["prompt"],
    override?: { itemId: string; draft: DraftEntry | undefined },
  ) => {
    const drafts = promptDrafts(prompt.items.map(item => item.item_id));
    if (!override) return drafts;
    if (draftHasContent(override.draft)) {
      drafts[override.itemId] = override.draft as DraftEntry;
    } else {
      delete drafts[override.itemId];
    }
    return drafts;
  };

  const flushDraftSave = async (
    prompt: PromptBankThread["prompt"],
    override?: { itemId: string; draft: DraftEntry | undefined },
  ) => {
    if (draftSaveTimer !== undefined) {
      window.clearTimeout(draftSaveTimer);
      draftSaveTimer = undefined;
    }
    return persistDraftsForPrompt(prompt, override);
  };

  const setActiveTask = (threadId: string, itemId: string | null, flushCurrent = true) => {
    const currentThread = selectedThread();
    if (flushCurrent && currentThread) {
      void flushDraftSave(currentThread.prompt);
    }
    setPromptBankGraph(previous => ({
      ...previous,
      activeThreadId: threadId,
    }));
    setActiveItemId(itemId);
    if (isCollapsedLayout()) {
      setSurfaceTab("interview");
    }
  };

  const mergeServerDrafts = (nextBank: PromptBankResponse) => {
    const nextGraph = mergePromptBankGraph(nextBank, untrack(promptBankGraph));
    const visibleItemIds = new Set(Object.keys(nextGraph.questionsById));
    const serverDrafts = Object.fromEntries(
      Object.entries(nextBank.saved_drafts ?? {})
        .map(([itemId, draft]) => [itemId, draftEntryFromSavedDraft(draft)])
        .filter((entry): entry is [string, DraftEntry] => !!entry[1]),
    );

    setPromptBankGraph(nextGraph);
    setDraftsByQuestionId(previous => {
      const next: Record<string, DraftEntry> = {};
      for (const itemId of visibleItemIds) {
        const existing = previous[itemId];
        if (draftHasContent(existing)) {
          next[itemId] = existing;
          continue;
        }
        const saved = serverDrafts[itemId];
        if (saved) next[itemId] = saved;
      }
      return next;
    });
    setProcessedByItemId(previous => {
      const next: Record<string, boolean> = {};
      for (const itemId of visibleItemIds) {
        if (previous[itemId]) {
          next[itemId] = true;
        }
      }
      return next;
    });
  };

  const persistDraftsForPrompt = async (
    prompt: PromptBankThread["prompt"],
    override?: { itemId: string; draft: DraftEntry | undefined },
  ) => {
    if (!params.sessionId) return false;

    const answers = buildPromptAnswers(prompt, promptDraftsWithOverride(prompt, override));
    const signature = JSON.stringify(answers);
    if (lastSavedSignatureByPromptId[prompt.prompt_id] === signature && draftSaveState() !== "dirty") {
      return true;
    }

    setDraftSaveState("saving");
    setDraftSaveMessage(null);

    try {
      const response = await savePromptDrafts(params.sessionId, {
        promptId: prompt.prompt_id,
        answers,
      });
      lastSavedSignatureByPromptId[prompt.prompt_id] = signature;
      setDraftSaveState("saved");
      setDraftSaveMessage(response.saved_count > 0 ? "Draft saved" : "Draft cleared");
      return true;
    } catch (error) {
      setDraftSaveState("error");
      setDraftSaveMessage(error instanceof Error ? error.message : "Unable to save drafts.");
      return false;
    }
  };

  const scheduleDraftSave = (prompt: PromptBankThread["prompt"]) => {
    if (draftSaveTimer !== undefined) {
      window.clearTimeout(draftSaveTimer);
    }
    setDraftSaveState("dirty");
    setDraftSaveMessage(null);
    draftSaveTimer = window.setTimeout(() => {
      void persistDraftsForPrompt(prompt);
    }, 500);
  };

  const findNextTask = (
    currentThreadId: string,
    currentItemId: string,
    nextProcessedByItemId: Record<string, boolean>,
  ) => {
    const orderedTasks = bankedThreads().flatMap(thread =>
      thread.prompt.items.map(item => ({
        threadId: thread.category_id,
        itemId: item.item_id,
      })),
    );
    const currentIndex = orderedTasks.findIndex(
      task => task.threadId === currentThreadId && task.itemId === currentItemId,
    );
    if (currentIndex < 0) return null;

    return orderedTasks.slice(currentIndex + 1).find(task => !nextProcessedByItemId[task.itemId]) ?? null;
  };

  const submitThread = async (thread: PromptBankThread) => {
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      setSubmitError("Live interview connection is not ready.");
      return false;
    }

    setSubmittingThreadId(thread.category_id);
    setSubmitError(null);

    const message: ClientPromptResponseMessage = {
      type: "prompt_response",
      prompt_id: thread.prompt.prompt_id,
      answers: buildPromptAnswers(
        thread.prompt,
        promptDrafts(thread.prompt.items.map(item => item.item_id)),
      ),
      submitted_at: new Date().toISOString(),
      client_context: {
        viewport_class: viewportClass(),
      },
    };

    socket.send(JSON.stringify(message));
    return true;
  };

  const handleCommitAnswer = async (
    thread: PromptBankThread,
    itemId: string,
    nextDraft?: DraftEntry,
  ) => {
    const item = thread.prompt.items.find(candidate => candidate.item_id === itemId);
    if (!item) return;

    setActiveTask(thread.category_id, item.item_id, false);
    const currentDraft = nextDraft ?? draftsByQuestionId()[item.item_id];
    if (item.required && !draftHasContent(currentDraft)) {
      setSubmitError("This prompt needs an answer before you can continue.");
      return;
    }

    setDraftsByQuestionId(previous => {
      const updated = { ...previous };
      if (draftHasContent(currentDraft)) {
        updated[item.item_id] = currentDraft as DraftEntry;
      } else {
        delete updated[item.item_id];
      }
      return updated;
    });

    const draftsSaved = await flushDraftSave(thread.prompt, {
      itemId: item.item_id,
      draft: currentDraft,
    });
    if (!draftsSaved) {
      setSubmitError("Could not save the latest draft changes before continuing.");
      return;
    }
    const nextProcessedByItemId = {
      ...processedByItemId(),
      [item.item_id]: true,
    };
    const threadProcessed = thread.prompt.items.every(candidate => nextProcessedByItemId[candidate.item_id]);
    const nextTask = findNextTask(thread.category_id, item.item_id, nextProcessedByItemId);
    batch(() => {
      setSubmitError(null);
      setProcessedByItemId(nextProcessedByItemId);
    });

    if (nextTask) {
      setActiveTask(nextTask.threadId, nextTask.itemId, false);
      queueMicrotask(() => inputRefs.get(nextTask.itemId)?.focus());
    } else {
      setActiveItemId(item.item_id);
      if (isCollapsedLayout()) {
        setSurfaceTab("artifact");
      }
    }

    if (threadProcessed) {
      void submitThread(thread);
    }
  };

  const handleCommitCurrentAnswer = async (nextDraft?: DraftEntry) => {
    const thread = selectedThread();
    const item = activeItem();
    if (!thread || !item) return;
    await handleCommitAnswer(thread, item.item_id, nextDraft);
  };

  const handleDraftChange = (thread: PromptBankThread, itemId: string, next: DraftEntry) => {
    setSubmitError(null);
    setDraftsByQuestionId(previous => {
      const updated = { ...previous };
      if (draftHasContent(next)) {
        updated[itemId] = next;
      } else {
        delete updated[itemId];
      }
      return updated;
    });
    scheduleDraftSave(thread.prompt);
  };

  const clearActionFeedback = () => {
    setActionNotice(null);
    setActionError(null);
  };

  const handleDuplicate = async (activeSession: Session) => {
    setActionPending("duplicate");
    clearActionFeedback();
    try {
      const response = await duplicateSession(activeSession.id, {
        title: `${presentSessionTitle(activeSession)} copy`,
      });
      navigate(`/sessions/${response.session.id}`);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "Unable to duplicate the session.");
    } finally {
      setActionPending(null);
    }
  };

  const handleExport = async (activeSession: Session) => {
    setActionPending("export");
    clearActionFeedback();
    try {
      const response = await exportSession(activeSession.id);
      const blob = new Blob([JSON.stringify(response, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = buildSessionExportFilename(activeSession);
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
      setActionNotice(`Exported ${link.download}`);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "Unable to export the session.");
    } finally {
      setActionPending(null);
    }
  };

  const resetAfterLifecycleAction = async (notice: string, resetPromptBankGraph: boolean) => {
    await Promise.all([refetchSession(), refetchPromptBank()]);
    setDraftsByQuestionId({});
    setProcessedByItemId({});
    if (resetPromptBankGraph) {
      setPromptBankGraph(emptyPromptBankGraph());
    }
    setDraftSaveState("idle");
    setDraftSaveMessage(null);
    setSurfaceTab("interview");
    setActionNotice(notice);
  };

  const handleRestart = async (activeSession: Session) => {
    setActionPending("restart");
    clearActionFeedback();
    try {
      if (socket) {
        socket.close();
        socket = null;
      }
      setSocketState("closed");
      await restartSessionFromDescription(activeSession.id);
      await resetAfterLifecycleAction("Session reset to the original description.", true);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "Unable to restart from description.");
    } finally {
      setActionPending(null);
    }
  };

  const handleRetry = async (activeSession: Session) => {
    setActionPending("retry");
    clearActionFeedback();
    try {
      if (socket) {
        socket.close();
        socket = null;
      }
      setSocketState("closed");
      await retrySessionPipeline(activeSession.id);
      await resetAfterLifecycleAction("Pipeline retry started.", false);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "Unable to retry the pipeline.");
    } finally {
      setActionPending(null);
    }
  };

  const handleRetryStartup = async (activeSession: Session) => {
    clearActionFeedback();

    if (activeSession.intake_phase === "error") {
      await handleRestart(activeSession);
      return;
    }

    if (socket) {
      socket.close();
      socket = null;
    }
    setSocketState("closed");
    setActionNotice("Retrying startup from the saved brief.");
    setStartupRetryNonce(current => current + 1);
  };

  createEffect(() => {
    const bank = promptBank();
    if (bank) {
      mergeServerDrafts(bank);
    }
  });

  createEffect(() => {
    const thread = selectedThread();
    if (!thread) {
      setActiveItemId(null);
      return;
    }
    const requestedId = activeItemId();
    if (requestedId && thread.prompt.items.some(item => item.item_id === requestedId)) {
      return;
    }
    const nextId = firstUnprocessedPromptItemId(thread.prompt, processedByItemId()) ?? thread.prompt.items[0]?.item_id ?? null;
    if (nextId !== requestedId) {
      setActiveItemId(nextId);
    }
  });

  createEffect(() => {
    const current = session();
    const sessionId = params.sessionId;
    startupRetryNonce();
    if (!current || !sessionId || !shouldOpenSessionSocket(current.session)) {
      if (socket) {
        socket.close();
        socket = null;
      }
      setSocketState("closed");
      return;
    }

    if (socket) return;

    setSocketState("connecting");
    socket = new WebSocket(buildSocraticWebSocketUrl(sessionId));
    const startupDescription = shouldSendStartupHandshake(current.session)
      ? current.session.project_description?.trim() ?? null
      : null;

    socket.onopen = async () => {
      setSocketState("open");
      if (socket && startupDescription) {
        const message: ClientStartSocraticMessage = {
          type: "start_socratic",
          description: startupDescription,
        };
        socket.send(JSON.stringify(message));
      }
      await refetchSession();
    };
    socket.onerror = () => setSocketState("error");
    socket.onclose = () => {
      setSocketState("closed");
      socket = null;
    };
    socket.onmessage = async event => {
      try {
        const payload = JSON.parse(event.data) as { type?: string; bank?: PromptBankResponse };
        if (payload.type === "prompt_bank" && payload.bank) {
          mergeServerDrafts(payload.bank);
          setPromptBankGraph(previous => mergePromptBankGraph(payload.bank as PromptBankResponse, previous));
          void refetchSession();
          setSubmittingThreadId(null);
          return;
        }
        if (
          payload.type === "converged"
          || payload.type === "planner_event"
          || payload.type === "pipeline_complete"
          || payload.type === "error"
        ) {
          await refetchSession();
          await refetchPromptBank();
          setSubmittingThreadId(null);
        }
      } catch {
        // Ignore malformed socket payloads; the next authoritative fetch will recover state.
      }
    };
  });

  if (typeof window !== "undefined") {
    const handleResize = () => {
      setWindowWidth(window.innerWidth);
    };

    window.addEventListener("resize", handleResize);
    onCleanup(() => {
      window.removeEventListener("resize", handleResize);
    });
  }

  onCleanup(() => {
    if (draftSaveTimer !== undefined) {
      window.clearTimeout(draftSaveTimer);
    }
    if (socket) socket.close();
  });

  return {
    session,
    currentSession,
    bankedThreads,
    queuedThreads,
    selectedThread,
    activeItem,
    activeItemIndex,
    activeThreadProgress,
    upcomingTasks,
    promptBankGraph,
    draftsByQuestionId,
    processedByItemId,
    socketState,
    isCollapsedLayout,
    surfaceTab,
    submitError,
    actionNotice,
    actionError,
    actionPending,
    submittingThreadId,
    draftSaveState,
    draftSaveMessage,
    sessionStatus,
    workspaceReady,
    needsSavedBriefAction: needsSavedBriefActionState,
    canRetryStartup: canRetryStartupState,
    handleSurfaceTabChange: setSurfaceTab,
    setActiveTask,
    handleDraftChange,
    handleCommitAnswer,
    handleCommitCurrentAnswer,
    handleDuplicate,
    handleExport,
    handleRestart,
    handleRetry,
    handleRetryStartup,
    registerInputRef: (itemId, element) => {
      inputRefs.set(itemId, element);
    },
  };
}
