import { useMemo, useRef, useState, useEffect, type KeyboardEvent } from 'react';
import { useShallow } from 'zustand/react/shallow';
import VirtualizedCategoryDocument from './VirtualizedCategoryDocument.tsx';
import {
  useHydrateSocraticDocumentGraph,
  useSocraticDocumentCategoryViews,
} from '../stores/socraticDocumentStore.ts';
import {
  selectPromptProgress,
  useSocraticDraftStore,
} from '../stores/useSocraticDraftStore.ts';
import type {
  PromptEnvelope,
  PromptAnswer,
  SocraticCategoryNode,
  SocraticCategoryPathEntry,
  SocraticWorkspaceSnapshot,
} from '../types.ts';

interface SocraticWorkspaceProps {
  workspace: SocraticWorkspaceSnapshot;
  currentPrompt: PromptEnvelope | null;
  pendingCategoryId: string | null;
  workspaceNotice: string | null;
  disabled?: boolean;
  onFocusCategory: (categoryId: string, revision: string) => void;
  onShowAll: () => void;
  onSubmitAnswers: (answers: PromptAnswer[]) => void;
  onDone: () => void;
}

interface SidebarRowModel {
  categoryId: string;
  title: string;
  depth: number;
  telemetry: string;
  state: 'active' | 'partial' | 'complete' | 'ready' | 'pending' | 'blocked';
  isActive: boolean;
  isInteractive: boolean;
}

function activePromptCategoryId(prompt: PromptEnvelope | null): string | null {
  if (!prompt) return null;
  return prompt.origin_category_id
    ?? prompt.category_path[prompt.category_path.length - 1]?.category_id
    ?? null;
}

function activePathCategoryId(workspace: SocraticWorkspaceSnapshot): string | null {
  return workspace.category_snapshot.active_category_path[
    workspace.category_snapshot.active_category_path.length - 1
  ]?.category_id ?? null;
}

function currentPath(
  prompt: PromptEnvelope | null,
  workspace: SocraticWorkspaceSnapshot,
): SocraticCategoryPathEntry[] {
  if (prompt?.category_path?.length) return prompt.category_path;
  return workspace.category_snapshot.active_category_path ?? [];
}

function visibleCategoryIds(workspace: SocraticWorkspaceSnapshot): Set<string> {
  const activeId = activePathCategoryId(workspace);
  if (!activeId) {
    return new Set(workspace.category_snapshot.root_category_ids);
  }

  return new Set(
    workspace.category_snapshot.nodes
      .filter((node) => node.parent_category_id === activeId)
      .map((node) => node.category_id),
  );
}

function nodeStatusState(
  node: SocraticCategoryNode,
  isActive: boolean,
  answeredCount: number,
  totalCount: number,
): SidebarRowModel['state'] {
  if (isActive) return 'active';
  if (totalCount > 0 && answeredCount >= totalCount) return 'complete';
  if (answeredCount > 0) return 'partial';

  switch (node.status) {
    case 'ready':
      return 'ready';
    case 'blocked':
      return 'blocked';
    case 'complete':
      return 'complete';
    case 'active':
      return 'active';
    case 'pending':
    default:
      return 'pending';
  }
}

function rowTelemetry(
  node: SocraticCategoryNode,
  answeredCount: number,
  totalCount: number,
): string {
  if (totalCount > 0) {
    return `[ ${answeredCount}/${totalCount} ]`;
  }
  if (node.has_children) {
    return `[ ${node.item_count_hint} ]`;
  }
  return '[ 0/1 ]';
}

function formatMappedDimensions(node: SocraticCategoryNode | null): string | null {
  if (!node || node.mapped_dimensions.length === 0) return null;
  return node.mapped_dimensions
    .map((dimension) => {
      if (typeof dimension === 'string') return dimension;
      const keys = Object.keys(dimension);
      if (keys.length === 1 && typeof dimension[keys[0]] === 'string') {
        return String(dimension[keys[0]]);
      }
      return JSON.stringify(dimension);
    })
    .join(' | ');
}

export default function SocraticWorkspace({
  workspace,
  currentPrompt,
  pendingCategoryId,
  workspaceNotice,
  disabled = false,
  onFocusCategory,
  onShowAll,
  onSubmitAnswers,
  onDone,
}: SocraticWorkspaceProps) {
  const rowRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const deskBodyRef = useRef<HTMLDivElement | null>(null);
  useHydrateSocraticDocumentGraph(workspace, currentPrompt);
  const draftProgress = useSocraticDraftStore(
    useShallow((state) => selectPromptProgress(state, currentPrompt)),
  );
  const documentCategories = useSocraticDocumentCategoryViews();
  const [jumpTargetCategoryId, setJumpTargetCategoryId] = useState<string | null>(null);
  const [visibleCategoryId, setVisibleCategoryId] = useState<string | null>(null);
  const [previewCategoryId, setPreviewCategoryId] = useState<string | null>(null);
  const [focusTargetCategoryId, setFocusTargetCategoryId] = useState<string | null>(null);
  const [editingCategoryId, setEditingCategoryId] = useState<string | null>(null);

  const activeCategoryId = activePromptCategoryId(currentPrompt);
  const activePathFocusId = activePathCategoryId(workspace);
  const focusedCategoryId = pendingCategoryId
    ?? workspace.focused_category_id
    ?? activeCategoryId
    ?? activePathFocusId
    ?? workspace.groups.find((group) => group.is_focused)?.category_id
    ?? workspace.groups[0]?.category_id
    ?? null;

  const path = currentPath(currentPrompt, workspace);
  const visibleIds = useMemo(() => visibleCategoryIds(workspace), [workspace]);
  const pathIds = useMemo(() => new Set(path.map((entry) => entry.category_id)), [path]);
  const groupMap = useMemo(
    () => new Map(workspace.groups.map((group) => [group.category_id, group])),
    [workspace.groups],
  );
  const categoryViews = documentCategories.length > 0
    ? documentCategories
    : workspace.category_snapshot.nodes.map((node) => ({
      categoryId: node.category_id,
      parentCategoryId: node.parent_category_id ?? null,
      title: node.title,
      summary: node.summary,
      status: node.status,
      depth: node.depth,
      mappedDimensions: node.mapped_dimensions,
      hasChildren: node.has_children,
      hasPromptReady: node.has_prompt_ready,
      itemCountHint: node.item_count_hint,
      isNewlyAvailable: workspace.category_snapshot.newly_available_category_ids.includes(node.category_id),
      questionIds: [],
      latestPromptId: null,
      latestPromptTitle: null,
      latestPromptInstructions: null,
      answeredCount: 0,
      totalCount: Math.max(node.item_count_hint, node.has_prompt_ready ? 1 : 0),
    }));
  const focusedCategoryView = focusedCategoryId
    ? categoryViews.find((category) => category.categoryId === focusedCategoryId) ?? null
    : null;
  const displayCategoryId = editingCategoryId
    ?? previewCategoryId
    ?? visibleCategoryId
    ?? focusedCategoryId;
  const displayCategoryView = displayCategoryId
    ? categoryViews.find((category) => category.categoryId === displayCategoryId) ?? null
    : null;
  const focusedNode = focusedCategoryId
    ? workspace.category_snapshot.nodes.find((node) => node.category_id === focusedCategoryId) ?? null
    : null;
  const focusedGroup = focusedCategoryId ? groupMap.get(focusedCategoryId) ?? null : null;
  const displayNode = displayCategoryId
    ? workspace.category_snapshot.nodes.find((node) => node.category_id === displayCategoryId) ?? null
    : null;
  const displayGroup = displayCategoryId ? groupMap.get(displayCategoryId) ?? null : null;
  const deskTitle = 'Socratic workspace';
  const deskSummary = displayCategoryView?.title
    ?? displayNode?.title
    ?? displayGroup?.title
    ?? focusedNode?.title
    ?? focusedGroup?.title
    ?? currentPrompt?.title
    ?? null;
  const mappedDimensions = formatMappedDimensions(displayNode ?? focusedNode ?? null);
  const activeQuestionCount = currentPrompt && activeCategoryId === focusedCategoryId
    ? currentPrompt.items.length
    : Math.max(
      displayCategoryView?.totalCount ?? focusedCategoryView?.totalCount ?? 0,
      focusedGroup?.question_count ?? 0,
      focusedNode?.has_prompt_ready ? focusedNode.item_count_hint : 0,
    );
  const isPromptActive = Boolean(currentPrompt && activeCategoryId === displayCategoryId);
  const displayGroupPreviewCount = displayGroup?.preview_items?.length ?? 0;
  const displayRetainedQuestionCount = displayCategoryView?.questionIds.length ?? 0;
  const deskHasLocalContent = Boolean(
    isPromptActive
    || displayRetainedQuestionCount > 0
    || displayGroupPreviewCount > 0,
  );
  const deskIsPreparing = Boolean(
    displayCategoryId
    && pendingCategoryId === displayCategoryId
    && activeCategoryId !== displayCategoryId,
  ) && !deskHasLocalContent;

  const sidebarRows = useMemo<SidebarRowModel[]>(() => (
    categoryViews.map((category) => {
      const isActive = displayCategoryId === category.categoryId;
      const isInteractive = category.hasPromptReady
        || visibleIds.has(category.categoryId)
        || pathIds.has(category.categoryId)
        || isActive;

      return {
        categoryId: category.categoryId,
        title: category.title,
        depth: category.depth,
        telemetry: rowTelemetry({
          category_id: category.categoryId,
          parent_category_id: category.parentCategoryId ?? null,
          title: category.title,
          summary: category.summary,
          status: category.status,
          depth: category.depth,
          mapped_dimensions: category.mappedDimensions,
          has_children: category.hasChildren,
          has_prompt_ready: category.hasPromptReady,
          item_count_hint: category.itemCountHint,
        }, category.answeredCount, category.totalCount),
        state: nodeStatusState({
          category_id: category.categoryId,
          parent_category_id: category.parentCategoryId ?? null,
          title: category.title,
          summary: category.summary,
          status: category.status,
          depth: category.depth,
          mapped_dimensions: category.mappedDimensions,
          has_children: category.hasChildren,
          has_prompt_ready: category.hasPromptReady,
          item_count_hint: category.itemCountHint,
        }, isActive, category.answeredCount, category.totalCount),
        isActive,
        isInteractive,
      };
    })
  ), [
    categoryViews,
    displayCategoryId,
    focusedCategoryId,
    pathIds,
    visibleIds,
  ]);

  const interactiveRowIds = useMemo(
    () => sidebarRows.filter((row) => row.isInteractive).map((row) => row.categoryId),
    [sidebarRows],
  );

  useEffect(() => {
    if (!jumpTargetCategoryId) return;
    if (visibleCategoryId === jumpTargetCategoryId) {
      setJumpTargetCategoryId(null);
    }
  }, [jumpTargetCategoryId, visibleCategoryId]);

  useEffect(() => {
    if (!previewCategoryId) return;
    if (visibleCategoryId === previewCategoryId) {
      setPreviewCategoryId(null);
    }
  }, [previewCategoryId, visibleCategoryId]);

  const handleRowKeyDown = (categoryId: string, event: KeyboardEvent<HTMLButtonElement>): void => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();

      const currentIndex = interactiveRowIds.findIndex((id) => id === categoryId);
      if (currentIndex === -1) return;

      const nextIndex = event.key === 'ArrowDown'
        ? Math.min(currentIndex + 1, interactiveRowIds.length - 1)
        : Math.max(currentIndex - 1, 0);
      const nextId = interactiveRowIds[nextIndex];
      rowRefs.current[nextId]?.focus();
      setEditingCategoryId(null);
      setPreviewCategoryId(nextId);
      setJumpTargetCategoryId(nextId);
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      setEditingCategoryId(null);
      setPreviewCategoryId(categoryId);
      setJumpTargetCategoryId(categoryId);
      setFocusTargetCategoryId(categoryId);
      if (categoryId !== focusedCategoryId) {
        onFocusCategory(categoryId, workspace.category_snapshot.revision);
      }
    }
  };

  const handleFocusCategory = (categoryId: string): void => {
    setEditingCategoryId(null);
    setPreviewCategoryId(categoryId);
    setJumpTargetCategoryId(categoryId);
    if (categoryId !== focusedCategoryId) {
      onFocusCategory(categoryId, workspace.category_snapshot.revision);
    }
  };

  return (
    <section className="socratic-consultant-desk" aria-label="Socratic lobby consultant desk">
      <aside className="socratic-map" aria-label="Thread index">
        <div className="socratic-map__header">
          <span className="socratic-map__eyebrow">Thread index</span>
          <span className="socratic-map__summary">
            {workspace.category_snapshot.nodes.length} active threads
          </span>
        </div>

        <div className="socratic-map__list" role="list">
          {sidebarRows.map((row) => (
            <div key={row.categoryId} role="listitem">
              <button
                ref={(element) => {
                  rowRefs.current[row.categoryId] = element;
                }}
                type="button"
                data-category-id={row.categoryId}
                className={[
                  'socratic-map-row',
                  `is-${row.state}`,
                  row.isActive ? 'is-active' : '',
                ].filter(Boolean).join(' ')}
                style={{ ['--socratic-row-depth' as string]: String(row.depth) }}
                onClick={() => handleFocusCategory(row.categoryId)}
                onKeyDown={(event) => handleRowKeyDown(row.categoryId, event)}
                disabled={disabled || !row.isInteractive}
                aria-current={row.isActive ? 'true' : undefined}
                aria-label={`${row.title} ${row.telemetry}`}
              >
                <span className="socratic-map-row__indicator" aria-hidden="true" />
                <span className="socratic-map-row__label">{row.title}</span>
                <span className="socratic-map-row__telemetry">{row.telemetry}</span>
              </button>
            </div>
          ))}
        </div>

        {workspace.category_snapshot.build_ready && (
          <div className="socratic-map__footer">
            <button
              type="button"
              onClick={onDone}
              disabled={disabled}
              className="socratic-action-button primary"
            >
              Commit plan
            </button>
          </div>
        )}
      </aside>

      <section className="socratic-desk" aria-label="Consultant desk">
        <header className="socratic-desk__header">
          <div className="socratic-desk__title-block">
            <span className="socratic-terminal-kicker">
              {deskIsPreparing ? 'Preparing' : 'Workspace'}
            </span>
            <h2 className="socratic-desk__title">{deskTitle}</h2>
          </div>

          <div className="socratic-desk__meta" aria-label="Planner context">
            {deskSummary && (
              <span className="socratic-desk__meta-line">
                Viewing: {deskSummary}
              </span>
            )}
            {mappedDimensions && (
              <span className="socratic-desk__meta-line">
                Mapped dimensions: {mappedDimensions}
              </span>
            )}
              <span className="socratic-desk__meta-line">
              {isPromptActive
                ? `Draft progress ${draftProgress.answeredCount}/${draftProgress.totalCount}`
                : `${activeQuestionCount} question${activeQuestionCount === 1 ? '' : 's'} in play`}
            </span>
          </div>
        </header>

        {(workspaceNotice || workspace.branch_notice) && (
          <div className="socratic-cascade-notice" role="status">
            {workspace.branch_notice || workspaceNotice}
          </div>
        )}

        <div ref={deskBodyRef} className="socratic-desk__body">
          {workspace.groups.length === 0 && workspace.category_snapshot.build_ready ? (
            <div className="socratic-build-hero">
              <span className="socratic-terminal-kicker">Build ready</span>
              <h3 className="socratic-build-title">The plan is settled. Move into delivery.</h3>
              <p className="socratic-build-copy">
                The interview has converged. Open the context shelf for a final check, or commit the plan now.
              </p>
              <button
                type="button"
                onClick={onDone}
                disabled={disabled}
                className="socratic-action-button primary large"
              >
                Commit plan
              </button>
            </div>
          ) : (
            <VirtualizedCategoryDocument
              scrollElementRef={deskBodyRef}
              categories={categoryViews}
              currentPrompt={currentPrompt}
              pendingCategoryId={pendingCategoryId}
              branchNotice={workspace.branch_notice ?? workspaceNotice}
              focusedCategoryId={focusedCategoryId}
              groupMap={groupMap}
              jumpTargetCategoryId={jumpTargetCategoryId}
              focusTargetCategoryId={focusTargetCategoryId}
              disabled={disabled}
              onVisibleCategoryChange={setVisibleCategoryId}
              onFocusTargetHandled={(categoryId) => {
                if (focusTargetCategoryId === categoryId) {
                  setFocusTargetCategoryId(null);
                  setEditingCategoryId(categoryId);
                }
              }}
              onAnswerFocus={(categoryId) => {
                setEditingCategoryId(categoryId);
                setPreviewCategoryId(null);
              }}
              onSubmitAnswers={onSubmitAnswers}
              onDone={workspace.category_snapshot.build_ready ? onDone : undefined}
              onShowAll={onShowAll}
            />
          )}
        </div>
      </section>
    </section>
  );
}
