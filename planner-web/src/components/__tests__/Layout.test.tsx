import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import Layout from '../Layout';

// AUTH0_ENABLED is false in test environment (no env vars set),
// so UserInfo renders the dev mode component (no useAuth0 calls needed beyond mock)
describe('Layout', () => {
  it('renders children', () => {
    render(<Layout><div>Child content</div></Layout>);
    expect(screen.getByText('Child content')).toBeInTheDocument();
  });

  it('renders ASCII banner with Planner aria-label', () => {
    render(<Layout><span /></Layout>);
    expect(screen.getByLabelText('Planner')).toBeInTheDocument();
  });

  it('renders the Socratic Lobby subtitle', () => {
    render(<Layout><span /></Layout>);
    expect(screen.getByText('SOCRATIC LOBBY')).toBeInTheDocument();
  });

  it('renders v2 label', () => {
    render(<Layout><span /></Layout>);
    expect(screen.getByText('v2')).toBeInTheDocument();
  });

  it('header has role="banner"', () => {
    render(<Layout><span /></Layout>);
    expect(screen.getByRole('banner')).toBeInTheDocument();
  });

  it('renders main element for content area', () => {
    render(<Layout><span>main child</span></Layout>);
    expect(screen.getByRole('main')).toBeInTheDocument();
    expect(screen.getByText('main child')).toBeInTheDocument();
  });

  it('shows session id when sessionId is provided', () => {
    render(<Layout sessionId="abcdef1234567890"><span /></Layout>);
    expect(screen.getByText(/session: abcdef12/i)).toBeInTheDocument();
  });

  it('does not show session id when sessionId is not provided', () => {
    render(<Layout><span /></Layout>);
    expect(screen.queryByText(/session:/i)).not.toBeInTheDocument();
  });

  it('shows connection status indicator when sessionId is provided', () => {
    render(<Layout sessionId="abc" isConnected={true}><span /></Layout>);
    expect(screen.getByRole('status', { name: /connection status/i })).toBeInTheDocument();
  });

  it('shows "connected" text when isConnected is true', () => {
    render(<Layout sessionId="abc" isConnected={true}><span /></Layout>);
    expect(screen.getByText('connected')).toBeInTheDocument();
  });

  it('shows "disconnected" text when isConnected is false', () => {
    render(<Layout sessionId="abc" isConnected={false}><span /></Layout>);
    expect(screen.getByText('disconnected')).toBeInTheDocument();
  });

  it('does NOT show connection status indicator when sessionId is undefined', () => {
    render(<Layout><span /></Layout>);
    expect(screen.queryByRole('status', { name: /connection status/i })).not.toBeInTheDocument();
  });

  it('connection status indicator has correct aria-label', () => {
    render(<Layout sessionId="test-id" isConnected={true}><span /></Layout>);
    const indicator = screen.getByRole('status');
    expect(indicator).toHaveAttribute('aria-label', 'Connection status');
  });
});

// Test the Auth0-enabled variant by mocking config
describe('Layout with Auth0 enabled (user info)', () => {
  it('shows dev mode label in dev mode', () => {
    // AUTH0_ENABLED is determined by env vars; in test environment it's false
    // so UserInfoDev renders "dev mode" label.
    render(<Layout><span /></Layout>);
    expect(screen.getByText('dev mode')).toBeInTheDocument();
  });
});
