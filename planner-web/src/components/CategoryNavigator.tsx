import type { SocraticCategorySnapshot } from '../types.ts';

interface CategoryNavigatorProps {
  snapshot: SocraticCategorySnapshot;
  disabled?: boolean;
  onEnterCategory: (categoryId: string, revision: string) => void;
  onBack: () => void;
  onDone: () => void;
}

export default function CategoryNavigator({
  snapshot,
  disabled = false,
  onEnterCategory,
  onBack,
  onDone,
}: CategoryNavigatorProps) {
  const atMainScreen = snapshot.active_category_path.length === 0;
  const currentTitle = atMainScreen
    ? 'Choose where to go deeper'
    : snapshot.active_category_path[snapshot.active_category_path.length - 1]?.title ?? 'Category';
  const activeCategoryId = snapshot.active_category_path[snapshot.active_category_path.length - 1]?.category_id ?? null;
  const visibleNodes = activeCategoryId
    ? snapshot.nodes.filter((node) => node.parent_category_id === activeCategoryId)
    : snapshot.root_category_ids
      .map((categoryId) => snapshot.nodes.find((node) => node.category_id === categoryId) ?? null)
      .filter((node): node is NonNullable<typeof node> => node !== null);
  const newlyAvailable = new Set(snapshot.newly_available_category_ids);

  return (
    <section
      aria-label="Interview categories"
      style={{
        background: 'var(--color-surface)',
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        padding: '14px 16px',
        flexShrink: 0,
        boxShadow: 'var(--shadow-sm)',
      }}
    >
      <header style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '12px', flexWrap: 'wrap' }}>
          <span
            style={{
              fontSize: '11px',
              fontWeight: 700,
              letterSpacing: '0.08em',
              textTransform: 'uppercase',
              color: 'var(--color-primary)',
            }}
          >
            {currentTitle}
          </span>

          {!atMainScreen && (
            <button
              type="button"
              onClick={onBack}
              disabled={disabled}
              style={{
                background: 'var(--color-surface-2)',
                boxShadow: 'inset 0 0 0 1px var(--color-divider)',
                borderRadius: '999px',
                color: 'var(--color-text)',
                fontSize: '11px',
                fontFamily: 'inherit',
                padding: '5px 10px',
                cursor: disabled ? 'not-allowed' : 'pointer',
              }}
            >
              Back to categories
            </button>
          )}
        </div>

        {snapshot.active_category_path.length > 0 && (
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
            {snapshot.active_category_path.map((entry) => (
              <span
                key={entry.category_id}
                style={{
                  fontSize: '10px',
                  color: 'var(--color-text-muted)',
                  borderRadius: '999px',
                  padding: '2px 8px',
                  background: 'var(--color-surface-offset)',
                }}
              >
                {entry.title}
              </span>
            ))}
          </div>
        )}
      </header>

      {atMainScreen && snapshot.build_readiness_message && (
        <div
          style={{
            borderRadius: '12px',
            padding: '12px 14px',
            background: snapshot.build_ready ? 'rgba(55, 208, 120, 0.08)' : 'var(--color-surface-2)',
            color: 'var(--color-text)',
            fontSize: '12px',
            lineHeight: 1.5,
          }}
        >
          {snapshot.build_readiness_message}
        </div>
      )}

      {snapshot.newly_available_category_ids.length > 0 && (
        <div
          style={{
            color: 'var(--color-text-muted)',
            fontSize: '11px',
            letterSpacing: '0.03em',
            textTransform: 'uppercase',
          }}
        >
          {snapshot.newly_available_category_ids.length === 1
            ? '1 new category opened up'
            : `${snapshot.newly_available_category_ids.length} new categories opened up`}
        </div>
      )}

      {visibleNodes.length > 0 ? (
        <div
          style={{
            display: 'grid',
            gap: '10px',
            gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
            maxHeight: '34vh',
            overflowY: 'auto',
          }}
        >
          {visibleNodes.map((node) => (
            <button
              key={node.category_id}
              type="button"
              onClick={() => onEnterCategory(node.category_id, snapshot.revision)}
              disabled={disabled}
              style={{
                textAlign: 'left',
                borderRadius: '14px',
                background: node.status === 'active'
                  ? 'var(--color-primary-highlight)'
                  : 'var(--color-surface-2)',
                color: 'var(--color-text)',
                padding: '14px',
                cursor: disabled ? 'not-allowed' : 'pointer',
                display: 'flex',
                flexDirection: 'column',
                gap: '6px',
                boxShadow: node.status === 'active'
                  ? 'var(--shadow-sm)'
                  : 'inset 0 0 0 1px var(--color-divider)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexWrap: 'wrap' }}>
                <span style={{ fontSize: '12px', fontWeight: 700 }}>
                  {node.title}
                </span>
                {newlyAvailable.has(node.category_id) && (
                  <span
                    style={{
                      fontSize: '9px',
                      fontWeight: 700,
                      letterSpacing: '0.06em',
                      textTransform: 'uppercase',
                      color: 'var(--color-primary)',
                    }}
                  >
                    New
                  </span>
                )}
              </div>
              <span style={{ fontSize: '11px', color: 'var(--color-text-muted)', lineHeight: 1.45 }}>
                {node.summary}
              </span>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', flexWrap: 'wrap' }}>
                <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                  {node.has_children
                    ? `${node.item_count_hint} subcategories`
                    : node.has_prompt_ready
                      ? 'Questions ready'
                      : 'No questions ready'}
                </span>
                <span style={{ fontSize: '10px', color: 'var(--color-text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                  {node.status}
                </span>
              </div>
            </button>
          ))}
        </div>
      ) : (
        <div
          style={{
            borderRadius: '12px',
            padding: '12px 14px',
            color: 'var(--color-text-muted)',
            fontSize: '12px',
            background: 'var(--color-surface-2)',
          }}
        >
          {snapshot.build_ready
            ? 'No more category branches are required. You can return to the main screen and start building.'
            : snapshot.build_readiness_message || 'This branch has no more subcategories right now. Go back to the main list or wait for follow-up questions to refresh the map.'}
        </div>
      )}

      {atMainScreen && (
        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <button
            type="button"
            onClick={onDone}
            disabled={disabled || !snapshot.build_ready}
            style={{
              background: snapshot.build_ready ? 'var(--color-success)' : 'transparent',
              boxShadow: snapshot.build_ready
                ? 'var(--shadow-sm)'
                : 'inset 0 0 0 1px var(--color-divider)',
              borderRadius: '10px',
              color: snapshot.build_ready ? 'var(--color-bg)' : 'var(--color-text-muted)',
              fontSize: '12px',
              fontWeight: 700,
              fontFamily: 'inherit',
              padding: '7px 14px',
              cursor: disabled || !snapshot.build_ready ? 'not-allowed' : 'pointer',
            }}
          >
            Start building
          </button>
        </div>
      )}
    </section>
  );
}
