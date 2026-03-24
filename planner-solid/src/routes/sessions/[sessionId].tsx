import { Title } from "@solidjs/meta";
import { A, useNavigate, useParams } from "@solidjs/router";
import { createEffect, createMemo, createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { createStore } from "solid-js/store";

import {
  buildSocraticWebSocketUrl,
  duplicateSession,
  exportSession,
  getPromptBank,
  getSession,
  restartSessionFromDescription,
  retrySessionPipeline,
} from "~/lib/api";
import {
  buildPromptAnswers,
  buildSessionExportFilename,
  presentSessionTitle,
} from "~/lib/workspace";
import type {
  ClientPromptResponseMessage,
  PromptItem,
  Session,
} from "~/lib/types";
import type { DraftEntry } from "~/lib/workspace";

function viewportClass(): "mobile" | "tablet" | "desktop" {
  if (typeof window === "undefined") return "desktop";
  if (window.innerWidth < 640) return "mobile";
  if (window.innerWidth < 960) return "tablet";
  return "desktop";
}

function QuestionBlock(props: {
  promptId: string;
  item: PromptItem;
  draft?: DraftEntry;
  onDraftChange: (itemId: string, next: DraftEntry) => void;
}) {
  const [selectedOptionId, setSelectedOptionId] = createSignal(props.draft?.selectedOptionId ?? null);
  const [customText, setCustomText] = createSignal(props.draft?.customText ?? "");

  createEffect(() => {
    setSelectedOptionId(props.draft?.selectedOptionId ?? null);
    setCustomText(props.draft?.customText ?? "");
  });

  const pushDraft = (next: DraftEntry) => {
    props.onDraftChange(props.item.item_id, next);
  };

  return (
    <section class="question-block">
      <div class="question-kicker">Q</div>
      <p class="question-text">{props.item.text}</p>
      <Show when={props.item.options.length > 0}>
        <div class="option-row">
          <For each={props.item.options}>
            {(option) => (
              <button
                class={`option-chip${selectedOptionId() === option.option_id ? " is-selected" : ""}`}
                type="button"
                onClick={() => {
                  const next = selectedOptionId() === option.option_id ? null : option.option_id;
                  setSelectedOptionId(next);
                  pushDraft({
                    selectedOptionId: next,
                    customText: customText(),
                  });
                }}
              >
                {option.label}
              </button>
            )}
          </For>
        </div>
      </Show>
      <textarea
        class="answer-field"
        value={customText()}
        onInput={(event) => {
          const next = event.currentTarget.value;
          setCustomText(next);
          pushDraft({
            selectedOptionId: selectedOptionId(),
            customText: next,
          });
        }}
        placeholder="Your answer"
      />
    </section>
  );
}

export default function SessionWorkspacePage() {
  const params = useParams();
  const navigate = useNavigate();
  const [session, { refetch: refetchSession }] = createResource(() => params.sessionId, getSession);
  const [promptBank, { refetch: refetchPromptBank }] = createResource(() => params.sessionId, getPromptBank);
  const [drafts, setDrafts] = createStore<Record<string, Record<string, DraftEntry>>>({});
  const [activeThreadId, setActiveThreadId] = createSignal<string | null>(null);
  const [socketState, setSocketState] = createSignal<"connecting" | "open" | "closed" | "error">("closed");
  const [submitError, setSubmitError] = createSignal<string | null>(null);
  const [actionNotice, setActionNotice] = createSignal<string | null>(null);
  const [actionError, setActionError] = createSignal<string | null>(null);
  const [actionPending, setActionPending] = createSignal<null | "duplicate" | "export" | "restart" | "retry">(null);
  const [submitting, setSubmitting] = createSignal(false);

  let socket: WebSocket | null = null;
  let workspaceScroll: HTMLDivElement | undefined;

  const bankedThreads = createMemo(() => promptBank()?.banked_threads ?? []);
  const queuedThreads = createMemo(() => promptBank()?.queued_threads ?? []);
  const selectedThread = createMemo(() => {
    const selectedId = activeThreadId();
    const available = bankedThreads();
    return available.find((thread) => thread.category_id === selectedId) ?? available[0] ?? null;
  });

  createEffect(() => {
    const bank = promptBank();
    if (!bank) return;
    const available = bank.banked_threads;
    if (available.length === 0) {
      setActiveThreadId(null);
      return;
    }

    const selectedId = activeThreadId();
    if (selectedId && available.some((thread) => thread.category_id === selectedId)) return;
    setActiveThreadId(bank.active_thread_id ?? available[0].category_id);
  });

  createEffect(() => {
    selectedThread();
    if (workspaceScroll) {
      workspaceScroll.scrollTop = 0;
    }
  });

  createEffect(() => {
    const current = session();
    const sessionId = params.sessionId;
    if (!current || !sessionId || current.session.intake_phase !== "interviewing") {
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

    socket.onopen = () => setSocketState("open");
    socket.onerror = () => setSocketState("error");
    socket.onclose = () => {
      setSocketState("closed");
      socket = null;
    };
    socket.onmessage = async (event) => {
      try {
        const payload = JSON.parse(event.data) as { type?: string };
        if (
          payload.type === "prompt" ||
          payload.type === "category_state" ||
          payload.type === "workspace_state" ||
          payload.type === "converged" ||
          payload.type === "planner_event"
        ) {
          await refetchSession();
          await refetchPromptBank();
          setSubmitting(false);
        }
      } catch {
        // Ignore malformed socket payloads; the next resource fetch remains authoritative.
      }
    };
  });

  onCleanup(() => {
    if (socket) socket.close();
  });

  const handleDraftChange = (promptId: string, itemId: string, next: DraftEntry) => {
    setDrafts(promptId, itemId, next);
  };

  const handleSubmit = async () => {
    const thread = selectedThread();
    if (!thread || !socket || socket.readyState !== WebSocket.OPEN) {
      setSubmitError("Live interview connection is not ready.");
      return;
    }

    setSubmitting(true);
    setSubmitError(null);

    const message: ClientPromptResponseMessage = {
      type: "prompt_response",
      prompt_id: thread.prompt.prompt_id,
      answers: buildPromptAnswers(thread.prompt, drafts[thread.prompt.prompt_id]),
      submitted_at: new Date().toISOString(),
      client_context: {
        viewport_class: viewportClass(),
      },
    };

    socket.send(JSON.stringify(message));
  };

  const clearActionFeedback = () => {
    setActionNotice(null);
    setActionError(null);
  };

  const handleDuplicate = async (currentSession: Session) => {
    setActionPending("duplicate");
    clearActionFeedback();
    try {
      const response = await duplicateSession(currentSession.id, {
        title: `${presentSessionTitle(currentSession)} copy`,
      });
      navigate(`/sessions/${response.session.id}`);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "Unable to duplicate the session.");
    } finally {
      setActionPending(null);
    }
  };

  const handleExport = async (currentSession: Session) => {
    setActionPending("export");
    clearActionFeedback();
    try {
      const response = await exportSession(currentSession.id);
      const blob = new Blob([JSON.stringify(response, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = buildSessionExportFilename(currentSession);
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

  const handleRestart = async (currentSession: Session) => {
    setActionPending("restart");
    clearActionFeedback();
    try {
      await restartSessionFromDescription(currentSession.id);
      await Promise.all([refetchSession(), refetchPromptBank()]);
      setDrafts({});
      setActionNotice("Session reset to the original description.");
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "Unable to restart from description.");
    } finally {
      setActionPending(null);
    }
  };

  const handleRetry = async (currentSession: Session) => {
    setActionPending("retry");
    clearActionFeedback();
    try {
      await retrySessionPipeline(currentSession.id);
      await Promise.all([refetchSession(), refetchPromptBank()]);
      setActionNotice("Pipeline retry started.");
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "Unable to retry the pipeline.");
    } finally {
      setActionPending(null);
    }
  };

  return (
    <section class="page">
      <Title>{session.latest ? presentSessionTitle(session.latest.session) : "Session"}</Title>
      <Show
        when={session.latest}
        fallback={
          <div class="loading-screen">
            <div class="loading-panel">
              <h1>Loading session…</h1>
              <p>Fetching the initial workspace snapshot.</p>
            </div>
          </div>
        }
      >
        {(sessionResponse) => {
          const currentSession = () => sessionResponse().session;
          const currentThread = () => selectedThread();

          return (
            <div class="shell-grid">
              <aside class="thread-pane">
                <div class="thread-scroll">
                  <div class="stack" style={{ padding: "18px 14px" }}>
                    <div class="eyebrow">Thread index</div>
                    <div class="panel-copy">
                      {bankedThreads().length} banked
                      {queuedThreads().length > 0 ? ` · ${queuedThreads().length} queued` : ""}
                    </div>
                  </div>
                  <div class="thread-list">
                    <For each={bankedThreads()}>
                      {(thread) => (
                        <button
                          class={`thread-row${currentThread()?.category_id === thread.category_id ? " is-active" : ""}`}
                          type="button"
                          onClick={() => setActiveThreadId(thread.category_id)}
                        >
                          <div class="thread-label">
                            <div class="thread-name">{thread.title}</div>
                            <div class="thread-summary">{thread.summary}</div>
                          </div>
                          <div class="thread-count">[{` ${thread.question_count}/${thread.question_count} `}]</div>
                        </button>
                      )}
                    </For>
                    <For each={queuedThreads()}>
                      {(thread) => (
                        <div class="thread-row is-queued">
                          <div class="thread-label">
                            <div class="thread-name">{thread.title}</div>
                            <div class="thread-summary">{thread.summary}</div>
                          </div>
                          <div class="thread-count">[ queued ]</div>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </aside>

              <section class="workspace-pane">
                <div class="workspace-scroll" ref={workspaceScroll}>
                  <div class="workspace-stack">
                    <div class="workspace-heading">
                      <div class="eyebrow">Socratic workspace</div>
                      <h1 class="workspace-title">{presentSessionTitle(currentSession())}</h1>
                      <div class="workspace-meta">
                        <span class="pill">{currentSession().intake_phase}</span>
                        <span class="pill">Socket {socketState()}</span>
                        <Show when={currentSession().current_step}>
                          <span>{currentSession().current_step}</span>
                        </Show>
                      </div>
                      <Show when={currentSession().project_description}>
                        <p class="workspace-summary">{currentSession().project_description}</p>
                      </Show>
                      <div class="session-action-row">
                        <Show when={currentSession().project_slug}>
                          {projectSlug => (
                            <>
                              <A class="btn btn-subtle" href={`/projects/${projectSlug()}`}>
                                Back to project
                              </A>
                              <A class="btn btn-subtle" href={`/projects/${projectSlug()}/import`}>
                                Project import
                              </A>
                            </>
                          )}
                        </Show>
                        <button
                          class="btn btn-subtle"
                          type="button"
                          disabled={actionPending() !== null}
                          onClick={() => void handleDuplicate(currentSession())}
                        >
                          {actionPending() === "duplicate" ? "Duplicating…" : "Duplicate"}
                        </button>
                        <button
                          class="btn btn-subtle"
                          type="button"
                          disabled={actionPending() !== null}
                          onClick={() => void handleExport(currentSession())}
                        >
                          {actionPending() === "export" ? "Exporting…" : "Export"}
                        </button>
                        <Show
                          when={currentSession().project_description?.trim()}
                        >
                          <button
                            class="btn btn-subtle"
                            type="button"
                            disabled={actionPending() !== null || currentSession().pipeline_running}
                            onClick={() => void handleRestart(currentSession())}
                          >
                            {actionPending() === "restart" ? "Restarting…" : "Restart from description"}
                          </button>
                        </Show>
                        <Show when={currentSession().intake_phase === "error" || !currentSession().pipeline_running}>
                          <button
                            class="btn btn-subtle"
                            type="button"
                            disabled={actionPending() !== null || !currentSession().project_description?.trim()}
                            onClick={() => void handleRetry(currentSession())}
                          >
                            {actionPending() === "retry" ? "Retrying…" : "Retry pipeline"}
                          </button>
                        </Show>
                      </div>
                      <Show when={actionNotice()}>
                        {notice => <div class="status-copy">{notice()}</div>}
                      </Show>
                      <Show when={actionError()}>
                        {message => <div class="error-copy">{message()}</div>}
                      </Show>
                    </div>

                    <Show
                      when={currentThread()}
                      fallback={
                        <div class="loading-panel">
                          <h1>Building the initial prompt bank…</h1>
                          <p>
                            This route waits for a fully materialized banked prompt instead of
                            pretending queued threads are already answerable.
                          </p>
                        </div>
                      }
                    >
                      {(threadSignal) => {
                        const thread = () => threadSignal();
                        return (
                          <>
                            <div class="workspace-heading">
                              <h2 class="workspace-title">{thread().title}</h2>
                              <p class="workspace-summary">{thread().summary}</p>
                              <div class="workspace-meta">
                                <span class="pill">{thread().prompt.kind}</span>
                                <span>{thread().question_count} questions</span>
                              </div>
                            </div>

                            <div class="question-list">
                              <For each={thread().prompt.items}>
                                {(item) => (
                                  <QuestionBlock
                                    promptId={thread().prompt.prompt_id}
                                    item={item}
                                    draft={drafts[thread().prompt.prompt_id]?.[item.item_id]}
                                    onDraftChange={(itemId, next) =>
                                      handleDraftChange(thread().prompt.prompt_id, itemId, next)
                                    }
                                  />
                                )}
                              </For>
                            </div>

                            {submitError() ? <div class="error-copy">{submitError()}</div> : null}

                            <div class="workspace-footer">
                              <div class="status-copy">
                                {queuedThreads().length > 0
                                  ? `${queuedThreads().length} queued threads will hydrate after the current prompt bank expands.`
                                  : "All currently known banked work is loaded locally."}
                              </div>
                              <button class="btn btn-primary" type="button" disabled={submitting()} onClick={handleSubmit}>
                                {submitting() ? "Submitting…" : "Submit answered items"}
                              </button>
                            </div>
                          </>
                        );
                      }}
                    </Show>
                  </div>
                </div>
              </section>
            </div>
          );
        }}
      </Show>
    </section>
  );
}
