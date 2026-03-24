import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { useAuth0 } from '@auth0/auth0-react';
import { CallbackPageAuth0, RootPageAuth0 } from '../Auth0Pages';

vi.mock('../../pages/LoginPage.tsx', () => ({
  default: () => <div>Login page stub</div>,
}));

vi.mock('../../pages/HomeHubPage.tsx', () => ({
  default: () => <div>Home hub stub</div>,
}));

function mockAuth0State(overrides: Partial<ReturnType<typeof useAuth0>> = {}) {
  vi.mocked(useAuth0).mockReturnValue({
    isAuthenticated: false,
    isLoading: false,
    user: undefined,
    error: undefined,
    loginWithRedirect: vi.fn(),
    logout: vi.fn(),
    getAccessTokenSilently: vi.fn().mockResolvedValue('mock-token'),
    getIdTokenClaims: vi.fn(),
    loginWithPopup: vi.fn(),
    handleRedirectCallback: vi.fn(),
    ...overrides,
  } as ReturnType<typeof useAuth0>);
}

describe('Auth0Pages', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the root entry loading shell while Auth0 state is resolving', () => {
    mockAuth0State({ isLoading: true });

    render(
      <MemoryRouter>
        <RootPageAuth0 />
      </MemoryRouter>,
    );

    expect(screen.getByRole('heading', { name: /loading planner/i })).toBeInTheDocument();
    expect(screen.getByText(/checking your sign-in state/i)).toBeInTheDocument();
  });

  it('routes anonymous Auth0 users to the shared login entry shell', () => {
    mockAuth0State({ isAuthenticated: false, isLoading: false });

    render(
      <MemoryRouter>
        <RootPageAuth0 />
      </MemoryRouter>,
    );

    expect(screen.getByText('Login page stub')).toBeInTheDocument();
  });

  it('routes authenticated Auth0 users to the home hub', () => {
    mockAuth0State({ isAuthenticated: true, isLoading: false });

    render(
      <MemoryRouter>
        <RootPageAuth0 />
      </MemoryRouter>,
    );

    expect(screen.getByText('Home hub stub')).toBeInTheDocument();
  });

  it('renders the callback loading shell while the authentication handshake is completing', () => {
    mockAuth0State({ isLoading: true });

    render(
      <MemoryRouter>
        <CallbackPageAuth0 />
      </MemoryRouter>,
    );

    expect(screen.getByRole('heading', { name: /completing sign-in/i })).toBeInTheDocument();
    expect(screen.getByText(/finishing the authentication handshake/i)).toBeInTheDocument();
  });

  it('renders a callback error shell with an explicit return-to-sign-in action', async () => {
    const user = userEvent.setup();
    mockAuth0State({
      isLoading: false,
      error: new Error('State mismatch during callback'),
    });

    render(
      <MemoryRouter>
        <CallbackPageAuth0 />
      </MemoryRouter>,
    );

    expect(screen.getByRole('heading', { name: /authentication could not be completed/i })).toBeInTheDocument();
    expect(screen.getByText(/state mismatch during callback/i)).toBeInTheDocument();
    expect(screen.getByText(/returning to the entry surface will restart sign-in/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /return to sign in/i }));
  });

  it('redirects completed callbacks back to the root entry route', () => {
    mockAuth0State({ isLoading: false, error: undefined });

    render(
      <MemoryRouter initialEntries={['/callback']}>
        <Routes>
          <Route path="/callback" element={<CallbackPageAuth0 />} />
          <Route path="/" element={<div>Root entry landing</div>} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText('Root entry landing')).toBeInTheDocument();
  });
});
