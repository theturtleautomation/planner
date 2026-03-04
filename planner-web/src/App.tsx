import { Routes, Route, Navigate } from 'react-router-dom';
import { useAuth0 } from '@auth0/auth0-react';
import { AUTH0_ENABLED } from './config.ts';
import LoginPage from './pages/LoginPage.tsx';
import Dashboard from './pages/Dashboard.tsx';
import SessionPage from './pages/SessionPage.tsx';
import AdminPage from './pages/AdminPage.tsx';
import ProtectedRoute from './auth/ProtectedRoute.tsx';

// ─── Auth0 callback handler ───────────────────────────────────────────────────
function CallbackPageAuth0() {
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

function CallbackPage() {
  if (AUTH0_ENABLED) return <CallbackPageAuth0 />;
  return <Navigate to="/" replace />;
}

// ─── Root page ────────────────────────────────────────────────────────────────
function RootPageAuth0() {
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

  return isAuthenticated ? <Dashboard /> : <LoginPage />;
}

function RootPage() {
  if (AUTH0_ENABLED) return <RootPageAuth0 />;
  return <Dashboard />;
}

// ─── App ──────────────────────────────────────────────────────────────────────
export default function App() {
  return (
    <Routes>
      <Route path="/" element={<RootPage />} />
      <Route path="/callback" element={<CallbackPage />} />
      <Route
        path="/session/new"
        element={
          <ProtectedRoute>
            <SessionPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/session/:id"
        element={
          <ProtectedRoute>
            <SessionPage />
          </ProtectedRoute>
        }
      />
      <Route path="/admin" element={<AdminPage />} />
      {/* Catch-all */}
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
