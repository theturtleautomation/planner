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
}: SocraticWorkspaceProps) {
  const activeCategoryId = activePromptCategoryId(currentPrompt);
  const focusedCategoryId = pendingCategoryId ?? workspace.focused_category_id ?? activeCategoryId;
  const visibleGroups = focusedCategoryId
    ? workspace.groups.filter((group) => group.category_id === focusedCategoryId)
    : workspace.groups;
  const statusCopy = workspaceNotice
    ?? latestWorkspaceEvent(events)
    ?? currentStep
    ?? workspace.category_snapshot.build_readiness_message;

  return (
    <section
      style={{
        display: 'grid',
        gridTemplateRows: 'auto 1fr',
        gap: '12px',
        minHeight: 0,
        background: 'var(--color-surface)',
        boxShadow: 'var(--shadow-sm)',
        padding: '14px 16px',
      }}
    >
      <header style={{ display: 'grid', gap: '10px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '12px', flexWrap: 'wrap' }}>
          <div style={{ display: 'grid', gap: '4px' }}>
            <span
              style={{
                fontSize: '11px',
                fontWeight: 700,
                letterSpacing: '0.08em',
                textTransform: 'uppercase',
                color: 'var(--color-primary)',
              }}
            >
              Live Question Workspace
            </span>
            <h3
              style={{
                fontFamily: 'var(--font-display)',
                fontSize: 'var(--text-lg)',
                lineHeight: 1.1,
              }}
            >
              {focusedCategoryId
                ? 'Focused category'
                : 'All active questions'}
            </h3>
          </div>

          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            <button
              type="button"
              onClick={onShowAll}
              disabled={disabled}
              style={{
                background: focusedCategoryId ? 'var(--color-surface-2)' : 'var(--color-primary-highlight)',
                boxShadow: 'inset 0 0 0 1px var(--color-ghost-border)',
                borderRadius: '999px',
                color: focusedCategoryId ? 'var(--color-text)' : 'var(--color-primary)',
                fontSize: '11px',
                fontWeight: 700,
                letterSpacing: '0.04em',
                padding: '6px 12px',
              }}
            >
              All active
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

        <div
          style={{
            borderRadius: '12px',
            padding: '10px 12px',
            background: workspaceNotice
              ? 'color-mix(in srgb, var(--color-warning-highlight) 72%, transparent)'
              : 'var(--color-surface-2)',
            color: 'var(--color-text)',
            fontSize: '12px',
            lineHeight: 1.5,
          }}
        >
          {statusCopy}
        </div>
      </header>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '280px minmax(0, 1fr)',
          gap: '14px',
          minHeight: 0,
        }}
      >
        <aside
          style={{
            minHeight: 0,
            overflowY: 'auto',
            display: 'grid',
            gap: '8px',
            paddingRight: '4px',
          }}
        >
          {workspace.groups.map((group) => {
            const isFocused = focusedCategoryId === group.category_id;
            const isPreparing = pendingCategoryId === group.category_id && activeCategoryId !== group.category_id;
            return (
              <button
                key={group.category_id}
                type="button"
                onClick={() => onFocusCategory(group.category_id, workspace.category_snapshot.revision)}
                disabled={disabled}
                style={{
                  textAlign: 'left',
                  display: 'grid',
                  gap: '6px',
                  padding: '12px',
                  borderRadius: '14px',
                  background: isFocused
                    ? 'var(--color-primary-highlight)'
                    : 'var(--color-surface-2)',
                  boxShadow: isFocused
                    ? 'var(--shadow-sm)'
                    : 'inset 0 0 0 1px var(--color-divider)',
                }}
              >
                <div style={{ display: 'flex', gap: '6px', alignItems: 'center', flexWrap: 'wrap' }}>
                  <span style={{ fontSize: '12px', fontWeight: 700 }}>{group.title}</span>
                  {group.is_new && (
                    <span style={{ fontSize: '9px', fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--color-primary)' }}>
                      New
                    </span>
                  )}
                  {isPreparing && (
                    <span style={{ fontSize: '9px', fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--color-warning)' }}>
                      Preparing
                    </span>
                  )}
                </div>
                <span style={{ fontSize: '11px', color: 'var(--color-text-muted)', lineHeight: 1.45 }}>{group.summary}</span>
                <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                  {group.question_count} active question{group.question_count === 1 ? '' : 's'}
                </span>
              </button>
            );
          })}
        </aside>

        <div
          style={{
            minHeight: 0,
            overflowY: 'auto',
            display: 'grid',
            gap: '12px',
            paddingRight: '4px',
          }}
        >
          {visibleGroups.length === 0 ? (
            <div
              style={{
                borderRadius: '14px',
                background: 'var(--color-surface-2)',
                boxShadow: 'inset 0 0 0 1px var(--color-divider)',
                padding: '18px',
                color: 'var(--color-text-muted)',
                fontSize: '12px',
                lineHeight: 1.6,
              }}
            >
              {workspace.category_snapshot.build_ready
                ? 'No active question groups remain. Build can start from this workspace.'
                : 'No question groups are active in the current workspace yet. Choose a category from the rail or wait for Planner to synthesize the next group.'}
            </div>
          ) : (
            visibleGroups.map((group) => {
              const isActivePrompt = activeCategoryId === group.category_id && currentPrompt !== null;
              const isPreparing = pendingCategoryId === group.category_id && !isActivePrompt;
              return (
                <section
                  key={group.category_id}
                  style={{
                    display: 'grid',
                    gap: '10px',
                    padding: '14px',
                    borderRadius: '16px',
                    background: 'var(--color-surface-2)',
                    boxShadow: 'inset 0 0 0 1px var(--color-divider)',
                  }}
                >
                  <header style={{ display: 'grid', gap: '4px' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', gap: '10px', flexWrap: 'wrap' }}>
                      <div style={{ display: 'flex', gap: '8px', alignItems: 'center', flexWrap: 'wrap' }}>
                        <h4 style={{ fontSize: '13px', fontWeight: 700 }}>{group.title}</h4>
                        <span style={{ fontSize: '10px', textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--color-text-muted)' }}>
                          {group.status}
                        </span>
                      </div>
                      {!isActivePrompt && (
                        <button
                          type="button"
                          onClick={() => onFocusCategory(group.category_id, workspace.category_snapshot.revision)}
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
                          Open questions
                        </button>
                      )}
                    </div>
                    <p style={{ fontSize: '12px', color: 'var(--color-text-muted)', lineHeight: 1.5 }}>
                      {group.summary}
                    </p>
                  </header>

                  {isPreparing ? (
                    <div
                      style={{
                        display: 'grid',
                        gap: '8px',
                        borderRadius: '14px',
                        background: 'var(--color-surface-offset)',
                        padding: '14px',
                      }}
                    >
                      <span style={{ fontSize: '11px', fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--color-warning)' }}>
                        Preparing question set
                      </span>
                      <div style={{ display: 'grid', gap: '8px' }}>
                        {[0, 1, 2].map((index) => (
                          <div
                            key={index}
                            style={{
                              height: '42px',
                              borderRadius: '10px',
                              background: 'linear-gradient(90deg, color-mix(in srgb, var(--color-surface-dynamic) 45%, transparent), color-mix(in srgb, var(--color-surface-raised) 75%, transparent), color-mix(in srgb, var(--color-surface-dynamic) 45%, transparent))',
                              backgroundSize: '180% 100%',
                              animation: 'workspacePulse 1.2s ease-in-out infinite',
                            }}
                          />
                        ))}
                      </div>
                    </div>
                  ) : isActivePrompt && currentPrompt ? (
                    <PromptBatchPanel
                      prompt={currentPrompt}
                      onSubmit={(_promptId, answers) => onSubmitAnswers(answers)}
                      disabled={disabled}
                    />
                  ) : (
                    <div style={{ display: 'grid', gap: '8px' }}>
                      {group.preview_items.map((item) => (
                        <div
                          key={item.item_id}
                          style={{
                            borderRadius: '12px',
                            background: 'var(--color-surface-offset)',
                            padding: '12px 14px',
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
                  )}
                </section>
              );
            })
          )}
        </div>
      </div>
    </section>
  );
}
