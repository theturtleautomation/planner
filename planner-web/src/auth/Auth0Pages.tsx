import { useAuth0 } from '@auth0/auth0-react';
import { Navigate } from 'react-router-dom';
import EntryShell from '../components/EntryShell.tsx';
import LoginPage from '../pages/LoginPage.tsx';
import HomeHubPage from '../pages/HomeHubPage.tsx';

// ─── Auth0 callback handler ───────────────────────────────────────────────────
export function CallbackPageAuth0() {
  const { isLoading, error } = useAuth0();

  if (error) {
    return (
      <EntryShell
        badge="Auth0"
        kicker="Authentication"
        title="Authentication could not be completed"
        description={error.message}
        actionLabel="Return to sign in"
        onAction={() => {
          window.location.assign('/');
        }}
        note="Planner could not finish the callback flow. Returning to the entry surface will restart sign-in."
        tone="error"
      />
    );
  }

  if (isLoading) {
    return (
      <EntryShell
        badge="Auth0"
        kicker="Authentication"
        title="Completing sign-in"
        description="Planner is finishing the authentication handshake and preparing your entry surface."
        note="This should only take a moment."
      />
    );
  }

  return <Navigate to="/" replace />;
}

// ─── Root page (Auth0 mode) ──────────────────────────────────────────────────
export function RootPageAuth0() {
  const { isAuthenticated, isLoading } = useAuth0();

  if (isLoading) {
    return (
      <EntryShell
        badge="Auth0"
        kicker="Authentication"
        title="Loading Planner"
        description="Checking your sign-in state and preparing the project-first workspace."
        note="Planner will route you to the correct entry surface automatically."
      />
    );
  }

  return isAuthenticated ? <HomeHubPage /> : <LoginPage />;
}
