import { Title } from "@solidjs/meta";
import { A } from "@solidjs/router";
import { For, Show, createSignal, onCleanup } from "solid-js";

import {
  countProcessedPromptItems,
  describePromptItemProjection,
  draftHasContent,
  firstUnprocessedPromptItemId,
  presentSessionTitle,
  type DraftEntry,
} from "~/lib/workspace";
import type { PromptItem } from "~/lib/types";
import {
  formatSavedLabel,
  getSessionReturnTarget,
} from "./session-workspace-view";
import type { SessionWorkspaceController } from "./session-workspace-controller";

function QuestionComposer(props: {
  item: PromptItem;
  itemIndex: number;
  itemCount: number;
  draft?: DraftEntry;
  saveStateLabel: string;
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

  return (
    <section class="session-interview-card">
      <div class="session-interview-card-head">
        <div class="session-interview-kicker">
          Question {props.itemIndex + 1}/{props.itemCount}
        </div>
        <div class="session-interview-save-state">{props.saveStateLabel}</div>
      </div>
      <p class="session-interview-question">{props.item.text}</p>
      <Show when={props.item.options.length > 0}>
        <div class="session-interview-options">
          <For each={props.item.options}>
            {(option, index) => (
              <button
                class={`session-option-chip${selectedOptionId() === option.option_id ? " is-selected" : ""}`}
                type="button"
                onClick={() => {
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
        class="session-interview-input"
        value={customText()}
        onKeyDown={event => {
          if (!(event.metaKey || event.ctrlKey) || event.key !== "Enter") return;
          event.preventDefault();
          event.stopPropagation();
          props.onCommit(flushDraftSync());
        }}
        onInput={event => {
          setCustomText(event.currentTarget.value);
          scheduleDraftSync();
        }}
        placeholder="Type your answer"
      />
      <div class="session-interview-actions">
        <div class="session-interview-hint">Press Cmd+Enter to commit and advance.</div>
        <button
          class="btn btn-primary session-commit-button"
          type="button"
          onClick={() => props.onCommit(flushDraftSync())}
        >
          Commit and next
        </button>
      </div>
    </section>
  );
}

export default function SessionWorkspaceScreen(props: { controller: SessionWorkspaceController }) {
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
          const committedAnswerCount = () =>
            props.controller.bankedThreads().reduce(
              (total, thread) => total + countProcessedPromptItems(thread.prompt, props.controller.processedByItemId()),
              0,
            );
          const totalPromptItemCount = () =>
            props.controller.bankedThreads().reduce((total, thread) => total + thread.prompt.items.length, 0);
          const liveSectionCount = () => props.controller.bankedThreads().length;
          const queuedSectionCount = () => props.controller.queuedThreads().length;

          return (
            <div class="session-artifact-route">
              <div class="session-artifact-topbar">
                <div class="session-artifact-topbar-primary">
                  <div class="session-artifact-topbar-context">
                    <A class="btn btn-subtle" href={returnTarget().href}>
                      {returnTarget().label}
                    </A>
                    <div class="eyebrow">Artifact-first workspace</div>
                  </div>
                  <div class="session-artifact-title-row">
                    <h1 class="session-artifact-title">{presentSessionTitle(currentSession())}</h1>
                    <Show when={sessionStatus()}>
                      {(status) => (
                        <div class="session-artifact-status-row">
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
                            <span class="session-artifact-status-copy">{status().detail}</span>
                          </Show>
                        </div>
                      )}
                    </Show>
                  </div>
                  <div class="session-artifact-summary-strip">
                    <div class="session-artifact-summary-pill">
                      <span class="session-artifact-summary-label">Committed</span>
                      <span>{committedAnswerCount()}/{totalPromptItemCount()} answers</span>
                    </div>
                    <div class="session-artifact-summary-pill">
                      <span class="session-artifact-summary-label">Live sections</span>
                      <span>{liveSectionCount()}</span>
                    </div>
                    <Show when={queuedSectionCount() > 0}>
                      <div class="session-artifact-summary-pill">
                        <span class="session-artifact-summary-label">Queued</span>
                        <span>{queuedSectionCount()}</span>
                      </div>
                    </Show>
                  </div>
                </div>

                <div class="session-artifact-topbar-actions">
                  <div class="session-action-group">
                    <div class="session-action-group-label">Workspace tools</div>
                    <div class="session-action-group-buttons">
                      <Show when={currentSession().project_slug}>
                        {(projectSlug) => (
                          <A class="btn btn-subtle" href={`/projects/${projectSlug()}/import`}>
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
                    </div>
                  </div>
                  <Show when={currentSession().can_restart_from_description || currentSession().can_retry_pipeline}>
                    <div class="session-action-group is-emphasis">
                      <div class="session-action-group-label">Recovery</div>
                      <div class="session-action-group-buttons">
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
                    </div>
                  </Show>
                </div>
              </div>

              <Show when={props.controller.actionNotice()}>
                {(notice) => <div class="status-copy session-inline-status">{notice()}</div>}
              </Show>
              <Show when={props.controller.actionError()}>
                {(message) => <div class="error-copy session-inline-status">{message()}</div>}
              </Show>
              <Show when={props.controller.submitError()}>
                {(message) => <div class="error-copy session-inline-status">{message()}</div>}
              </Show>

              <Show when={props.controller.isCollapsedLayout()}>
                <div class="session-surface-tabs" role="tablist" aria-label="Session surfaces">
                  <button
                    class={`session-surface-tab${props.controller.surfaceTab() === "interview" ? " is-active" : ""}`}
                    type="button"
                    role="tab"
                    aria-selected={props.controller.surfaceTab() === "interview"}
                    onClick={() => props.controller.handleSurfaceTabChange("interview")}
                  >
                    Interview
                  </button>
                  <button
                    class={`session-surface-tab${props.controller.surfaceTab() === "artifact" ? " is-active" : ""}`}
                    type="button"
                    role="tab"
                    aria-selected={props.controller.surfaceTab() === "artifact"}
                    onClick={() => props.controller.handleSurfaceTabChange("artifact")}
                  >
                    Artifact
                  </button>
                </div>
              </Show>

              <Show
                when={workspaceReady()}
                fallback={
                  <div class="loading-panel session-artifact-loading">
                    <h1>{sessionStatus()?.label ?? "Loading session"}</h1>
                    <p>{sessionStatus()?.detail ?? "Waiting for the next truthful workspace update."}</p>
                    <Show when={props.controller.needsSavedBriefAction()}>
                      <div class="button-row">
                        <A class="btn btn-primary" href="/sessions/new">
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
                <div class="session-artifact-shell">
                  <Show when={!props.controller.isCollapsedLayout() || props.controller.surfaceTab() === "interview"}>
                    <aside class="session-interview-pane">
                      <div class="session-interview-scroll">
                        <div class="session-interview-stack">
                          <div class="session-lane-head">
                            <div class="session-lane-kicker">Interview lane</div>
                            <p class="session-lane-copy">
                              Answer the current thread. Committed responses land in the blueprint immediately.
                            </p>
                          </div>

                          <Show when={props.controller.bankedThreads().length > 0}>
                            <div class="session-thread-switcher">
                              <For each={props.controller.bankedThreads()}>
                                {(thread) => (
                                  <button
                                    class={`session-thread-chip${currentThread()?.category_id === thread.category_id ? " is-active" : ""}`}
                                    type="button"
                                    onClick={() => props.controller.setActiveTask(
                                      thread.category_id,
                                      firstUnprocessedPromptItemId(thread.prompt, props.controller.processedByItemId())
                                        ?? thread.prompt.items[0]?.item_id
                                        ?? null,
                                    )}
                                  >
                                    <span>{thread.title}</span>
                                    <span class="session-thread-chip-count">
                                      {countProcessedPromptItems(thread.prompt, props.controller.processedByItemId())}/{thread.prompt.items.length}
                                    </span>
                                  </button>
                                )}
                              </For>
                            </div>
                          </Show>

                          <Show
                            when={currentThread()}
                            fallback={
                              <div class="loading-panel session-artifact-loading is-inline">
                                <h1>Build path ready</h1>
                                <p>
                                  {props.controller.promptBankGraph().buildReadinessMessage
                                    ?? "No remaining prompt threads are blocking the build handoff."}
                                </p>
                              </div>
                            }
                          >
                            {(thread) => (
                              <Show
                                when={currentQuestion()}
                                keyed
                                fallback={
                                  <div class="loading-panel session-artifact-loading is-inline">
                                    <h1>Build path ready</h1>
                                    <p>
                                      {props.controller.promptBankGraph().buildReadinessMessage
                                        ?? "No remaining prompt threads are blocking the build handoff."}
                                    </p>
                                  </div>
                                }
                              >
                                {(question) => (
                                  <>
                                    <div class="session-interview-head">
                                      <div class="session-interview-thread-title-row">
                                        <div>
                                          <div class="session-lane-kicker">Current thread</div>
                                          <h2 class="session-interview-thread-title">{thread().title}</h2>
                                        </div>
                                        <span class="session-interview-thread-progress">
                                          {props.controller.activeThreadProgress()}/{thread().prompt.items.length} committed
                                        </span>
                                      </div>
                                      <p class="session-interview-thread-summary">{thread().summary}</p>
                                    </div>

                                    <QuestionComposer
                                      item={question}
                                      itemIndex={props.controller.activeItemIndex()}
                                      itemCount={thread().prompt.items.length}
                                      draft={props.controller.draftsByQuestionId()[question.item_id]}
                                      saveStateLabel={formatSavedLabel(
                                        props.controller.draftSaveState(),
                                        props.controller.draftSaveMessage(),
                                      )}
                                      onDraftChange={(itemId, next) => props.controller.handleDraftChange(thread(), itemId, next)}
                                      onCommit={draft => void props.controller.handleCommitCurrentAnswer(draft)}
                                      inputRef={props.controller.registerInputRef}
                                    />

                                    <Show when={props.controller.upcomingTasks().length > 0 || props.controller.queuedThreads().length > 0}>
                                      <div class="session-up-next-panel">
                                        <div class="session-up-next-title">Up next</div>
                                        <div class="session-up-next-list">
                                          <For each={props.controller.upcomingTasks()}>
                                            {(task) => (
                                              <div class="session-up-next-row">
                                                <div class="session-up-next-thread">{task.threadTitle}</div>
                                                <div class="session-up-next-copy">{task.text}</div>
                                              </div>
                                            )}
                                          </For>
                                          <For each={props.controller.queuedThreads().slice(0, 2)}>
                                            {(queuedThread) => (
                                              <div class="session-up-next-row is-queued">
                                                <div class="session-up-next-thread">{queuedThread.title}</div>
                                                <div class="session-up-next-copy">{queuedThread.summary}</div>
                                              </div>
                                            )}
                                          </For>
                                        </div>
                                      </div>
                                    </Show>

                                    <Show when={props.controller.submittingThreadId() === thread().category_id}>
                                      <div class="status-copy session-inline-status is-inline">
                                        Continuing synthesis for {thread().title}…
                                      </div>
                                    </Show>
                                  </>
                                )}
                              </Show>
                            )}
                          </Show>
                        </div>
                      </div>
                    </aside>
                  </Show>

                  <Show when={!props.controller.isCollapsedLayout() || props.controller.surfaceTab() === "artifact"}>
                    <section class="session-artifact-pane">
                      <div class="session-artifact-scroll">
                        <div class="session-artifact-document">
                          <div class="session-artifact-document-head">
                            <div class="session-lane-kicker">Working blueprint</div>
                            <h2 class="session-artifact-document-title">{presentSessionTitle(currentSession())}</h2>
                            <p class="session-artifact-document-copy">
                              Committed answers land here immediately as working draft notes while synthesis catches up.
                            </p>
                            <div class="session-artifact-document-facts">
                              <div class="session-artifact-document-fact">
                                <span class="session-artifact-summary-label">Current shape</span>
                                <span>{liveSectionCount()} live sections</span>
                              </div>
                              <div class="session-artifact-document-fact">
                                <span class="session-artifact-summary-label">Captured so far</span>
                                <span>{committedAnswerCount()} committed answers</span>
                              </div>
                              <Show when={queuedSectionCount() > 0}>
                                <div class="session-artifact-document-fact">
                                  <span class="session-artifact-summary-label">Still queued</span>
                                  <span>{queuedSectionCount()} sections</span>
                                </div>
                              </Show>
                            </div>
                          </div>

                          <div class="session-artifact-sections">
                            <For each={props.controller.bankedThreads()}>
                              {(thread) => (
                                <section
                                  class={`session-artifact-section${currentThread()?.category_id === thread.category_id ? " is-active" : ""}`}
                                >
                                  <div class="session-artifact-section-head">
                                    <div>
                                      <div class="session-artifact-section-state-row">
                                        <span class="session-artifact-section-state">
                                          {currentThread()?.category_id === thread.category_id ? "Current section" : "Live section"}
                                        </span>
                                      </div>
                                      <h2 class="session-artifact-section-title">{thread.title}</h2>
                                      <p class="session-artifact-section-summary">{thread.summary}</p>
                                    </div>
                                    <div class="session-artifact-section-meta">
                                      {countProcessedPromptItems(thread.prompt, props.controller.processedByItemId())}/{thread.prompt.items.length} committed
                                    </div>
                                  </div>

                                  <div class="session-artifact-section-body">
                                    <For each={thread.prompt.items}>
                                      {(item) => {
                                        const projection = () =>
                                          describePromptItemProjection(item, props.controller.draftsByQuestionId()[item.item_id]);
                                        return (
                                          <article class={`session-artifact-question${currentQuestion()?.item_id === item.item_id ? " is-current" : ""}`}>
                                            <div class="session-artifact-question-label-row">
                                              <div class="session-artifact-question-label">Prompt anchor</div>
                                              <Show when={currentQuestion()?.item_id === item.item_id}>
                                                <span class="session-artifact-question-state">Current prompt</span>
                                              </Show>
                                            </div>
                                            <p class="session-artifact-question-copy">{item.text}</p>
                                            <Show
                                              when={projection().length > 0}
                                              fallback={
                                                <div class="session-artifact-placeholder">
                                                  <span class="session-artifact-placeholder-line" />
                                                  <span class="session-artifact-placeholder-line short" />
                                                </div>
                                              }
                                            >
                                              <div class="session-artifact-answer-label">Working draft note</div>
                                              <div class="session-artifact-draft-block">
                                                <For each={projection()}>
                                                  {(line) => <p>{line}</p>}
                                                </For>
                                              </div>
                                            </Show>
                                          </article>
                                        );
                                      }}
                                    </For>
                                  </div>
                                </section>
                              )}
                            </For>

                            <Show when={props.controller.queuedThreads().length > 0}>
                              <section class="session-artifact-section is-queued">
                                <div class="session-artifact-section-head">
                                  <div>
                                    <div class="session-artifact-section-state-row">
                                      <span class="session-artifact-section-state">Queued section</span>
                                    </div>
                                    <h2 class="session-artifact-section-title">Queued sections</h2>
                                    <p class="session-artifact-section-summary">
                                      These blueprint sections will fill in as more context becomes available.
                                    </p>
                                  </div>
                                </div>
                                <div class="session-artifact-queued-list">
                                  <For each={props.controller.queuedThreads()}>
                                    {(thread) => (
                                      <div class="session-artifact-queued-row">
                                        <div class="session-artifact-queued-title">{thread.title}</div>
                                        <div class="session-artifact-queued-summary">{thread.summary}</div>
                                      </div>
                                    )}
                                  </For>
                                </div>
                              </section>
                            </Show>
                          </div>
                        </div>
                      </div>
                    </section>
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
