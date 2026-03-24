import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SessionPulseBar from '../SessionPulseBar.tsx';
import type { SocraticWorkspaceSnapshot } from '../../types.ts';

function makeWorkspace(
  overrides: Partial<SocraticWorkspaceSnapshot> = {},
): SocraticWorkspaceSnapshot {
  return {
    focused_category_id: null,
    branch_notice: null,
    category_snapshot: {
      revision: 'category-1',
      root_category_ids: ['root-verification'],
      nodes: [],
      active_category_path: [],
      newly_available_category_ids: [],
      build_ready: false,
      build_readiness_message: 'Build is blocked until verification is complete.',
    },
    groups: [
      {
        category_id: 'category-verification-platform',
        title: 'Verify Platform',
        summary: 'Current assumption: "Web application" (50% confidence).',
        status: 'pending',
        question_count: 1,
        preview_items: [],
        is_focused: false,
        is_new: true,
      },
    ],
    ...overrides,
  };
}

describe('SessionPulseBar', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows ready state when the workspace is generated and waiting for thread selection', () => {
    render(
      <SessionPulseBar
        sessionTitle="personal calendar app with task tracking"
        currentStep="socratic.workspace.generated"
        events={[]}
        isError={false}
        errorMessage={null}
        workspace={makeWorkspace()}
        unreadEventCount={0}
        hasDraft={false}
        isContextShelfOpen={false}
        onToggleContextShelf={vi.fn()}
      />,
    );

    expect(screen.getByText('Ready')).toBeInTheDocument();
    expect(screen.getByText(/Ready to choose a thread/i)).toBeInTheDocument();
  });
});
