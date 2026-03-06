import { lazy, Suspense } from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { AUTH0_ENABLED } from './config.ts';
import Dashboard from './pages/Dashboard.tsx';
import SessionPage from './pages/SessionPage.tsx';
import AdminPage from './pages/AdminPage.tsx';

// Blueprint page is heavy (D3) — lazy-load it
const BlueprintPage = lazy(() => import('./pages/BlueprintPage.tsx'));
const KnowledgeLibraryPage = lazy(() => import('./pages/KnowledgeLibraryPage.tsx'));
import ProtectedRoute from './auth/ProtectedRoute.tsx';

// ─── Auth0-dependent pages ───────────────────────────────────────────────────
// Lazy-loaded at module level so the @auth0/auth0-react module is never
// imported when AUTH0_ENABLED is false. This avoids React 19 context/hook
// edge cases when the Auth0Provider is absent from the tree.
const LazyCallbackPageAuth0 = lazy(() =>
  import('./auth/Auth0Pages.tsx').then((m) => ({ default: m.CallbackPageAuth0 }))
);
const LazyRootPageAuth0 = lazy(() =>
  import('./auth/Auth0Pages.tsx').then((m) => ({ default: m.RootPageAuth0 }))
);

function AuthLoadingFallback() {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      height: '100vh', background: 'var(--color-bg, #111110)', color: 'var(--color-text-muted, #8a8987)',
      fontFamily: "'Inter', system-ui, sans-serif", fontSize: '13px',
    }}>
      loading…
    </div>
  );
}

function CallbackPage() {
  if (AUTH0_ENABLED) {
    return (
      <Suspense fallback={<AuthLoadingFallback />}>
        <LazyCallbackPageAuth0 />
      </Suspense>
    );
  }
  return <Navigate to="/" replace />;
}

function RootPage() {
  if (AUTH0_ENABLED) {
    return (
      <Suspense fallback={<AuthLoadingFallback />}>
        <LazyRootPageAuth0 />
      </Suspense>
    );
  }
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
      <Route
        path="/blueprint"
        element={
          <ProtectedRoute>
            <Suspense fallback={<AuthLoadingFallback />}>
              <BlueprintPage />
            </Suspense>
          </ProtectedRoute>
        }
      />
      <Route
        path="/knowledge"
        element={
          <ProtectedRoute>
            <Suspense fallback={<AuthLoadingFallback />}>
              <KnowledgeLibraryPage />
            </Suspense>
          </ProtectedRoute>
        }
      />
      {/* Catch-all */}
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
