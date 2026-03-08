import { useAuth0 } from '@auth0/auth0-react';
import { Navigate } from 'react-router-dom';
import LoginPage from '../pages/LoginPage.tsx';
import HomeHubPage from '../pages/HomeHubPage.tsx';

// ─── Auth0 callback handler ───────────────────────────────────────────────────
export function CallbackPageAuth0() {
  const { isLoading, error } = useAuth0();

  if (error) {
    return (
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        height: '100vh', background: '#0a0a0f', color: '#ff4444',
        fontFamily: 'monospace', fontSize: '13px',
      }}>
        Auth error: {error.message}
      </div>
    );
  }

  if (isLoading) {
    return (
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        height: '100vh', background: '#0a0a0f', color: '#8888a0',
        fontFamily: 'monospace', fontSize: '13px',
      }}>
        completing authentication…
      </div>
    );
  }

  return <Navigate to="/" replace />;
}

// ─── Root page (Auth0 mode) ──────────────────────────────────────────────────
export function RootPageAuth0() {
  const { isAuthenticated, isLoading } = useAuth0();

  if (isLoading) {
    return (
      <div style={{
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        height: '100vh', background: '#0a0a0f', color: '#8888a0',
        fontFamily: 'monospace', fontSize: '13px',
      }}>
        loading…
      </div>
    );
  }

  return isAuthenticated ? <HomeHubPage /> : <LoginPage />;
}
