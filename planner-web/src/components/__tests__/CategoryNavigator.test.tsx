import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import CategoryNavigator from '../CategoryNavigator.tsx';
import type { SocraticCategorySnapshot } from '../../types.ts';

function makeSnapshot(overrides: Partial<SocraticCategorySnapshot> = {}): SocraticCategorySnapshot {
  return {
    revision: 'category-1',
    root_category_ids: ['root-discovery'],
    nodes: [
      {
        category_id: 'root-discovery',
        parent_category_id: null,
        title: 'Explore missing areas',
        summary: '1 area still needs discovery.',
        status: 'ready',
        depth: 0,
        mapped_dimensions: [],
        has_children: true,
        has_prompt_ready: false,
        item_count_hint: 1,
      },
      {
        category_id: 'root-discovery::dimension::goal',
        parent_category_id: 'root-discovery',
        title: 'Goal / Purpose',
        summary: '1 discovery branch under Goal / Purpose.',
        status: 'ready',
        depth: 1,
        mapped_dimensions: ['goal'],
        has_children: true,
        has_prompt_ready: false,
        item_count_hint: 1,
      },
      {
        category_id: 'category-goal-leaf',
        parent_category_id: 'root-discovery::dimension::goal',
        title: 'Explore Goal / Purpose',
        summary: 'Discover missing dimension Goal / Purpose.',
        status: 'ready',
        depth: 2,
        mapped_dimensions: ['goal'],
        has_children: false,
        has_prompt_ready: true,
        item_count_hint: 1,
      },
    ],
    active_category_path: [],
    newly_available_category_ids: [],
    build_ready: false,
    build_readiness_message: 'Build is blocked until 1 remaining area is explored.',
    ...overrides,
  };
}

describe('CategoryNavigator', () => {
  it('renders only the active branch children for deep category paths', () => {
    render(
      <CategoryNavigator
        snapshot={makeSnapshot({
          active_category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
          ],
        })}
        onEnterCategory={vi.fn()}
        onBack={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByRole('button', { name: /Goal \/ Purpose/i })).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /Explore missing areas/i }),
    ).not.toBeInTheDocument();
  });

  it('renders deep breadcrumbs and sends visible child selection', async () => {
    const user = userEvent.setup();
    const onEnterCategory = vi.fn();

    render(
      <CategoryNavigator
        snapshot={makeSnapshot({
          active_category_path: [
            { category_id: 'root-discovery', title: 'Explore missing areas' },
            { category_id: 'root-discovery::dimension::goal', title: 'Goal / Purpose' },
          ],
        })}
        onEnterCategory={onEnterCategory}
        onBack={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText('Explore missing areas')).toBeInTheDocument();
    expect(screen.getAllByText('Goal / Purpose').length).toBeGreaterThan(0);

    await user.click(screen.getByRole('button', { name: /Explore Goal \/ Purpose/i }));

    expect(onEnterCategory).toHaveBeenCalledWith('category-goal-leaf', 'category-1');
  });

  it('renders new-category and build-guidance copy from the snapshot', () => {
    render(
      <CategoryNavigator
        snapshot={makeSnapshot({
          newly_available_category_ids: ['root-discovery'],
          build_readiness_message: 'Build is blocked until 2 uncertain areas are verified.',
        })}
        onEnterCategory={vi.fn()}
        onBack={vi.fn()}
        onDone={vi.fn()}
      />,
    );

    expect(screen.getByText('1 new category opened up')).toBeInTheDocument();
    expect(screen.getByText('Build is blocked until 2 uncertain areas are verified.')).toBeInTheDocument();
    expect(screen.getByText('New')).toBeInTheDocument();
  });
});
