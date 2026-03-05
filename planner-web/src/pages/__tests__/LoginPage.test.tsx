import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import LoginPage from '../LoginPage';
import { useAuth0 } from '@auth0/auth0-react';

// In tests, AUTH0_ENABLED=false (no VITE_AUTH0_DOMAIN/CLIENT_ID env vars set).
// This means LoginPageDev renders, which navigates on button click.

describe('LoginPage (dev mode - AUTH0_ENABLED=false)', () => {
  it('renders the ASCII PLANNER banner', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    // The banner is an ASCII art <pre> with aria-label="Planner"
    expect(screen.getByLabelText('Planner')).toBeInTheDocument();
  });

  it('renders a button', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button')).toBeInTheDocument();
  });

  it('shows the dev mode notice', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/auth0 not configured/i)).toBeInTheDocument();
  });

  it('renders feature list items', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/real-time pipeline visualization/i)).toBeInTheDocument();
    expect(screen.getByText(/socratic dialogue/i)).toBeInTheDocument();
  });

  it('button is clickable (not disabled)', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    const button = screen.getByRole('button');
    expect(button).not.toBeDisabled();
  });

  it('renders the terminal window title bar text', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/planner-v2.*socratic-lobby/i)).toBeInTheDocument();
  });

  it('renders planning description text', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/socratic AI planning tool/i)).toBeInTheDocument();
  });

  it('button text says "enter  (dev mode)" when AUTH0_ENABLED is false', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button')).toHaveTextContent(/enter.*dev mode/i);
  });

  it('renders the live WebSocket updates feature', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/live WebSocket updates/i)).toBeInTheDocument();
  });

  it('renders the 12-stage pipeline feature', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/12-stage compilation pipeline/i)).toBeInTheDocument();
  });
});

describe('LoginPage Auth0 loginWithRedirect behavior', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('useAuth0 mock is accessible and returns loginWithRedirect', () => {
    // Verify the global Auth0 mock is set up correctly
    const auth0 = useAuth0();
    expect(auth0.loginWithRedirect).toBeDefined();
    expect(typeof auth0.loginWithRedirect).toBe('function');
  });

  it('clicking login button does not throw an error', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    const button = screen.getByRole('button');
    // In dev mode this navigates; just verify no error is thrown
    await expect(user.click(button)).resolves.not.toThrow();
  });
});
