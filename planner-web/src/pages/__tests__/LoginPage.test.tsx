import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import LoginPage from '../LoginPage';
import { useAuth0 } from '@auth0/auth0-react';

// In tests, AUTH0_ENABLED=false (no VITE_AUTH0_DOMAIN/CLIENT_ID env vars set).
// This means LoginPageDev renders, which navigates on button click.

describe('LoginPage (dev mode - AUTH0_ENABLED=false)', () => {
  it('renders the Planner entry title', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByRole('heading', { name: /enter planner/i })).toBeInTheDocument();
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
    expect(screen.getByText(/auth0 is not configured/i)).toBeInTheDocument();
  });

  it('renders project-first entry pills', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/project-centered workspace/i)).toBeInTheDocument();
    expect(screen.getByText(/socratic planning flow/i)).toBeInTheDocument();
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

  it('renders the project-first description text', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/start from project context/i)).toBeInTheDocument();
  });

  it('renders the development-mode note', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/local development mode/i)).toBeInTheDocument();
  });

  it('button text says "Enter Planner" when AUTH0_ENABLED is false', () => {
    render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );
    expect(screen.getByRole('button')).toHaveTextContent(/enter planner/i);
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
