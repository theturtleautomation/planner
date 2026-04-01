import { Title } from "@solidjs/meta";
import { A } from "@solidjs/router";
import { For, Show, createEffect, createSignal, onCleanup } from "solid-js";

import {
  countProcessedPromptItems,
  draftHasContent,
  firstUnprocessedPromptItemId,
  presentSessionTitle,
  type DraftEntry,
} from "~/lib/workspace";
import type { PromptItem } from "~/lib/types";
import { withFrontendMockSearch } from "~/lib/mock/runtime";
import {
  formatSavedLabel,
  getSessionReturnTarget,
} from "./session-workspace-view";
import type { SessionWorkspaceController } from "./session-workspace-controller";

function describeDraft(item: PromptItem, draft: DraftEntry | undefined, isProcessed: boolean): string {
  const customText = draft?.customText?.trim();
  if (customText) {
    return customText.length > 88 ? `${customText.slice(0, 88).trimEnd()}…` : customText;
  }

  if (draft?.selectedOptionId) {
    const selected = item.options.find(option => option.option_id === draft.selectedOptionId);
    if (selected) return selected.label;
  }

  return isProcessed ? "Committed" : "Select to answer";
}

function QuestionComposer(props: {
  item: PromptItem;
  itemIndex: number;
  itemCount: number;
  draft?: DraftEntry;
  isActive: boolean;
  isProcessed: boolean;
  saveStateLabel: string;
  onActivate: () => void;
  onDraftChange: (itemId: string, next: DraftEntry) => void;
  onCommit: (next: DraftEntry) => void;
  inputRef: (itemId: string, element: HTMLTextAreaElement) => void;
}) {
  const [selectedOptionId, setSelectedOptionId] = createSignal(props.draft?.selectedOptionId ?? null);
  const [customText, setCustomText] = createSignal(props.draft?.customText ?? "");
  let draftSyncTimer: number | undefined;

  const currentDraft = (): DraftEntry => ({
    selectedOptionId: selectedOptionId(),
    customText: customText(),
  });

  createEffect(() => {
    setSelectedOptionId(props.draft?.selectedOptionId ?? null);
    setCustomText(props.draft?.customText ?? "");
  });

  const scheduleDraftSync = () => {
    if (draftSyncTimer !== undefined) {
      window.clearTimeout(draftSyncTimer);
    }
    draftSyncTimer = window.setTimeout(() => {
      props.onDraftChange(props.item.item_id, currentDraft());
      draftSyncTimer = undefined;
    }, 200);
  };

  const flushDraftSync = () => {
    if (draftSyncTimer !== undefined) {
      window.clearTimeout(draftSyncTimer);
      draftSyncTimer = undefined;
    }
    const next = currentDraft();
    props.onDraftChange(props.item.item_id, next);
    return next;
  };

  onCleanup(() => {
    if (draftSyncTimer !== undefined) {
      window.clearTimeout(draftSyncTimer);
    }
  });

  const hasDraft = () => draftHasContent(props.draft);
  const preview = () => describeDraft(props.item, props.draft, props.isProcessed);

  return (
    <section
      class={`session-question-card${props.isActive ? " is-active" : ""}${props.isProcessed ? " is-processed" : ""}`}
      onClick={() => props.onActivate()}
    >
      <div class="session-question-card-head">
        <div class="session-question-card-title-row">
          <div class="session-question-kicker">
            Question {props.itemIndex + 1}/{props.itemCount}
          </div>
          <Show when={props.isActive}>
            <span class="session-question-current-badge">Current</span>
          </Show>
          <Show when={!props.isActive && props.isProcessed}>
            <span class="session-question-state-badge">Committed</span>
          </Show>
          <Show when={!props.isActive && !props.isProcessed && hasDraft()}>
            <span class="session-question-state-badge is-draft">Draft</span>
          </Show>
        </div>
        <Show when={props.isActive}>
          <div class="session-question-save-state">{props.saveStateLabel}</div>
        </Show>
      </div>
      <p class="session-question-copy">{props.item.text}</p>
      <Show
        when={props.isActive}
        fallback={<div class="session-question-preview">{preview()}</div>}
      >
        <Show when={props.item.options.length > 0}>
          <div class="session-question-options">
            <For each={props.item.options}>
              {(option, index) => (
                <button
                  class={`session-option-chip${selectedOptionId() === option.option_id ? " is-selected" : ""}`}
                  type="button"
                  onClick={() => {
                    props.onActivate();
                    const next = selectedOptionId() === option.option_id ? null : option.option_id;
                    setSelectedOptionId(next);
                    scheduleDraftSync();
                  }}
                >
                  <span class="session-option-chip-index">[{index() + 1}]</span>
                  {option.label}
                </button>
              )}
            </For>
          </div>
        </Show>
        <textarea
          ref={element => props.inputRef(props.item.item_id, element)}
          class="session-question-input"
          value={customText()}
          onFocus={() => props.onActivate()}
          onKeyDown={event => {
            if (!(event.metaKey || event.ctrlKey) || event.key !== "Enter") return;
            event.preventDefault();
            event.stopPropagation();
            props.onActivate();
            props.onCommit(flushDraftSync());
          }}
          onInput={event => {
            props.onActivate();
            setCustomText(event.currentTarget.value);
            scheduleDraftSync();
          }}
          placeholder="Type your answer"
        />
        <div class="session-question-actions">
          <div class="session-question-hint">Draft saves automatically. Press Cmd+Enter to commit and advance.</div>
          <button
            class="btn btn-primary session-commit-button"
            type="button"
            onClick={() => {
              props.onActivate();
              props.onCommit(flushDraftSync());
            }}
          >
            Commit and next
          </button>
        </div>
      </Show>
    </section>
  );
}

export default function SessionWorkspaceScreen(props: { controller: SessionWorkspaceController }) {
  let workAreaRef: HTMLElement | undefined;

  const jumpToThread = (threadId: string, itemId: string | null) => {
    props.controller.setActiveTask(threadId, itemId, false);
    workAreaRef?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  return (
    <section class="page">
      <Title>{props.controller.session()?.session ? presentSessionTitle(props.controller.session()!.session) : "Session"}</Title>
      <Show
        when={props.controller.session()}
        fallback={
          <div class="loading-screen">
            <div class="loading-panel">
              <h1>Loading session…</h1>
              <p>Fetching the initial workspace snapshot.</p>
            </div>
          </div>
        }
      >
        {sessionResponse => {
          const currentSession = () => sessionResponse().session;
          const currentThread = () => props.controller.selectedThread();
          const currentQuestion = () => props.controller.activeItem();
          const sessionStatus = () => props.controller.sessionStatus();
          const workspaceReady = () => props.controller.workspaceReady();
          const returnTarget = () => getSessionReturnTarget(currentSession());
          const activeThreadProgress = () => props.controller.activeThreadProgress();
          const committedAnswerCount = () =>
            props.controller.bankedThreads().reduce(
              (total, thread) => total + countProcessedPromptItems(thread.prompt, props.controller.processedByItemId()),
              0,
            );
          const totalPromptItemCount = () =>
            props.controller.bankedThreads().reduce((total, thread) => total + thread.prompt.items.length, 0);
          const liveThreadCount = () => props.controller.bankedThreads().length;
          const queuedThreadCount = () => props.controller.queuedThreads().length;

          return (
            <div class="session-question-route">
              <header class="session-question-header">
                <div class="session-question-header-main">
                  <div class="session-question-header-top">
                    <A class="btn btn-subtle" href={returnTarget().href}>
                      {returnTarget().label}
                    </A>
                    <div class="session-question-eyebrow">Question-bank workspace</div>
                  </div>
                  <h1 class="session-question-title">{presentSessionTitle(currentSession())}</h1>
                  <Show when={sessionStatus()}>
                    {(status) => (
                      <div class="session-question-status-row">
                        <span class={`state-badge${status().tone === "success"
                          ? " is-active"
                          : status().tone === "warning"
                            ? " is-attention"
                            : status().tone === "active"
                              ? " is-recent"
                              : " is-quiet"}`}
                        >
                          {status().label}
                        </span>
                        <Show when={status().detail}>
                          <span class="session-question-status-copy">{status().detail}</span>
                        </Show>
                      </div>
                    )}
                  </Show>
                  <div class="session-question-progress-line">
                    <span>{committedAnswerCount()} of {totalPromptItemCount()} answers committed</span>
                    <span>{liveThreadCount()} live {liveThreadCount() === 1 ? "thread" : "threads"}</span>
                    <Show when={queuedThreadCount() > 0}>
                      <span>{queuedThreadCount()} queued later</span>
                    </Show>
                  </div>
                </div>

                <details class="session-question-header-actions">
                  <summary class="btn btn-subtle session-question-actions-trigger">Session actions</summary>
                  <div class="session-question-actions-menu">
                    <Show when={currentSession().project_slug}>
                      {(projectSlug) => (
                        <A
                          class="btn btn-subtle"
                          href={withFrontendMockSearch(`/projects/${projectSlug()}/import`)}
                        >
                          Project import
                        </A>
                      )}
                    </Show>
                    <button
                      class="btn btn-subtle"
                      type="button"
                      disabled={props.controller.actionPending() !== null}
                      onClick={() => void props.controller.handleDuplicate(currentSession())}
                    >
                      {props.controller.actionPending() === "duplicate" ? "Duplicating…" : "Duplicate"}
                    </button>
                    <button
                      class="btn btn-subtle"
                      type="button"
                      disabled={props.controller.actionPending() !== null}
                      onClick={() => void props.controller.handleExport(currentSession())}
                    >
                      {props.controller.actionPending() === "export" ? "Exporting…" : "Export"}
                    </button>
                    <Show when={currentSession().can_restart_from_description}>
                      <button
                        class="btn btn-subtle"
                        type="button"
                        disabled={props.controller.actionPending() !== null}
                        onClick={() => void props.controller.handleRestart(currentSession())}
                      >
                        {props.controller.actionPending() === "restart"
                          ? currentSession().intake_phase === "error"
                            ? "Retrying…"
                            : "Restarting…"
                          : currentSession().intake_phase === "error"
                            ? "Retry startup"
                            : "Restart"}
                      </button>
                    </Show>
                    <Show when={currentSession().can_retry_pipeline}>
                      <button
                        class="btn btn-subtle"
                        type="button"
                        disabled={props.controller.actionPending() !== null}
                        onClick={() => void props.controller.handleRetry(currentSession())}
                      >
                        {props.controller.actionPending() === "retry" ? "Retrying…" : "Retry pipeline"}
                      </button>
                    </Show>
                  </div>
                </details>
              </header>

              <Show when={props.controller.actionNotice()}>
                {(notice) => <div class="status-copy session-inline-status">{notice()}</div>}
              </Show>
              <Show when={props.controller.actionError()}>
                {(message) => <div class="error-copy session-inline-status">{message()}</div>}
              </Show>
              <Show when={props.controller.submitError()}>
                {(message) => <div class="error-copy session-inline-status">{message()}</div>}
              </Show>

              <Show
                when={workspaceReady()}
                fallback={
                  <div class="loading-panel session-question-loading">
                    <h1>{sessionStatus()?.label ?? "Loading session"}</h1>
                    <p>{sessionStatus()?.detail ?? "Waiting for the next truthful workspace update."}</p>
                    <Show when={props.controller.needsSavedBriefAction()}>
                      <div class="button-row">
                        <A class="btn btn-primary" href={withFrontendMockSearch("/sessions/new")}>
                          Start a new session
                        </A>
                      </div>
                    </Show>
                    <Show when={props.controller.canRetryStartup()}>
                      <div class="button-row">
                        <button
                          class="btn btn-primary"
                          type="button"
                          disabled={props.controller.actionPending() !== null}
                          onClick={() => void props.controller.handleRetryStartup(currentSession())}
                        >
                          {props.controller.actionPending() === "restart" ? "Retrying…" : "Retry startup"}
                        </button>
                      </div>
                    </Show>
                  </div>
                }
              >
                <div class="session-question-shell">
                  <aside class="session-question-rail">
                    <div class="session-question-rail-head">
                      <div class="session-lane-kicker">Threads</div>
                      <div class="session-question-rail-count">{liveThreadCount()} live threads</div>
                    </div>
                    <div class="session-question-rail-list">
                      <For each={props.controller.bankedThreads()}>
                        {(thread) => (
                          <button
                            class={`session-thread-rail-button${currentThread()?.category_id === thread.category_id ? " is-active" : ""}`}
                            type="button"
                            onClick={() => jumpToThread(
                              thread.category_id,
                              firstUnprocessedPromptItemId(thread.prompt, props.controller.processedByItemId())
                                ?? thread.prompt.items[0]?.item_id
                                ?? null,
                            )}
                          >
                            <span class="session-thread-rail-title">{thread.title}</span>
                            <span class="session-thread-rail-progress">
                              {countProcessedPromptItems(thread.prompt, props.controller.processedByItemId())}/{thread.prompt.items.length} answered
                            </span>
                          </button>
                        )}
                      </For>
                    </div>
                    <Show when={props.controller.queuedThreads().length > 0}>
                      <details class="session-question-queued-disclosure">
                        <summary>
                          Queued later
                          <span>{props.controller.queuedThreads().length}</span>
                        </summary>
                        <div class="session-queued-list">
                          <For each={props.controller.queuedThreads()}>
                            {(thread) => (
                              <div class="session-queued-row">
                                <div class="session-queued-title">{thread.title}</div>
                                <div class="session-queued-summary">{thread.summary}</div>
                              </div>
                            )}
                          </For>
                        </div>
                      </details>
                    </Show>
                  </aside>

                  <Show
                    when={props.controller.bankedThreads().length > 0}
                    fallback={
                      <div class="loading-panel session-question-loading is-inline">
                        <h1>Build path ready</h1>
                        <p>
                          {props.controller.promptBankGraph().buildReadinessMessage
                            ?? "No remaining prompt threads are blocking the build handoff."}
                        </p>
                      </div>
                    }
                  >
                    <Show when={currentThread()}>
                      {(thread) => (
                        <article ref={workAreaRef} class="session-thread-workspace">
                          <div class="session-thread-workspace-head">
                            <div class="session-thread-workspace-copy">
                              <div class="session-thread-workspace-kicker">Active thread</div>
                              <h2 class="session-thread-section-title">{thread().title}</h2>
                              <p class="session-thread-section-summary">{thread().summary}</p>
                            </div>
                            <div class="session-thread-workspace-meta">
                              <span>{activeThreadProgress()} of {thread().prompt.items.length} answered</span>
                              <Show when={currentQuestion()}>
                                <span>Question {props.controller.activeItemIndex() + 1} of {thread().prompt.items.length}</span>
                              </Show>
                            </div>
                          </div>

                          <div class="session-thread-section-body">
                            <For each={thread().prompt.items}>
                              {(item, index) => (
                                <QuestionComposer
                                  item={item}
                                  itemIndex={index()}
                                  itemCount={thread().prompt.items.length}
                                  draft={props.controller.draftsByQuestionId()[item.item_id]}
                                  isActive={currentQuestion()?.item_id === item.item_id}
                                  isProcessed={!!props.controller.processedByItemId()[item.item_id]}
                                  saveStateLabel={formatSavedLabel(
                                    props.controller.draftSaveState(),
                                    props.controller.draftSaveMessage(),
                                  )}
                                  onActivate={() => props.controller.setActiveTask(thread().category_id, item.item_id, false)}
                                  onDraftChange={(itemId, next) => props.controller.handleDraftChange(thread(), itemId, next)}
                                  onCommit={draft => void props.controller.handleCommitAnswer(thread(), item.item_id, draft)}
                                  inputRef={props.controller.registerInputRef}
                                />
                              )}
                            </For>
                          </div>

                          <Show when={props.controller.submittingThreadId() === thread().category_id}>
                            <div class="status-copy session-inline-status is-inline">
                              Continuing synthesis for {thread().title}…
                            </div>
                          </Show>
                        </article>
                      )}
                    </Show>
                  </Show>
                </div>
              </Show>
            </div>
          );
        }}
      </Show>
    </section>
  );
}
