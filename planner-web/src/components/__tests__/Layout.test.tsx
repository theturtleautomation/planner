import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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

  it('renders persistent shell navigation links', () => {
    renderLayout({});
    expect(screen.getByText('Home')).toBeInTheDocument();
    expect(screen.getByText('Projects')).toBeInTheDocument();
    expect(screen.getByText('Sessions')).toBeInTheDocument();
    expect(screen.getByText('Knowledge')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /more/i })).toBeInTheDocument();
    expect(screen.queryByText('Admin')).not.toBeInTheDocument();
  });

  it('renders main element for content area', () => {
    renderLayout({ children: <span>main child</span> });
    expect(screen.getByRole('main')).toBeInTheDocument();
    expect(screen.getByText('main child')).toBeInTheDocument();
  });

  it('highlights active nav item based on current route', () => {
    renderLayout({}, '/projects/acme/sessions');
    const projectsLink = screen.getByText('Projects').closest('a');
    expect(projectsLink?.className).toContain('active');
  });

  it('highlights Sessions for session sub-routes', () => {
    renderLayout({}, '/session/abc123');
    const sessionsLink = screen.getByText('Sessions').closest('a');
    expect(sessionsLink?.className).toContain('active');
  });

  it('renders theme toggle button', () => {
    renderLayout({});
    // Theme toggle has aria-label "Switch to light mode" or "Switch to dark mode"
    expect(screen.getByRole('button', { name: /switch to/i })).toBeInTheDocument();
  });

  it('reveals utility routes from the More control', async () => {
    const user = userEvent.setup();
    renderLayout({});
    await user.click(screen.getByRole('button', { name: /more/i }));
    expect(screen.getByText('Admin')).toBeInTheDocument();
    expect(screen.getByText('Events')).toBeInTheDocument();
    expect(screen.getByText('Blueprint')).toBeInTheDocument();
    expect(screen.getByText('Discovery')).toBeInTheDocument();
  });

  it('expands utility routes when a utility destination is active', () => {
    renderLayout({}, '/admin');
    const moreButton = screen.getByRole('button', { name: /more/i });
    expect(moreButton).toHaveAttribute('aria-expanded', 'true');
    const adminLink = screen.getByText('Admin').closest('a');
    expect(adminLink?.className).toContain('active');
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
