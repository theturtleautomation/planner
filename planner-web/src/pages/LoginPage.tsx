import { useAuth0 } from '@auth0/auth0-react';
import { useNavigate } from 'react-router-dom';
import { AUTH0_ENABLED } from '../config.ts';
import EntryShell from '../components/EntryShell.tsx';

// ─── Auth0 version ────────────────────────────────────────────────────────────
function LoginPageAuth0() {
  const { loginWithRedirect } = useAuth0();

  const handleLogin = (): void => {
    void loginWithRedirect();
  };

  return <LoginView onLogin={handleLogin} />;
}

// ─── Dev mode version ─────────────────────────────────────────────────────────
function LoginPageDev() {
  const navigate = useNavigate();

  const handleEnter = (): void => {
    void navigate('/');
  };

  return <LoginView onLogin={handleEnter} />;
}

// ─── Router ───────────────────────────────────────────────────────────────────
export default function LoginPage() {
  if (AUTH0_ENABLED) {
    return <LoginPageAuth0 />;
  }
  return <LoginPageDev />;
}

// ─── Shared view ─────────────────────────────────────────────────────────────
function LoginView({ onLogin }: { onLogin: () => void }) {
  return (
    <EntryShell
      badge={AUTH0_ENABLED ? 'Auth0' : 'Dev mode'}
      kicker="Project-first planning"
      title="Enter Planner"
      description="Start from project context, move into sessions and knowledge deliberately, and keep operational tools in supporting roles."
      actionLabel={AUTH0_ENABLED ? 'Sign In' : 'Enter Planner'}
      onAction={onLogin}
      details={(
        <div className="entry-pill-list">
          <span className="entry-pill">Project-centered workspace</span>
          <span className="entry-pill">Socratic planning flow</span>
          <span className="entry-pill">Knowledge and blueprint surfaces</span>
        </div>
      )}
      note={AUTH0_ENABLED
        ? 'Secure sign-in uses your configured Auth0 tenant.'
        : 'Auth0 is not configured. Planner will continue in local development mode.'}
    />
  );
}
