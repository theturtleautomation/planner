import { Title } from "@solidjs/meta";
import { A } from "@solidjs/router";
import { For, Show, createEffect, createMemo, createSignal, onCleanup } from "solid-js";

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
  areaWorkspacePressurePoints,
  deriveAreaShapingObjects,
  deriveProjectPictureAreas,
  formatSavedLabel,
  type ProjectPictureArea,
  type ProjectPictureAreaId,
  previewProjectPictureArea,
  selectRecommendedProjectArea,
  shouldShowQuestionSaveState,
  getSessionReturnTarget,
} from "./session-workspace-view";
import type { SessionWorkspaceController } from "./session-workspace-controller";
import type { DraftSaveState } from "./session-workspace-view";

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

function areaStateTone(state: ProjectPictureArea["state"]): string {
  switch (state) {
    case "defined":
      return "is-active";
    case "conflicted":
      return "is-attention";
    case "incomplete":
      return "is-recent";
    case "unclear":
    default:
      return "is-quiet";
  }
}

type AreaSurfaceMode = "preview" | "shape" | "discuss";

type AreaSeed = {
  id: string;
  text: string;
  readyToResurface: boolean;
};

function QuestionComposer(props: {
  item: PromptItem;
  itemIndex: number;
  itemCount: number;
  draft?: DraftEntry;
  isActive: boolean;
  isProcessed: boolean;
  saveStateLabel: string;
  saveStateState: DraftSaveState;
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
        <div class="session-question-kicker">
          Question {props.itemIndex + 1}/{props.itemCount}
        </div>
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
          <Show when={shouldShowQuestionSaveState(props.saveStateState)}>
            <div
              class={`session-question-save-state${props.saveStateState === "error" ? " is-error" : ""}`}
              role={props.saveStateState === "error" ? "status" : undefined}
            >
              {props.saveStateLabel}
            </div>
          </Show>
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

function ProjectAreaCard(props: {
  area: ProjectPictureArea;
  isActive: boolean;
  isRecommended: boolean;
  isRecentlyUpdated: boolean;
}) {
  return (
    <article class={`session-project-area-card${props.isActive ? " is-active" : ""}${props.isRecommended ? " is-recommended" : ""}`}>
      <div class="session-project-area-head">
        <div class="session-project-area-title-row">
          <span class="session-project-area-title">{props.area.title}</span>
          <span class={`state-badge ${areaStateTone(props.area.state)}`}>{props.area.state}</span>
        </div>
        <Show when={props.isRecentlyUpdated}>
          <span class="session-project-area-freshness">Updated</span>
        </Show>
      </div>
      <p class="session-project-area-summary">{props.area.summary}</p>
      <Show when={props.area.relationshipLabels.length > 0}>
        <div class="session-project-area-relations">
          <For each={props.area.relationshipLabels}>
            {(label) => <span class="session-project-area-relation">{label}</span>}
          </For>
        </div>
      </Show>
      <div class="session-project-area-meta">
        <span>{props.area.pressurePoints.length} pressure point{props.area.pressurePoints.length === 1 ? "" : "s"}</span>
        <Show when={props.area.pendingRevisions.length > 0}>
          <span>{props.area.pendingRevisions.length} pending revision{props.area.pendingRevisions.length === 1 ? "" : "s"}</span>
        </Show>
      </div>
    </article>
  );
}

function AreaShapingObjectCard(props: {
  kind: "label" | "claim" | "constraint";
  title: string;
  helper: string;
  value: string;
  editing: boolean;
  onStartEdit: () => void;
  onChange: (value: string) => void;
  onSave: () => void;
  onCancel: () => void;
}) {
  return (
    <section class={`session-area-shaping-object${props.editing ? " is-editing" : ""}`}>
      <div class="session-area-shaping-object-head">
        <div>
          <div class="session-question-kicker">{props.title}</div>
          <p class="session-project-support-copy">{props.helper}</p>
        </div>
        <Show
          when={props.editing}
          fallback={(
            <button class="btn btn-subtle" type="button" onClick={props.onStartEdit}>
              Edit inline
            </button>
          )}
        >
          <div class="session-area-shaping-object-actions">
            <button class="btn btn-subtle" type="button" onClick={props.onCancel}>
              Cancel
            </button>
            <button class="btn btn-primary" type="button" onClick={props.onSave}>
              Save
            </button>
          </div>
        </Show>
      </div>
      <Show
        when={props.editing}
        fallback={<div class="session-area-shaping-object-value">{props.value}</div>}
      >
        <Show
          when={props.kind === "claim"}
          fallback={(
            <input
              class="session-area-shaping-object-input"
              value={props.value}
              onInput={event => props.onChange(event.currentTarget.value)}
            />
          )}
        >
          <textarea
            class="session-area-shaping-object-input is-multiline"
            value={props.value}
            onInput={event => props.onChange(event.currentTarget.value)}
          />
        </Show>
      </Show>
    </section>
  );
}

function ResurfacedSeedCard(props: {
  areaTitle: string;
  seed: AreaSeed;
  onPromote: () => void;
  onDismiss: () => void;
}) {
  return (
    <section class="session-area-seed-card">
      <div class="session-question-kicker">Resurfaced seed</div>
      <p class="session-project-support-copy">
        This unresolved hunch stayed quiet until {props.areaTitle} needed attention again.
      </p>
      <div class="session-area-seed-text">{props.seed.text}</div>
      <div class="session-area-seed-actions">
        <button class="btn btn-primary" type="button" onClick={props.onPromote}>
          Promote into active work
        </button>
        <button class="btn btn-subtle" type="button" onClick={props.onDismiss}>
          Dismiss for now
        </button>
      </div>
    </section>
  );
}

function ThreadRail(props: {
  currentThreadId: string | undefined;
  liveThreadCount: number;
  bankedThreads: SessionWorkspaceController["bankedThreads"];
  queuedThreads: SessionWorkspaceController["queuedThreads"];
  processedByItemId: SessionWorkspaceController["processedByItemId"];
  onSelectThread: (threadId: string, itemId: string | null) => void;
}) {
  return (
    <>
      <div class="session-question-rail-head">
        <div class="session-lane-kicker">Threads</div>
        <div class="session-question-rail-count">{props.liveThreadCount} live threads</div>
      </div>
      <div class="session-question-rail-list">
        <For each={props.bankedThreads()}>
          {(thread) => (
            <button
              class={`session-thread-rail-button${props.currentThreadId === thread.category_id ? " is-active" : ""}`}
              type="button"
              onClick={() => props.onSelectThread(
                thread.category_id,
                firstUnprocessedPromptItemId(thread.prompt, props.processedByItemId())
                  ?? thread.prompt.items[0]?.item_id
                  ?? null,
              )}
            >
              <span class="session-thread-rail-title">{thread.title}</span>
              <span class="session-thread-rail-progress">
                {countProcessedPromptItems(thread.prompt, props.processedByItemId())}/{thread.prompt.items.length} answered
              </span>
            </button>
          )}
        </For>
      </div>
      <Show when={props.queuedThreads().length > 0}>
        <details class="session-question-queued-disclosure">
          <summary>
            Queued later
            <span>{props.queuedThreads().length}</span>
          </summary>
          <div class="session-queued-list">
            <For each={props.queuedThreads()}>
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
    </>
  );
}

function InsightRail(props: {
  currentThread: NonNullable<ReturnType<SessionWorkspaceController["selectedThread"]>>;
  currentQuestion: ReturnType<SessionWorkspaceController["activeItem"]>;
  activeThreadProgress: number;
  upcomingTasks: ReturnType<SessionWorkspaceController["upcomingTasks"]>;
  queuedCount: number;
  liveThreadCount: number;
  socketState: ReturnType<SessionWorkspaceController["socketState"]>;
  sessionStatus: NonNullable<ReturnType<SessionWorkspaceController["sessionStatus"]>>;
  draftSaveState: ReturnType<SessionWorkspaceController["draftSaveState"]>;
  draftSaveMessage: ReturnType<SessionWorkspaceController["draftSaveMessage"]>;
  submittingThreadId: ReturnType<SessionWorkspaceController["submittingThreadId"]>;
  buildReady: boolean;
  buildReadinessMessage?: string | null;
}) {
  const liveStatusLabel = () => {
    if (props.submittingThreadId === props.currentThread.category_id) {
      return `Synthesizing ${props.currentThread.title}`;
    }
    if (props.socketState === "open") return "Live updates connected";
    if (props.socketState === "connecting") return "Connecting live updates";
    if (props.socketState === "error") return "Live updates need attention";
    return "Working from local state";
  };

  const draftStateLabel = () => {
    if (props.draftSaveState === "error") {
      return props.draftSaveMessage ?? "Draft save failed";
    }
    if (props.draftSaveState === "saved") {
      return props.draftSaveMessage ?? "Drafts are saved";
    }
    if (props.draftSaveState === "saving") {
      return "Draft save in progress";
    }
    return "Drafts sync as you work";
  };

  return (
    <aside class="session-question-insight-rail">
      <section class="session-question-insight-panel">
        <div class="session-question-insight-kicker">Live state</div>
        <div class="session-question-insight-title">{liveStatusLabel()}</div>
        <p class="session-question-insight-copy">
          {props.sessionStatus.detail
            ?? "The active thread stays local while the session continues to refine around it."}
        </p>
        <dl class="session-question-insight-stats">
          <div>
            <dt>Active progress</dt>
            <dd>{props.activeThreadProgress}/{props.currentThread.prompt.items.length}</dd>
          </div>
          <div>
            <dt>Live threads</dt>
            <dd>{props.liveThreadCount}</dd>
          </div>
          <div>
            <dt>Queued later</dt>
            <dd>{props.queuedCount}</dd>
          </div>
        </dl>
      </section>

      <section class="session-question-insight-panel">
        <div class="session-question-insight-kicker">Current focus</div>
        <div class="session-question-insight-title">{props.currentThread.title}</div>
        <p class="session-question-insight-copy">
          {props.currentQuestion
            ? `Question ${props.activeThreadProgress + 1} is live now: ${props.currentQuestion.text}`
            : "No active question is selected in this thread."}
        </p>
        <div class={`session-question-insight-chip${props.draftSaveState === "error" ? " is-error" : ""}`}>
          {draftStateLabel()}
        </div>
      </section>

      <section class="session-question-insight-panel">
        <div class="session-question-insight-kicker">Up next</div>
        <Show
          when={props.upcomingTasks.length > 0}
          fallback={<p class="session-question-insight-copy">No additional banked questions are waiting after the current focus.</p>}
        >
          <div class="session-question-insight-list">
            <For each={props.upcomingTasks}>
              {(task) => (
                <div class="session-question-insight-row">
                  <div class="session-question-insight-row-title">{task.threadTitle}</div>
                  <div class="session-question-insight-row-copy">{task.text}</div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </section>

      <Show when={props.buildReady || props.buildReadinessMessage}>
        <section class="session-question-insight-panel is-accent">
          <div class="session-question-insight-kicker">Build readiness</div>
          <div class="session-question-insight-title">
            {props.buildReady ? "Path is clear" : "Waiting on the route state"}
          </div>
          <p class="session-question-insight-copy">
            {props.buildReadinessMessage
              ?? "No additional prompt-bank blockers are reported from the current workspace snapshot."}
          </p>
        </section>
      </Show>
    </aside>
  );
}

export default function SessionWorkspaceScreen(props: { controller: SessionWorkspaceController }) {
  let areaWorkspaceRef: HTMLElement | undefined;
  let areaWorkspaceHeadingRef: HTMLHeadingElement | undefined;
  let previousAreaContextKey: string | null = null;
  const areaCaptureInputRefs: Partial<Record<ProjectPictureAreaId, HTMLTextAreaElement>> = {};
  const [globalCaptureText, setGlobalCaptureText] = createSignal("");
  const [globalCaptures, setGlobalCaptures] = createSignal<string[]>([]);
  const [areaCaptureTextById, setAreaCaptureTextById] = createSignal<Record<string, string>>({});
  const [areaCapturesById, setAreaCapturesById] = createSignal<Record<string, string[]>>({});
  const [areaSeedsById, setAreaSeedsById] = createSignal<Record<string, AreaSeed[]>>({});
  const [recentAreaUpdates, setRecentAreaUpdates] = createSignal<Record<string, number>>({});
  const [areaSurface, setAreaSurface] = createSignal<AreaSurfaceMode>("preview");
  const [focusedAreaId, setFocusedAreaId] = createSignal<ProjectPictureAreaId | null>(null);
  const [focusedPressureThreadId, setFocusedPressureThreadId] = createSignal<string | null>(null);
  const [discussComposerOpen, setDiscussComposerOpen] = createSignal(false);
  const [areaObjectDraftsByAreaId, setAreaObjectDraftsByAreaId] = createSignal<Record<string, Partial<Record<"label" | "claim" | "constraint", string>>>>({});
  const [editingObjectKey, setEditingObjectKey] = createSignal<string | null>(null);
  let previousAreaSignatures: Record<string, string> | null = null;

  const projectAreas = createMemo(() => deriveProjectPictureAreas(
    props.controller.currentSession(),
    props.controller.bankedThreads(),
    props.controller.queuedThreads(),
    props.controller.processedByItemId(),
  ));
  const recommendedArea = () => selectRecommendedProjectArea(projectAreas());
  const selectedAreaFromThread = () => {
    const currentThread = props.controller.selectedThread();
    if (currentThread) {
      const areaFromThread = projectAreas().find(area =>
        area.pressurePoints.some(point => point.threadId === currentThread.category_id),
      );
      if (areaFromThread) return areaFromThread;
    }
    return recommendedArea();
  };
  const activeArea = () => {
    if (areaSurface() === "preview") return recommendedArea();
    const areaId = focusedAreaId();
    if (areaId) {
      return projectAreas().find(area => area.id === areaId)
        ?? selectedAreaFromThread()
        ?? recommendedArea();
    }
    return selectedAreaFromThread();
  };
  const activeAreaKey = createMemo(() => activeArea()?.id ?? null);
  const recommendedAreaKey = createMemo(() => recommendedArea()?.id ?? null);
  const activeAreaPreview = createMemo(() => previewProjectPictureArea(activeArea()));
  const dominantPreviewPoint = createMemo(() => activeAreaPreview().dominant);
  const secondaryPreviewPoints = createMemo(() => activeAreaPreview().secondary);
  const areaPressurePoints = createMemo(() => areaWorkspacePressurePoints(activeArea()));
  const dominantAreaPressurePoint = createMemo(() => areaPressurePoints()[0] ?? null);
  const secondaryAreaPressurePoints = createMemo(() => areaPressurePoints().slice(1));
  const focusedAreaPressurePoint = createMemo(() => (
    areaPressurePoints().find(point => point.threadId === focusedPressureThreadId())
    ?? dominantAreaPressurePoint()
  ));
  const activeAreaContextKey = createMemo(() => (
    areaSurface() === "preview"
      ? "preview"
      : `${areaSurface()}:${activeArea()?.id ?? "none"}`
  ));
  const resurfacedSeed = createMemo(() => {
    const areaId = activeArea()?.id;
    if (!areaId || areaSurface() === "preview") return null;
    return [...(areaSeedsById()[areaId] ?? [])]
      .reverse()
      .find(seed => seed.readyToResurface)
      ?? null;
  });
  const shapingObjects = createMemo(() => deriveAreaShapingObjects(activeArea()).map(object => ({
    ...object,
    sourceValue: object.value,
    value: areaObjectDraftsByAreaId()[activeArea()?.id ?? ""]?.[object.kind] ?? object.value,
  })));

  createEffect(() => {
    const signatures = Object.fromEntries(projectAreas().map(area => [area.id, area.signature]));
    if (previousAreaSignatures) {
      const changedAreaIds = Object.entries(signatures)
        .filter(([areaId, signature]) => previousAreaSignatures?.[areaId] !== signature)
        .map(([areaId]) => areaId);
      if (changedAreaIds.length > 0) {
        const updatedAt = Date.now();
        setRecentAreaUpdates(previous => {
          const next = { ...previous };
          for (const areaId of changedAreaIds) {
            next[areaId] = updatedAt;
          }
          return next;
        });
      }
    }
    previousAreaSignatures = signatures;
  });

  const commitGlobalCapture = () => {
    const next = globalCaptureText().trim();
    if (!next) return;
    setGlobalCaptures(previous => [next, ...previous].slice(0, 3));
    setGlobalCaptureText("");
  };

  const updateAreaCaptureText = (areaId: string, value: string) => {
    setAreaCaptureTextById(previous => ({
      ...previous,
      [areaId]: value,
    }));
  };

  const commitAreaCapture = (areaId: string) => {
    const next = areaCaptureTextById()[areaId]?.trim();
    if (!next) return;
    setAreaCapturesById(previous => ({
      ...previous,
      [areaId]: [next, ...(previous[areaId] ?? [])].slice(0, 3),
    }));
    setAreaCaptureTextById(previous => ({
      ...previous,
      [areaId]: "",
    }));
  };

  const commitAreaSeed = (areaId: ProjectPictureAreaId) => {
    const next = areaCaptureTextById()[areaId]?.trim();
    if (!next) return;
    setAreaSeedsById(previous => ({
      ...previous,
      [areaId]: [
        {
          id: `seed-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
          text: next,
          readyToResurface: false,
        },
        ...(previous[areaId] ?? []),
      ].slice(0, 3),
    }));
    setAreaCaptureTextById(previous => ({
      ...previous,
      [areaId]: "",
    }));
  };

  const updateSeed = (areaId: ProjectPictureAreaId, seedId: string, updater: (seed: AreaSeed) => AreaSeed | null) => {
    setAreaSeedsById(previous => {
      const nextSeeds = (previous[areaId] ?? [])
        .map(seed => seed.id === seedId ? updater(seed) : seed)
        .filter((seed): seed is AreaSeed => seed !== null);
      return {
        ...previous,
        [areaId]: nextSeeds,
      };
    });
  };

  const dismissSeedForNow = (areaId: ProjectPictureAreaId, seedId: string) => {
    updateSeed(areaId, seedId, seed => ({
      ...seed,
      readyToResurface: false,
    }));
  };

  const promoteSeedIntoAreaWork = (areaId: ProjectPictureAreaId, seedId: string) => {
    const seed = (areaSeedsById()[areaId] ?? []).find(candidate => candidate.id === seedId);
    if (!seed) return;
    setAreaCaptureTextById(previous => ({
      ...previous,
      [areaId]: seed.text,
    }));
    updateSeed(areaId, seedId, () => null);
    queueMicrotask(() => areaCaptureInputRefs[areaId]?.focus());
  };

  const isAreaFresh = (areaId: string) => {
    const updatedAt = recentAreaUpdates()[areaId];
    return updatedAt !== undefined && Date.now() - updatedAt < 30_000;
  };

  const enterShapingMode = (areaId: ProjectPictureAreaId, threadId: string | null) => {
    setFocusedAreaId(areaId);
    setFocusedPressureThreadId(threadId);
    setDiscussComposerOpen(false);
    setAreaSurface("shape");
  };

  const enterDiscussMode = (areaId: ProjectPictureAreaId, threadId: string | null) => {
    setFocusedAreaId(areaId);
    setFocusedPressureThreadId(threadId);
    setDiscussComposerOpen(false);
    setAreaSurface("discuss");

    if (!threadId) return;
    const thread = props.controller.bankedThreads().find(candidate => candidate.category_id === threadId);
    props.controller.setActiveTask(
      threadId,
      thread
        ? firstUnprocessedPromptItemId(thread.prompt, props.controller.processedByItemId())
          ?? thread.prompt.items[0]?.item_id
          ?? null
        : null,
      false,
    );
  };

  const returnToShapingMode = () => {
    setDiscussComposerOpen(false);
    setAreaSurface("shape");
  };

  const openDiscussComposer = () => {
    setDiscussComposerOpen(true);
  };

  const updateShapingObjectDraft = (areaId: ProjectPictureAreaId, kind: "label" | "claim" | "constraint", value: string) => {
    setAreaObjectDraftsByAreaId(previous => ({
      ...previous,
      [areaId]: {
        ...(previous[areaId] ?? {}),
        [kind]: value,
      },
    }));
  };

  const startEditingObject = (areaId: ProjectPictureAreaId, kind: "label" | "claim" | "constraint", fallbackValue: string) => {
    updateShapingObjectDraft(areaId, kind, areaObjectDraftsByAreaId()[areaId]?.[kind] ?? fallbackValue);
    setEditingObjectKey(`${areaId}:${kind}`);
  };

  const cancelEditingObject = (areaId: ProjectPictureAreaId, kind: "label" | "claim" | "constraint", fallbackValue: string) => {
    updateShapingObjectDraft(areaId, kind, fallbackValue);
    setEditingObjectKey(null);
  };

  const saveEditingObject = () => {
    setEditingObjectKey(null);
  };

  createEffect(() => {
    const currentContextKey = activeAreaContextKey();
    const previousContext = previousAreaContextKey;
    if (previousContext && previousContext !== currentContextKey) {
      const [previousSurface, previousAreaId] = previousContext.split(":");
      if ((previousSurface === "shape" || previousSurface === "discuss") && previousAreaId) {
        setAreaSeedsById(previous => ({
          ...previous,
          [previousAreaId]: (previous[previousAreaId] ?? []).map(seed => ({
            ...seed,
            readyToResurface: true,
          })),
        }));
      }
    }
    previousAreaContextKey = currentContextKey;
  });

  const renderCompactProjectSupport = () => (
    <details class="session-area-support-disclosure">
      <summary>Workspace support</summary>
      <div class="session-area-support-body">
        <section class="session-area-support-block">
          <div class="session-question-kicker">Project-wide capture</div>
          <p class="session-project-support-copy">
            Capture a thought that belongs to the whole project, not just this area.
          </p>
          <textarea
            class="session-project-capture-input"
            value={globalCaptureText()}
            onInput={(event) => setGlobalCaptureText(event.currentTarget.value)}
            placeholder="Capture an idea for the current project"
          />
          <div class="session-project-capture-actions">
            <button class="btn btn-primary" type="button" onClick={commitGlobalCapture}>
              Save local note
            </button>
          </div>
          <Show when={globalCaptures().length > 0}>
            <div class="session-project-capture-list">
              <For each={globalCaptures()}>
                {(note) => <div class="session-project-capture-note">{note}</div>}
              </For>
            </div>
          </Show>
        </section>

        <Show when={props.controller.promptBankGraph().buildReady || props.controller.promptBankGraph().buildReadinessMessage}>
          <section class="session-area-support-block is-muted">
            <div class="session-question-kicker">Build readiness</div>
            <div class="session-project-support-title">
              {props.controller.promptBankGraph().buildReady ? "Path is clear" : "Still converging"}
            </div>
            <p class="session-project-support-copy">
              {props.controller.promptBankGraph().buildReadinessMessage
                ?? "No additional prompt-bank blockers are reported from the current workspace snapshot."}
            </p>
          </section>
        </Show>
      </div>
    </details>
  );

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
          const sessionStatus = () => props.controller.sessionStatus();
          const workspaceReady = () => props.controller.workspaceReady();
          const returnTarget = () => getSessionReturnTarget(currentSession());
          const currentThread = () => props.controller.selectedThread();
          const currentQuestion = () => props.controller.activeItem();
          const activeAreaThread = () => {
            if (areaSurface() !== "discuss") return null;
            const area = activeArea();
            const thread = currentThread();
            if (!area || !thread) return null;
            return area.pressurePoints.some(point => point.threadId === thread.category_id) ? thread : null;
          };
          const currentThreadProgress = () => {
            const thread = activeAreaThread();
            return thread ? countProcessedPromptItems(thread.prompt, props.controller.processedByItemId()) : 0;
          };
          const routeFeedback = () => {
            const submitError = props.controller.submitError();
            if (submitError) return { tone: "error" as const, message: submitError };

            const actionError = props.controller.actionError();
            if (actionError) return { tone: "error" as const, message: actionError };

            const actionNotice = props.controller.actionNotice();
            if (actionNotice) return { tone: "notice" as const, message: actionNotice };

            return null;
          };

          return (
            <div class="session-question-route">
              <header class="session-question-header">
                <div class="session-question-header-main">
                  <div class="session-question-header-top">
                    <A class="btn btn-subtle" href={returnTarget().href}>
                      {returnTarget().label}
                    </A>
                    <div class="session-question-eyebrow">Project picture workspace</div>
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
                </div>

                <details class="session-question-header-actions">
                  <summary class="session-question-actions-trigger">Actions</summary>
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

              <Show when={routeFeedback()}>
                {(feedback) => (
                  <div class={`session-question-feedback${feedback().tone === "error" ? " is-error" : ""}`}>
                    {feedback().message}
                  </div>
                )}
              </Show>

              <Show
                when={workspaceReady()}
                fallback={
                  <div class="loading-panel session-question-loading">
                    <h1>{sessionStatus()?.label ?? "Loading session"}</h1>
                    <p>{sessionStatus()?.detail ?? "Waiting for the next truthful workspace update."}</p>
                    <Show when={props.controller.needsSavedBriefAction()}>
                      <div class="button-row">
                        <A class="btn btn-primary" href={withFrontendMockSearch("/projects/new")}>
                          Start a new project
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
                <div class={`session-project-shell${areaSurface() === "preview" ? "" : " is-focus-mode"}`}>
                  <div class="session-project-primary">
                    <section class="session-project-identity panel">
                      <div class="panel-head">
                        <div>
                          <div class="eyebrow">Project</div>
                          <h2 class="session-project-identity-title">{presentSessionTitle(currentSession())}</h2>
                        </div>
                        <Show when={sessionStatus()}>
                          {(status) => (
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
                          )}
                        </Show>
                      </div>
                      <div class="session-project-identity-body">
                        <p class="page-copy">
                          {currentSession().project_description?.trim()
                            ?? "Darkfactory is still converging the project shape around the saved brief."}
                        </p>
                      </div>
                    </section>

                    <section class="session-project-picture panel">
                      <div class="panel-head">
                        <div>
                          <div class="eyebrow">Project picture</div>
                          <h2 class="session-project-picture-title">Current project shape</h2>
                        </div>
                      </div>
                      <div class="session-project-area-grid">
                        <For each={projectAreas()}>
                          {(area) => (
                            <ProjectAreaCard
                              area={area}
                              isActive={activeAreaKey() === area.id}
                              isRecommended={recommendedAreaKey() === area.id}
                              isRecentlyUpdated={isAreaFresh(area.id)}
                            />
                          )}
                        </For>
                      </div>
                    </section>

                    <Show when={activeArea()}>
                      {(area) => (
                        <section
                          ref={areaWorkspaceRef}
                          class={`session-area-workspace panel${areaSurface() === "preview" ? " is-preview" : areaSurface() === "shape" ? " is-shape" : " is-discuss"}`}
                        >
                          <div class="panel-head">
                            <div>
                              <div class="eyebrow">
                                {areaSurface() === "preview"
                                  ? "Recommended area"
                                  : areaSurface() === "shape"
                                    ? "Area workspace"
                                    : "Area discussion"}
                              </div>
                              <h2
                                ref={areaWorkspaceHeadingRef}
                                class="session-area-workspace-title"
                                tabindex="-1"
                              >
                                {area().title}
                              </h2>
                            </div>
                            <div class="session-area-workspace-meta">
                              <Show when={areaSurface() === "preview"}>
                                <span class="state-badge is-active">start here</span>
                              </Show>
                              <Show when={areaSurface() === "shape"}>
                                <span class="state-badge is-recent">object-first</span>
                              </Show>
                              <Show when={areaSurface() === "discuss"}>
                                <span class="state-badge is-attention">discuss</span>
                              </Show>
                              <span class={`state-badge ${areaStateTone(area().state)}`}>{area().state}</span>
                              <Show when={isAreaFresh(area().id)}>
                                <span class="session-project-area-freshness">Updated</span>
                              </Show>
                            </div>
                          </div>
                          <div class="session-area-workspace-body">
                            <p class="session-area-workspace-summary">{area().summary}</p>
                            <Show when={area().relationshipLabels.length > 0}>
                              <div class="session-project-area-relations">
                                <For each={area().relationshipLabels}>
                                  {(label) => <span class="session-project-area-relation">{label}</span>}
                                </For>
                              </div>
                            </Show>
                            <Show
                              when={areaSurface() === "preview"}
                              fallback={
                                <Show
                                  when={areaSurface() === "shape"}
                                  fallback={
                                    <>
                                      <Show when={activeArea()?.pendingRevisions.length}>
                                        <section class="session-area-revisions session-area-revisions-compact">
                                          <div class="session-question-kicker">Pending revisions still in context</div>
                                          <div class="session-area-revision-list">
                                            <For each={activeArea()?.pendingRevisions ?? []}>
                                              {(revision) => (
                                                <div class={`session-area-revision-card${revision.conflict ? " is-conflict" : ""}`}>
                                                  <div class="session-area-revision-title-row">
                                                    <div class="session-area-revision-copy">
                                                      <span class="session-area-revision-title">{revision.title}</span>
                                                      <span class="session-area-revision-kind">{revision.kindLabel} revision</span>
                                                    </div>
                                                    <span class={`state-badge${revision.conflict ? " is-attention" : " is-recent"}`}>
                                                      {revision.conflict ? "conflict" : "pending revision"}
                                                    </span>
                                                  </div>
                                                </div>
                                              )}
                                            </For>
                                          </div>
                                        </section>
                                      </Show>

                                      <section class="session-area-discuss-context">
                                        <div class="session-question-kicker">Area context</div>
                                        <p class="session-project-support-copy">{area().summary}</p>
                                        <Show when={focusedAreaPressurePoint()}>
                                          {(point) => (
                                            <div class="session-area-shaping-focus">
                                              <div class="session-question-kicker">Why this still matters now</div>
                                              <p class="session-project-support-copy">{point().summary}</p>
                                            </div>
                                          )}
                                        </Show>
                                      </section>

                                      <Show when={resurfacedSeed()}>
                                        {(seed) => (
                                          <ResurfacedSeedCard
                                            areaTitle={area().title}
                                            seed={seed()}
                                            onPromote={() => {
                                              returnToShapingMode();
                                              promoteSeedIntoAreaWork(area().id, seed().id);
                                            }}
                                            onDismiss={() => dismissSeedForNow(area().id, seed().id)}
                                          />
                                        )}
                                      </Show>

                                      <div class="session-thread-workspace-note">
                                        Discussion stays secondary here. Return to shaping when you want to refine the visible objects directly.
                                      </div>

                                      <div class="session-area-discuss-actions">
                                        <button class="btn btn-subtle session-area-back-button" type="button" onClick={returnToShapingMode}>
                                          Back to shaping
                                        </button>
                                        <Show when={!discussComposerOpen() && activeAreaThread()}>
                                          <button class="btn btn-primary" type="button" onClick={openDiscussComposer}>
                                            Open composer
                                          </button>
                                        </Show>
                                      </div>

                                      {renderCompactProjectSupport()}

                                      <Show when={discussComposerOpen() && activeAreaThread()}>
                                        {(thread) => (
                                          <section class="session-area-thread-workspace">
                                            <div class="session-thread-workspace-head">
                                              <div class="session-thread-workspace-copy">
                                                <div class="session-thread-workspace-kicker">Active discussion</div>
                                                <h3 class="session-thread-section-title">{thread().title}</h3>
                                                <p class="session-thread-section-summary">{thread().summary}</p>
                                              </div>
                                              <div class="session-thread-workspace-meta">
                                                <span>{currentThreadProgress()} of {thread().prompt.items.length} answered</span>
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
                                                    saveStateState={props.controller.draftSaveState()}
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
                                          </section>
                                        )}
                                      </Show>
                                    </>
                                  }
                                >
                                  <section class="session-area-shaping">
                                    <section class="session-area-shaping-overview">
                                      <div class="session-question-kicker">Current context</div>
                                      <p class="session-project-support-copy">
                                        Start by refining the few objects that most change this area now. Use discussion only when the visible pressure becomes ambiguous or structural.
                                      </p>
                                      <Show when={focusedAreaPressurePoint()}>
                                        {(point) => (
                                          <div class="session-area-shaping-focus">
                                            <div class="session-question-kicker">Why this matters now</div>
                                            <p class="session-project-support-copy">{point().summary}</p>
                                          </div>
                                        )}
                                      </Show>
                                    </section>

                                    <Show when={resurfacedSeed()}>
                                      {(seed) => (
                                        <ResurfacedSeedCard
                                          areaTitle={area().title}
                                          seed={seed()}
                                          onPromote={() => promoteSeedIntoAreaWork(area().id, seed().id)}
                                          onDismiss={() => dismissSeedForNow(area().id, seed().id)}
                                        />
                                      )}
                                    </Show>

                                    <section class="session-area-shaping-pressure">
                                      <div class="session-question-kicker">Pressure points</div>
                                      <Show when={dominantAreaPressurePoint()}>
                                        {(point) => (
                                          <button
                                            class={`session-area-pressure-point is-dominant${focusedAreaPressurePoint()?.threadId === point().threadId ? " is-active" : ""}`}
                                            type="button"
                                            onClick={() => setFocusedPressureThreadId(point().threadId)}
                                          >
                                            <div class="session-area-pressure-head">
                                              <span class="session-area-pressure-title">{point().title}</span>
                                              <span class={`state-badge ${areaStateTone(point().state)}`}>{point().state}</span>
                                            </div>
                                            <p class="session-area-pressure-summary">{point().summary}</p>
                                            <div class="session-area-pressure-meta">
                                              {point().answeredCount}/{point().questionCount} answered · dominant
                                            </div>
                                          </button>
                                        )}
                                      </Show>
                                      <Show when={secondaryAreaPressurePoints().length > 0}>
                                        <div class="session-area-preview-secondary-list">
                                          <For each={secondaryAreaPressurePoints()}>
                                            {(point) => (
                                              <button
                                                class={`session-area-preview-secondary${focusedAreaPressurePoint()?.threadId === point.threadId ? " is-active" : ""}`}
                                                type="button"
                                                onClick={() => setFocusedPressureThreadId(point.threadId)}
                                              >
                                                <span class="session-area-preview-secondary-title">{point.title}</span>
                                                <span class={`state-badge ${areaStateTone(point.state)}`}>{point.state}</span>
                                              </button>
                                            )}
                                          </For>
                                        </div>
                                      </Show>
                                    </section>

                                    <section class="session-area-shaping-objects">
                                      <div class="session-question-kicker">Direct objects</div>
                                      <For each={shapingObjects()}>
                                        {(object) => (
                                          <AreaShapingObjectCard
                                            kind={object.kind}
                                            title={object.title}
                                            helper={object.helper}
                                            value={object.value}
                                            editing={editingObjectKey() === `${area().id}:${object.kind}`}
                                            onStartEdit={() => startEditingObject(area().id, object.kind, object.value)}
                                            onChange={value => updateShapingObjectDraft(area().id, object.kind, value)}
                                            onSave={saveEditingObject}
                                            onCancel={() => cancelEditingObject(area().id, object.kind, object.sourceValue)}
                                          />
                                        )}
                                      </For>
                                    </section>

                                    <Show when={area().pendingRevisions.length > 0}>
                                      <section class="session-area-revisions">
                                        <div class="session-question-kicker">Pending revisions</div>
                                        <div class="session-area-revision-list">
                                          <For each={area().pendingRevisions}>
                                            {(revision) => (
                                              <div class={`session-area-revision-card${revision.conflict ? " is-conflict" : ""}`}>
                                                <div class="session-area-revision-title-row">
                                                  <div class="session-area-revision-copy">
                                                    <span class="session-area-revision-title">{revision.title}</span>
                                                    <span class="session-area-revision-kind">{revision.kindLabel} revision</span>
                                                  </div>
                                                  <span class={`state-badge${revision.conflict ? " is-attention" : " is-recent"}`}>
                                                    {revision.conflict ? "conflict" : "pending revision"}
                                                  </span>
                                                </div>
                                                <p class="session-area-revision-summary">{revision.summary}</p>
                                              </div>
                                            )}
                                          </For>
                                        </div>
                                      </section>
                                    </Show>

                                    <section class="session-area-capture">
                                      <div class="session-question-kicker">Add to this area</div>
                                      <textarea
                                        ref={element => {
                                          areaCaptureInputRefs[area().id] = element;
                                        }}
                                        class="session-project-capture-input"
                                        value={areaCaptureTextById()[area().id] ?? ""}
                                        onInput={(event) => updateAreaCaptureText(area().id, event.currentTarget.value)}
                                        placeholder={`Capture something local to ${area().title}`}
                                      />
                                      <div class="session-project-capture-actions">
                                        <button class="btn btn-primary" type="button" onClick={() => commitAreaCapture(area().id)}>
                                          Save local note
                                        </button>
                                        <button class="btn btn-subtle" type="button" onClick={() => commitAreaSeed(area().id)}>
                                          Save as seed
                                        </button>
                                        <button
                                          class="btn btn-subtle"
                                          type="button"
                                          onClick={() => enterDiscussMode(area().id, focusedAreaPressurePoint()?.threadId ?? null)}
                                        >
                                          Discuss in composer
                                        </button>
                                      </div>
                                      <Show when={(areaCapturesById()[area().id] ?? []).length > 0}>
                                        <div class="session-project-capture-list">
                                          <For each={areaCapturesById()[area().id] ?? []}>
                                            {(note) => <div class="session-project-capture-note">{note}</div>}
                                          </For>
                                        </div>
                                      </Show>
                                      <Show when={(areaSeedsById()[area().id] ?? []).length > 0}>
                                        <div class="session-area-seed-resting-copy">
                                          {(areaSeedsById()[area().id] ?? []).length} seed{(areaSeedsById()[area().id] ?? []).length === 1 ? "" : "s"} resting quietly for later.
                                        </div>
                                      </Show>
                                    </section>

                                    {renderCompactProjectSupport()}
                                  </section>
                                </Show>
                              }
                            >
                              <section class="session-area-preview">
                                <div class="session-thread-workspace-note">
                                  Start here with one dominant pressure point, then go deeper when you are ready to shape this area directly.
                                </div>
                                <Show when={dominantPreviewPoint()}>
                                  {(point) => (
                                    <button
                                      class="session-area-preview-dominant"
                                      type="button"
                                      onClick={() => enterShapingMode(area().id, point().threadId)}
                                    >
                                      <div class="session-area-pressure-head">
                                        <span class="session-area-pressure-title">{point().title}</span>
                                        <span class={`state-badge ${areaStateTone(point().state)}`}>{point().state}</span>
                                      </div>
                                      <p class="session-area-pressure-summary">{point().summary}</p>
                                      <div class="session-area-pressure-meta">
                                        {point().answeredCount}/{point().questionCount} answered
                                      </div>
                                    </button>
                                  )}
                                </Show>
                                <Show when={secondaryPreviewPoints().length > 0}>
                                  <div class="session-area-preview-secondary-list">
                                    <For each={secondaryPreviewPoints()}>
                                      {(point) => (
                                        <button
                                          class="session-area-preview-secondary"
                                          type="button"
                                          onClick={() => enterShapingMode(area().id, point.threadId)}
                                        >
                                          <span class="session-area-preview-secondary-title">{point.title}</span>
                                          <span class={`state-badge ${areaStateTone(point.state)}`}>{point.state}</span>
                                        </button>
                                      )}
                                    </For>
                                  </div>
                                </Show>
                                <div class="session-area-preview-actions">
                                  <button
                                    class="btn btn-primary"
                                    type="button"
                                    onClick={() => enterShapingMode(area().id, dominantPreviewPoint()?.threadId ?? null)}
                                  >
                                    Go deeper in {area().title}
                                  </button>
                                </div>
                              </section>
                            </Show>
                          </div>
                        </section>
                      )}
                    </Show>
                  </div>

                  <Show when={areaSurface() === "preview"}>
                    <aside class="session-project-support">
                      <section class="session-project-support-panel">
                        <div class="session-question-kicker">Next move</div>
                        <Show when={recommendedArea()}>
                          {(area) => (
                            <>
                              <div class="session-project-support-title">{area().title}</div>
                              <p class="session-project-support-copy">{area().summary}</p>
                              <button
                                class="btn btn-primary"
                                type="button"
                                onClick={() => enterShapingMode(area().id, area().pressurePoints[0]?.threadId ?? null)}
                              >
                                Go deeper in {area().title}
                              </button>
                            </>
                          )}
                        </Show>
                      </section>

                      <section class="session-project-support-panel">
                        <div class="session-question-kicker">Global capture</div>
                        <p class="session-project-support-copy">
                          Capture a thought before deciding which area it belongs to.
                        </p>
                        <textarea
                          class="session-project-capture-input"
                          value={globalCaptureText()}
                          onInput={(event) => setGlobalCaptureText(event.currentTarget.value)}
                          placeholder="Capture an idea for the current project"
                        />
                        <div class="session-project-capture-actions">
                          <button class="btn btn-primary" type="button" onClick={commitGlobalCapture}>
                            Save local note
                          </button>
                        </div>
                        <Show when={globalCaptures().length > 0}>
                          <div class="session-project-capture-list">
                            <For each={globalCaptures()}>
                              {(note) => <div class="session-project-capture-note">{note}</div>}
                            </For>
                          </div>
                        </Show>
                      </section>

                      <Show when={props.controller.promptBankGraph().buildReady || props.controller.promptBankGraph().buildReadinessMessage}>
                        <section class="session-project-support-panel is-accent">
                          <div class="session-question-kicker">Build readiness</div>
                          <div class="session-project-support-title">
                            {props.controller.promptBankGraph().buildReady ? "Path is clear" : "Still converging"}
                          </div>
                          <p class="session-project-support-copy">
                            {props.controller.promptBankGraph().buildReadinessMessage
                              ?? "No additional prompt-bank blockers are reported from the current workspace snapshot."}
                          </p>
                        </section>
                      </Show>
                    </aside>
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
