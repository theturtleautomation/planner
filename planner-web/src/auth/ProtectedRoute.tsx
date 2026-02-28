import { useAuth0 } from '@auth0/auth0-react';
import { Navigate, useLocation } from 'react-router-dom';
import type { ReactNode } from 'react';
import { AUTH0_ENABLED } from '../config.ts';

interface Props {
  children: ReactNode;
}

// ─── Dev mode: no auth ────────────────────────────────────────────────────────
function DevRoute({ children }: Props) {
  return <>{children}</>;
}

// ─── Auth0 mode: requires login ───────────────────────────────────────────────
function Auth0Route({ children }: Props) {
  const { isAuthenticated, isLoading } = useAuth0();
  const location = useLocation();

  if (isLoading) {
    return (
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100vh',
        background: '#0a0a0f',
        color: '#8888a0',
        fontFamily: 'monospace',
        fontSize: '13px',
      }}>
        authenticating…
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/" state={{ from: location }} replace />;
  }

  return <>{children}</>;
}

// ─── Export ───────────────────────────────────────────────────────────────────
export default function ProtectedRoute({ children }: Props) {
  if (AUTH0_ENABLED) {
    return <Auth0Route>{children}</Auth0Route>;
  }
  return <DevRoute>{children}</DevRoute>;
}
