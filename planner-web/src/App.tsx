import { lazy, Suspense } from 'react';
import { Routes, Route, Navigate, useParams } from 'react-router-dom';
import { AUTH0_ENABLED } from './config.ts';
import Dashboard from './pages/Dashboard.tsx';
import SessionPage from './pages/SessionPage.tsx';
import AdminPage from './pages/AdminPage.tsx';
import HomeHubPage from './pages/HomeHubPage.tsx';
import ProjectsPage from './pages/ProjectsPage.tsx';
import ProjectSessionsPage from './pages/ProjectSessionsPage.tsx';

// Blueprint page is heavy (D3) — lazy-load it
const BlueprintPage = lazy(() => import('./pages/BlueprintPage.tsx'));
const KnowledgeLibraryPage = lazy(() => import('./pages/KnowledgeLibraryPage.tsx'));
const EventTimelinePage = lazy(() => import('./pages/EventTimelinePage.tsx'));
const DiscoveryPage = lazy(() => import('./pages/DiscoveryPage.tsx'));
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
  return <HomeHubPage />;
}

function ProjectRootRedirect() {
  const { projectSlug } = useParams<{ projectSlug: string }>();
  if (!projectSlug) return <Navigate to="/projects" replace />;
  return <Navigate to={`/projects/${encodeURIComponent(projectSlug)}/sessions`} replace />;
}

function ProjectKnowledgeRedirect() {
  const { projectSlug } = useParams<{ projectSlug: string }>();
  if (!projectSlug) return <Navigate to="/projects" replace />;
  return <Navigate to={`/knowledge/projects/${encodeURIComponent(projectSlug)}`} replace />;
}

function ProjectBlueprintRedirect() {
  const { projectSlug } = useParams<{ projectSlug: string }>();
  if (!projectSlug) return <Navigate to="/projects" replace />;
  return <Navigate to={`/blueprint?project_id=${encodeURIComponent(projectSlug)}`} replace />;
}

function ProjectEventsRedirect() {
  const { projectSlug } = useParams<{ projectSlug: string }>();
  if (!projectSlug) return <Navigate to="/projects" replace />;
  return <Navigate to="/events" replace />;
}

// ─── App ──────────────────────────────────────────────────────────────────────
export default function App() {
  return (
    <Routes>
      <Route path="/" element={<RootPage />} />
      <Route path="/callback" element={<CallbackPage />} />
      <Route
        path="/sessions"
        element={
          <ProtectedRoute>
            <Dashboard />
          </ProtectedRoute>
        }
      />
      <Route
        path="/projects"
        element={
          <ProtectedRoute>
            <ProjectsPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/projects/:projectSlug"
        element={
          <ProtectedRoute>
            <ProjectRootRedirect />
          </ProtectedRoute>
        }
      />
      <Route
        path="/projects/:projectSlug/sessions"
        element={
          <ProtectedRoute>
            <ProjectSessionsPage />
          </ProtectedRoute>
        }
      />
      <Route
        path="/projects/:projectSlug/knowledge"
        element={
          <ProtectedRoute>
            <ProjectKnowledgeRedirect />
          </ProtectedRoute>
        }
      />
      <Route
        path="/projects/:projectSlug/blueprint"
        element={
          <ProtectedRoute>
            <ProjectBlueprintRedirect />
          </ProtectedRoute>
        }
      />
      <Route
        path="/projects/:projectSlug/events"
        element={
          <ProtectedRoute>
            <ProjectEventsRedirect />
          </ProtectedRoute>
        }
      />
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
      <Route
        path="/knowledge/all"
        element={
          <ProtectedRoute>
            <Suspense fallback={<AuthLoadingFallback />}>
              <KnowledgeLibraryPage />
            </Suspense>
          </ProtectedRoute>
        }
      />
      <Route
        path="/knowledge/projects/:projectId"
        element={
          <ProtectedRoute>
            <Suspense fallback={<AuthLoadingFallback />}>
              <KnowledgeLibraryPage />
            </Suspense>
          </ProtectedRoute>
        }
      />
      <Route
        path="/events"
        element={
          <ProtectedRoute>
            <Suspense fallback={<AuthLoadingFallback />}>
              <EventTimelinePage />
            </Suspense>
          </ProtectedRoute>
        }
      />
      <Route
        path="/discovery"
        element={
          <ProtectedRoute>
            <Suspense fallback={<AuthLoadingFallback />}>
              <DiscoveryPage />
            </Suspense>
          </ProtectedRoute>
        }
      />
      {/* Catch-all */}
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
