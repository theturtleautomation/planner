import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import Layout from '../Layout';

// Helper: wraps Layout with MemoryRouter so useLocation/Link work
function renderLayout(props: { sessionId?: string; isConnected?: boolean; children?: React.ReactNode }, route = '/') {
  return render(
    <MemoryRouter initialEntries={[route]}>
      <Layout {...props}>{props.children ?? <span />}</Layout>
    </MemoryRouter>
  );
}

describe('Layout', () => {
  it('renders children', () => {
    renderLayout({ children: <div>Child content</div> });
    expect(screen.getByText('Child content')).toBeInTheDocument();
  });

  it('renders Planner logo with aria-label', () => {
    renderLayout({});
    expect(screen.getByLabelText('Planner logo')).toBeInTheDocument();
  });

  it('renders Planner wordmark', () => {
    renderLayout({});
    expect(screen.getByText('Planner')).toBeInTheDocument();
  });

  it('renders sidebar navigation links', () => {
    renderLayout({});
    expect(screen.getByText('Sessions')).toBeInTheDocument();
    expect(screen.getByText('Blueprint')).toBeInTheDocument();
    expect(screen.getByText('Admin')).toBeInTheDocument();
  });

  it('renders main element for content area', () => {
    renderLayout({ children: <span>main child</span> });
    expect(screen.getByRole('main')).toBeInTheDocument();
    expect(screen.getByText('main child')).toBeInTheDocument();
  });

  it('highlights active nav item based on current route', () => {
    renderLayout({}, '/blueprint');
    const blueprintLink = screen.getByText('Blueprint').closest('a');
    expect(blueprintLink?.className).toContain('active');
  });

  it('highlights Sessions for session sub-routes', () => {
    renderLayout({}, '/session/abc123');
    const sessionsLink = screen.getByText('Sessions').closest('a');
    expect(sessionsLink?.className).toContain('active');
  });

  it('shows connection status when sessionId is provided and connected', () => {
    renderLayout({ sessionId: 'abc', isConnected: true });
    expect(screen.getByText('connected')).toBeInTheDocument();
  });

  it('shows disconnected status when isConnected is false', () => {
    renderLayout({ sessionId: 'abc', isConnected: false });
    expect(screen.getByText('disconnected')).toBeInTheDocument();
  });

  it('does NOT show connection status when sessionId is undefined', () => {
    renderLayout({});
    expect(screen.queryByText('connected')).not.toBeInTheDocument();
    expect(screen.queryByText('disconnected')).not.toBeInTheDocument();
  });

  it('renders theme toggle button', () => {
    renderLayout({});
    // Theme toggle has aria-label "Switch to light mode" or "Switch to dark mode"
    expect(screen.getByRole('button', { name: /switch to/i })).toBeInTheDocument();
  });
});

// Test the Auth0-enabled variant by mocking config
describe('Layout with Auth0 disabled (dev mode)', () => {
  it('shows dev label in dev mode', () => {
    // AUTH0_ENABLED is determined by env vars; in test environment it's false
    // so UserInfoDev renders "dev" label.
    renderLayout({});
    expect(screen.getByText('dev')).toBeInTheDocument();
  });
});
