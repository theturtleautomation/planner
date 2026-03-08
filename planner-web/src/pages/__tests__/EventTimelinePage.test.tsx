import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import EventTimelinePage from '../EventTimelinePage.tsx';
import type { BlueprintEventPayload } from '../../types/blueprint.ts';

const mockListBlueprintEvents = vi.fn();
const mockListBlueprintHistory = vi.fn();
const mockCreateBlueprintSnapshot = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    listBlueprintEvents: mockListBlueprintEvents,
    listBlueprintHistory: mockListBlueprintHistory,
    createBlueprintSnapshot: mockCreateBlueprintSnapshot,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => vi.fn().mockResolvedValue('mock-token')),
}));

vi.mock('../../components/Layout.tsx', () => ({
  default: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

function makeScopedUpdateEvent(): BlueprintEventPayload {
  return {
    event_type: 'node_updated',
    summary: "Updated component 'comp-task-widget'",
    timestamp: '2026-03-07T11:00:00Z',
    data: {
      before: {
        node_type: 'component',
        id: 'comp-task-widget',
        name: 'Task Tracker Widget',
        tags: ['task-tracker'],
        scope: {
          scope_class: 'project_contextual',
          project: {
            project_id: 'proj-task-tracker',
            project_name: 'Task Tracker',
          },
          secondary: {
            feature: 'task_management',
            widget: 'task-tracker',
            artifact: 'widgets/task-tracker.tsx',
            component: 'task-widget',
          },
          is_shared: false,
          lifecycle: 'active',
        },
      },
      after: {
        node_type: 'component',
        id: 'comp-task-widget',
        name: 'Task Tracker Widget',
        tags: ['task-tracker'],
        scope: {
          scope_class: 'project_contextual',
          project: {
            project_id: 'proj-task-tracker',
            project_name: 'Task Tracker',
          },
          secondary: {
            feature: 'task_management',
            widget: 'task-tracker',
            artifact: 'widgets/task-tracker.tsx',
            component: 'task-widget',
          },
          is_shared: false,
          lifecycle: 'active',
        },
      },
    },
  };
}

function makeExportEvent(): BlueprintEventPayload {
  return {
    event_type: 'export_recorded',
    summary: 'Recorded scoped view export',
    timestamp: '2026-03-07T12:00:00Z',
    data: {
      kind: 'scoped_view',
      export_id: 'exp-task-tracker',
      project_id: 'proj-task-tracker',
      project_name: 'Task Tracker',
      node_count: 4,
      edge_count: 2,
    },
  };
}

function LocationSnapshot() {
  const location = useLocation();
  const params = Object.fromEntries(new URLSearchParams(location.search).entries());
  return (
    <div>
      <div data-testid="location-path">{location.pathname}</div>
      <pre data-testid="location-params">{JSON.stringify(params)}</pre>
    </div>
  );
}

describe('EventTimelinePage related knowledge links', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListBlueprintHistory.mockResolvedValue({ snapshots: [] });
    mockCreateBlueprintSnapshot.mockResolvedValue({ timestamp: '2026-03-07T12:00:00Z', filename: 'snapshot.json' });
  });

  it('opens project-scoped related knowledge from node events', async () => {
    const user = userEvent.setup();
    mockListBlueprintEvents.mockResolvedValue({
      events: [makeScopedUpdateEvent()],
      total: 1,
    });

    render(
      <MemoryRouter initialEntries={['/events']}>
        <Routes>
          <Route path="/events" element={<EventTimelinePage />} />
          <Route path="/knowledge/projects/:projectId" element={<LocationSnapshot />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockListBlueprintEvents).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: /view related knowledge/i }));

    expect(await screen.findByTestId('location-path')).toHaveTextContent('/knowledge/projects/proj-task-tracker');
    const params = JSON.parse(screen.getByTestId('location-params').textContent ?? '{}') as Record<string, string>;
    expect(params.project).toBe('proj-task-tracker');
    expect(params.feature).toBe('task_management');
    expect(params.widget).toBe('task-tracker');
    expect(params.artifact).toBe('widgets/task-tracker.tsx');
    expect(params.component).toBe('task-widget');
    expect(params.from).toBe('/events');
    expect(params.from_label).toBe('Event Timeline');
  });

  it('opens project-scoped related knowledge from export events', async () => {
    const user = userEvent.setup();
    mockListBlueprintEvents.mockResolvedValue({
      events: [makeExportEvent()],
      total: 1,
    });

    render(
      <MemoryRouter initialEntries={['/events']}>
        <Routes>
          <Route path="/events" element={<EventTimelinePage />} />
          <Route path="/knowledge/projects/:projectId" element={<LocationSnapshot />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockListBlueprintEvents).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: /view related knowledge/i }));

    expect(await screen.findByTestId('location-path')).toHaveTextContent('/knowledge/projects/proj-task-tracker');
    const params = JSON.parse(screen.getByTestId('location-params').textContent ?? '{}') as Record<string, string>;
    expect(params.project).toBe('proj-task-tracker');
    expect(params.from).toBe('/events');
    expect(params.from_label).toBe('Event Timeline');
  });
});
