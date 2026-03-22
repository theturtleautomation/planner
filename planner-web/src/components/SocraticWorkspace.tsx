import PromptBatchPanel from './PromptBatchPanel.tsx';
import type {
  PlannerEvent,
  PromptEnvelope,
  PromptAnswer,
  SocraticWorkspaceSnapshot,
} from '../types.ts';

interface SocraticWorkspaceProps {
  workspace: SocraticWorkspaceSnapshot;
  currentPrompt: PromptEnvelope | null;
  pendingCategoryId: string | null;
  workspaceNotice: string | null;
  currentStep: string | null;
  events: PlannerEvent[];
  disabled?: boolean;
  onFocusCategory: (categoryId: string, revision: string) => void;
  onShowAll: () => void;
  onSubmitAnswers: (answers: PromptAnswer[]) => void;
  onDone: () => void;
  isQuestionMapOpen: boolean;
  onToggleQuestionMap: () => void;
  isContextOpen: boolean;
  onToggleContext: () => void;
  contextUnreadCount: number;
  hasDraft: boolean;
}

function latestWorkspaceEvent(events: PlannerEvent[]): string | null {
  return events.find((event) => event.step?.startsWith('socratic.'))?.message ?? null;
}

function activePromptCategoryId(prompt: PromptEnvelope | null): string | null {
  if (!prompt) return null;
  return prompt.origin_category_id
    ?? prompt.category_path[prompt.category_path.length - 1]?.category_id
    ?? null;
}

function groupStatusLabel(status: SocraticWorkspaceSnapshot['groups'][number]['status']): string {
  switch (status) {
    case 'ready':
      return 'ready';
    case 'active':
      return 'active';
    case 'blocked':
      return 'blocked';
    case 'complete':
      return 'resolved';
    case 'pending':
    default:
      return 'preparing';
  }
}

export default function SocraticWorkspace({
  workspace,
  currentPrompt,
  pendingCategoryId,
  workspaceNotice,
  currentStep,
  events,
  disabled = false,
  onFocusCategory,
  onShowAll,
  onSubmitAnswers,
  onDone,
  isQuestionMapOpen,
  onToggleQuestionMap,
  isContextOpen,
  onToggleContext,
  contextUnreadCount,
  hasDraft,
}: SocraticWorkspaceProps) {
  const activeCategoryId = activePromptCategoryId(currentPrompt);
  const focusedCategoryId = pendingCategoryId ?? workspace.focused_category_id ?? activeCategoryId ?? workspace.groups.find((group) => group.is_focused)?.category_id ?? workspace.groups[0]?.category_id ?? null;
  const focusedGroup = focusedCategoryId
    ? workspace.groups.find((group) => group.category_id === focusedCategoryId) ?? null
    : null;
  const readyCount = workspace.groups.filter((group) => group.status === 'ready' || group.status === 'active').length;
  const blockedCount = workspace.groups.filter((group) => group.status === 'blocked').length;
  const resolvedCount = workspace.groups.filter((group) => group.status === 'complete').length;
  const changedCount = workspace.groups.filter((group) => group.is_new).length;
  const preparingCount = pendingCategoryId
    ? 1
    : workspace.groups.filter((group) => group.status === 'pending').length;
  const statusCopy = workspace.branch_notice
    ?? workspaceNotice
    ?? latestWorkspaceEvent(events)
    ?? currentStep
    ?? workspace.category_snapshot.build_readiness_message;
  const pulseLabel = workspace.category_snapshot.build_ready
    ? 'Build ready'
    : preparingCount > 0
      ? 'Preparing'
      : workspace.branch_notice || workspaceNotice
        ? 'Changed'
        : readyCount > 0
          ? 'Ready now'
          : 'Waiting';
  const isFocusedGroupPreparing = Boolean(
    focusedGroup && pendingCategoryId === focusedGroup.category_id && activeCategoryId !== focusedGroup.category_id,
  );
  const isFocusedGroupActive = Boolean(
    focusedGroup && currentPrompt && activeCategoryId === focusedGroup.category_id,
  );
  const canvasHeading = workspace.category_snapshot.build_ready
    ? 'All required Socratic work is complete.'
    : focusedGroup?.title ?? 'Preparing the next question set';
  const canvasSummary = workspace.branch_notice
    ?? focusedGroup?.summary
    ?? workspace.category_snapshot.build_readiness_message;

  return (
    <section
      style={{
        position: 'relative',
        display: 'grid',
        gap: '14px',
        minHeight: 0,
        background: 'var(--color-surface)',
        boxShadow: 'var(--shadow-sm)',
        padding: '18px',
      }}
    >
      <header
        style={{
          display: 'grid',
          gap: '12px',
          padding: '14px 16px',
          borderRadius: '18px',
          background: 'var(--color-surface-2)',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '12px', flexWrap: 'wrap' }}>
          <div style={{ display: 'grid', gap: '6px' }}>
            <div style={{ display: 'flex', gap: '8px', alignItems: 'center', flexWrap: 'wrap' }}>
              <span
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  padding: '5px 10px',
                  borderRadius: '999px',
                  background: workspace.category_snapshot.build_ready
                    ? 'rgba(109, 170, 69, 0.14)'
                    : pulseLabel === 'Preparing'
                      ? 'rgba(255, 215, 0, 0.12)'
                      : pulseLabel === 'Changed'
                        ? 'rgba(0, 212, 255, 0.12)'
                        : 'rgba(136, 136, 160, 0.14)',
                  color: workspace.category_snapshot.build_ready
                    ? 'var(--color-success)'
                    : pulseLabel === 'Preparing'
                      ? 'var(--color-gold)'
                      : pulseLabel === 'Changed'
                        ? 'var(--color-primary)'
                        : 'var(--color-text)',
                  fontSize: '10px',
                  fontWeight: 700,
                  letterSpacing: '0.08em',
                  textTransform: 'uppercase',
                }}
              >
                {pulseLabel}
              </span>
              <span
                style={{
                  fontSize: '11px',
                  fontWeight: 700,
                  letterSpacing: '0.08em',
                  textTransform: 'uppercase',
                  color: 'var(--color-primary)',
                }}
              >
                Focused question lobby
              </span>
            </div>
            <h3
              style={{
                fontFamily: 'var(--font-display)',
                fontSize: 'var(--text-lg)',
                lineHeight: 1.1,
                margin: 0,
              }}
            >
              {canvasHeading}
            </h3>
            <p style={{ margin: 0, color: 'var(--color-text-muted)', fontSize: '12px', lineHeight: 1.5 }}>
              {statusCopy}
            </p>
          </div>

          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            <button
              type="button"
              onClick={onToggleQuestionMap}
              disabled={disabled}
              aria-expanded={isQuestionMapOpen}
              aria-label="Toggle question map"
              style={{
                background: isQuestionMapOpen ? 'var(--color-primary-highlight)' : 'var(--color-surface)',
                boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                borderRadius: '999px',
                color: isQuestionMapOpen ? 'var(--color-primary)' : 'var(--color-text)',
                fontSize: '11px',
                fontWeight: 700,
                letterSpacing: '0.04em',
                padding: '6px 12px',
              }}
            >
              {isQuestionMapOpen ? 'Hide question map' : 'Question map'}
            </button>
            <button
              type="button"
              onClick={onToggleContext}
              disabled={disabled}
              aria-expanded={isContextOpen}
              aria-label="Toggle context shelf"
              style={{
                background: isContextOpen ? 'var(--color-primary-highlight)' : 'var(--color-surface)',
                boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                borderRadius: '999px',
                color: isContextOpen ? 'var(--color-primary)' : 'var(--color-text)',
                fontSize: '11px',
                fontWeight: 700,
                letterSpacing: '0.04em',
                padding: '6px 12px',
              }}
            >
              Context
              {(contextUnreadCount > 0 || hasDraft) && !isContextOpen && (
                <span style={{ marginLeft: '6px', color: 'var(--color-primary)' }}>
                  {contextUnreadCount > 0 ? `(${contextUnreadCount})` : 'new'}
                </span>
              )}
            </button>
            <button
              type="button"
              onClick={onDone}
              disabled={disabled || !workspace.category_snapshot.build_ready}
              style={{
                background: workspace.category_snapshot.build_ready ? 'var(--color-success)' : 'transparent',
                boxShadow: workspace.category_snapshot.build_ready
                  ? 'var(--shadow-sm)'
                  : 'inset 0 0 0 1px var(--color-divider)',
                borderRadius: '999px',
                color: workspace.category_snapshot.build_ready ? 'var(--color-bg)' : 'var(--color-text-muted)',
                fontSize: '11px',
                fontWeight: 700,
                letterSpacing: '0.04em',
                padding: '6px 12px',
              }}
            >
              Start building
            </button>
          </div>
        </div>

        <div className="directory-row-meta">
          <span className="utility-pill">{readyCount} ready now</span>
          {preparingCount > 0 && <span className="utility-pill">{preparingCount} preparing</span>}
          {changedCount > 0 && <span className="utility-pill">{changedCount} changed</span>}
          {blockedCount > 0 && <span className="utility-pill">{blockedCount} blocked</span>}
          {resolvedCount > 0 && <span className="utility-pill">{resolvedCount} resolved</span>}
        </div>
      </header>

      <div
        style={{
          display: 'grid',
          minHeight: 0,
        }}
      >
        {!focusedGroup ? (
          <div
            style={{
              borderRadius: '20px',
              background: 'var(--color-surface-2)',
              boxShadow: 'inset 0 0 0 1px var(--color-divider)',
              padding: '22px',
              color: 'var(--color-text-muted)',
              fontSize: '13px',
              lineHeight: 1.6,
            }}
          >
            {workspace.category_snapshot.build_ready
              ? 'No active question groups remain. Build can start from this focused lobby.'
              : 'Planner is preparing the next branch. Keep the current lobby focused here and open Question map if you want to inspect the wider category state.'}
          </div>
        ) : (
          <section
            style={{
              display: 'grid',
              gap: '14px',
              padding: '18px',
              borderRadius: '20px',
              background: 'linear-gradient(180deg, color-mix(in srgb, var(--color-surface-2) 86%, transparent), var(--color-surface))',
              boxShadow: 'inset 0 0 0 1px var(--color-divider)',
            }}
          >
            <header style={{ display: 'grid', gap: '8px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', alignItems: 'flex-start', flexWrap: 'wrap' }}>
                <div style={{ display: 'grid', gap: '6px' }}>
                  <span style={{ fontSize: '11px', color: 'var(--color-primary)', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase' }}>
                    {groupStatusLabel(focusedGroup.status)}
                  </span>
                  <h4 style={{ margin: 0, fontSize: '20px', lineHeight: 1.15 }}>{focusedGroup.title}</h4>
                  <p style={{ margin: 0, color: 'var(--color-text-muted)', fontSize: '13px', lineHeight: 1.6 }}>
                    {canvasSummary}
                  </p>
                </div>
                <div className="directory-row-meta">
                  {focusedGroup.is_new && <span className="utility-pill">new branch</span>}
                  <span className="utility-pill">{focusedGroup.question_count} active question{focusedGroup.question_count === 1 ? '' : 's'}</span>
                </div>
              </div>
              {workspace.branch_notice && (
                <div
                  style={{
                    borderRadius: '14px',
                    padding: '12px 14px',
                    background: 'color-mix(in srgb, var(--color-primary-highlight) 72%, transparent)',
                    color: 'var(--color-text)',
                    fontSize: '12px',
                    lineHeight: 1.55,
                  }}
                >
                  {workspace.branch_notice}
                </div>
              )}
            </header>

            {isFocusedGroupPreparing ? (
              <div
                style={{
                  display: 'grid',
                  gap: '10px',
                  borderRadius: '16px',
                  background: 'var(--color-surface-offset)',
                  padding: '18px',
                }}
              >
                <span style={{ fontSize: '11px', fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--color-gold)' }}>
                  Preparing next questions
                </span>
                <p style={{ margin: 0, color: 'var(--color-text-muted)', fontSize: '13px', lineHeight: 1.6 }}>
                  Planner is synthesizing the next question set for this branch. The canvas stays focused here while the question map continues to reflect wider category changes.
                </p>
                <div style={{ display: 'grid', gap: '8px' }}>
                  {[0, 1, 2].map((index) => (
                    <div
                      key={index}
                      style={{
                        height: '48px',
                        borderRadius: '12px',
                        background: 'linear-gradient(90deg, color-mix(in srgb, var(--color-surface-dynamic) 45%, transparent), color-mix(in srgb, var(--color-surface-raised) 75%, transparent), color-mix(in srgb, var(--color-surface-dynamic) 45%, transparent))',
                        backgroundSize: '180% 100%',
                        animation: 'workspacePulse 1.2s ease-in-out infinite',
                      }}
                    />
                  ))}
                </div>
              </div>
            ) : isFocusedGroupActive && currentPrompt ? (
              <PromptBatchPanel
                prompt={currentPrompt}
                onSubmit={(_promptId, answers) => onSubmitAnswers(answers)}
                disabled={disabled}
                onDone={onDone}
              />
            ) : (
              <div style={{ display: 'grid', gap: '10px' }}>
                <div
                  style={{
                    borderRadius: '16px',
                    background: 'var(--color-surface-offset)',
                    padding: '16px',
                    display: 'grid',
                    gap: '8px',
                  }}
                >
                  <span style={{ fontSize: '11px', color: 'var(--color-primary)', fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase' }}>
                    Focus transition
                  </span>
                  <p style={{ margin: 0, color: 'var(--color-text)', fontSize: '13px', lineHeight: 1.6 }}>
                    {workspace.category_snapshot.build_ready
                      ? 'Required question work is complete. Review context if needed, then start building.'
                      : 'This branch is in view even though the active prompt has moved. Use the question map to inspect all active groups or return the lobby to the server-selected branch.'}
                  </p>
                  <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
                    <button
                      type="button"
                      onClick={() => onFocusCategory(focusedGroup.category_id, workspace.category_snapshot.revision)}
                      disabled={disabled}
                      style={{
                        background: 'var(--color-primary-highlight)',
                        boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                        borderRadius: '999px',
                        color: 'var(--color-primary)',
                        fontSize: '11px',
                        fontWeight: 700,
                        padding: '6px 12px',
                      }}
                    >
                      Focus this branch
                    </button>
                    <button
                      type="button"
                      onClick={onShowAll}
                      disabled={disabled}
                      style={{
                        background: 'var(--color-surface)',
                        boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                        borderRadius: '999px',
                        color: 'var(--color-text)',
                        fontSize: '11px',
                        fontWeight: 700,
                        padding: '6px 12px',
                      }}
                    >
                      Follow server focus
                    </button>
                  </div>
                </div>
                <div style={{ display: 'grid', gap: '8px' }}>
                  {focusedGroup.preview_items.map((item) => (
                    <div
                      key={item.item_id}
                      style={{
                        borderRadius: '14px',
                        background: 'var(--color-surface-offset)',
                        padding: '14px 16px',
                        display: 'grid',
                        gap: '4px',
                      }}
                    >
                      <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                        {item.kind.replace('_', ' ')}
                      </span>
                      <span style={{ fontSize: '13px', lineHeight: 1.55 }}>{item.text}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </section>
        )}
      </div>

      {isQuestionMapOpen && (
        <aside
          aria-label="Question map"
          style={{
            position: 'absolute',
            top: '92px',
            right: '18px',
            width: 'min(420px, calc(100vw - 48px))',
            maxHeight: 'calc(100% - 110px)',
            overflowY: 'auto',
            display: 'grid',
            gap: '12px',
            padding: '16px',
            borderRadius: '20px',
            background: 'color-mix(in srgb, var(--color-surface) 96%, transparent)',
            boxShadow: 'var(--shadow-lg)',
            border: '1px solid var(--color-divider)',
            zIndex: 20,
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', alignItems: 'flex-start' }}>
            <div style={{ display: 'grid', gap: '4px' }}>
              <span style={{ fontSize: '11px', color: 'var(--color-primary)', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase' }}>
                Question map
              </span>
              <h4 style={{ margin: 0, fontSize: '18px', lineHeight: 1.15 }}>All active question groups</h4>
              <p style={{ margin: 0, color: 'var(--color-text-muted)', fontSize: '12px', lineHeight: 1.5 }}>
                Inspect dynamic categories and move the focused canvas without serial branch hunting.
              </p>
            </div>
            <button
              type="button"
              onClick={onToggleQuestionMap}
              style={{
                background: 'var(--color-surface-2)',
                boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                borderRadius: '999px',
                color: 'var(--color-text)',
                fontSize: '11px',
                fontWeight: 700,
                padding: '6px 12px',
              }}
            >
              Close
            </button>
          </div>

          {workspace.groups.map((group) => {
            const isFocused = focusedCategoryId === group.category_id;
            const isPreparing = pendingCategoryId === group.category_id && activeCategoryId !== group.category_id;
            return (
              <section
                key={group.category_id}
                style={{
                  display: 'grid',
                  gap: '8px',
                  padding: '14px',
                  borderRadius: '16px',
                  background: isFocused ? 'var(--color-primary-highlight)' : 'var(--color-surface-2)',
                  boxShadow: isFocused ? 'var(--shadow-sm)' : 'inset 0 0 0 1px var(--color-divider)',
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '10px', alignItems: 'flex-start' }}>
                  <div style={{ display: 'grid', gap: '5px' }}>
                    <div style={{ display: 'flex', gap: '6px', alignItems: 'center', flexWrap: 'wrap' }}>
                      <span style={{ fontSize: '13px', fontWeight: 700 }}>{group.title}</span>
                      <span className="utility-pill">{groupStatusLabel(group.status)}</span>
                      {group.is_new && <span className="utility-pill">new</span>}
                      {isPreparing && <span className="utility-pill">preparing</span>}
                    </div>
                    <p style={{ margin: 0, color: 'var(--color-text-muted)', fontSize: '12px', lineHeight: 1.5 }}>
                      {group.summary}
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      onFocusCategory(group.category_id, workspace.category_snapshot.revision);
                      onToggleQuestionMap();
                    }}
                    disabled={disabled}
                    style={{
                      background: isFocused ? 'var(--color-surface)' : 'var(--color-primary-highlight)',
                      boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                      borderRadius: '999px',
                      color: isFocused ? 'var(--color-text)' : 'var(--color-primary)',
                      fontSize: '11px',
                      fontWeight: 700,
                      padding: '6px 12px',
                    }}
                  >
                    {isFocused ? 'Focused' : 'Focus'}
                  </button>
                </div>
                <div className="directory-row-meta">
                  <span className="utility-pill">{group.question_count} question{group.question_count === 1 ? '' : 's'}</span>
                </div>
                <div style={{ display: 'grid', gap: '6px' }}>
                  {group.preview_items.map((item) => (
                    <div
                      key={item.item_id}
                      style={{
                        borderRadius: '12px',
                        background: 'var(--color-surface-offset)',
                        padding: '10px 12px',
                        display: 'grid',
                        gap: '4px',
                      }}
                    >
                      <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                        {item.kind.replace('_', ' ')}
                      </span>
                      <span style={{ fontSize: '12px', lineHeight: 1.5 }}>{item.text}</span>
                    </div>
                  ))}
                </div>
              </section>
            );
          })}
        </aside>
      )}
    </section>
  );
}
