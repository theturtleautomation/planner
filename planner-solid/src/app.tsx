import { MetaProvider, Title } from "@solidjs/meta";
import { A, Route, Router, useLocation } from "@solidjs/router";
import type { JSX } from "solid-js";
import { Suspense, createMemo, lazy } from "solid-js";
import "./app.css";
import { FrontendMockBadge } from "./components/app/FrontendMockBadge";
import {
  getFrontendMockBadgeCopy,
  getFrontendMockScenarioKey,
  isFrontendMockEnabled,
  setMockRuntimeLocationSearch,
  withFrontendMockSearch,
} from "./lib/mock/runtime";

const HomePage = lazy(() => import("./routes/index"));
const AdminPage = lazy(() => import("./routes/admin/index"));
const BlueprintPage = lazy(() => import("./routes/blueprint/index"));
const DiscoveryPage = lazy(() => import("./routes/discovery/index"));
const EventsPage = lazy(() => import("./routes/events/index"));
const KnowledgePage = lazy(() => import("./routes/knowledge/index"));
const ProjectsPage = lazy(() => import("./routes/projects/index"));
const ProjectImportReviewPage = lazy(() => import("./routes/projects/[projectSlug]/import"));
const NewProjectPage = lazy(() => import("./routes/projects/new"));
const ProjectWorkspacePage = lazy(() => import("./routes/projects/[projectSlug]"));
const SessionsPage = lazy(() => import("./routes/sessions/index"));
const NewSessionPage = lazy(() => import("./routes/sessions/new"));
const SessionWorkspacePage = lazy(() => import("./routes/sessions/[sessionId]"));
const NotFoundPage = lazy(() => import("./routes/[...404]"));

function AppShell(props: { children: JSX.Element }) {
  const location = useLocation();
  setMockRuntimeLocationSearch(location.search);
  const mockLabel = createMemo(() => getFrontendMockBadgeCopy(getFrontendMockScenarioKey()));

  return (
    <MetaProvider>
      <Title>Planner</Title>
      <div class="app-shell">
        <header class="app-header">
          <div class="app-brand">
            <span class="app-brand-mark">Planner</span>
            <span class="app-brand-copy">Local-first project analysis and build workspace</span>
            <FrontendMockBadge active={isFrontendMockEnabled()} label={mockLabel()} />
          </div>
          <nav class="app-nav" aria-label="Primary">
            <A href={withFrontendMockSearch("/")} end activeClass="is-active">
              Home
            </A>
            <A href={withFrontendMockSearch("/knowledge")} activeClass="is-active">
              Knowledge
            </A>
            <A href={withFrontendMockSearch("/blueprint")} activeClass="is-active">
              Blueprint
            </A>
            <A href={withFrontendMockSearch("/discovery")} activeClass="is-active">
              Discovery
            </A>
            <A href={withFrontendMockSearch("/events")} activeClass="is-active">
              Events
            </A>
            <A href={withFrontendMockSearch("/admin")} activeClass="is-active">
              Admin
            </A>
            <A href={withFrontendMockSearch("/sessions")} activeClass="is-active">
              Sessions
            </A>
          </nav>
        </header>
        <main class="app-main">
          <Suspense>{props.children}</Suspense>
        </main>
      </div>
    </MetaProvider>
  );
}

export default function App() {
  return (
    <Router root={props => <AppShell>{props.children}</AppShell>}>
      <Route path="/" component={HomePage} />
      <Route path="/admin" component={AdminPage} />
      <Route path="/blueprint" component={BlueprintPage} />
      <Route path="/discovery" component={DiscoveryPage} />
      <Route path="/events" component={EventsPage} />
      <Route path="/knowledge" component={KnowledgePage} />
      <Route path="/projects" component={ProjectsPage} />
      <Route path="/projects/new" component={NewProjectPage} />
      <Route path="/projects/:projectSlug/import" component={ProjectImportReviewPage} />
      <Route path="/projects/:projectSlug" component={ProjectWorkspacePage} />
      <Route path="/sessions" component={SessionsPage} />
      <Route path="/sessions/new" component={NewSessionPage} />
      <Route path="/sessions/:sessionId" component={SessionWorkspacePage} />
      <Route path="*404" component={NotFoundPage} />
    </Router>
  );
}
