import { MetaProvider, Title } from "@solidjs/meta";
import { A, Route, Router } from "@solidjs/router";
import { Suspense, lazy } from "solid-js";
import "./app.css";

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

export default function App() {
  return (
    <Router
      root={props => (
        <MetaProvider>
          <Title>Planner</Title>
          <div class="app-shell">
            <header class="app-header">
              <div class="app-brand">
                <span class="app-brand-mark">Planner</span>
                <span class="app-brand-copy">Local-first project analysis and build workspace</span>
              </div>
              <nav class="app-nav" aria-label="Primary">
                <A href="/" end activeClass="is-active">
                  Home
                </A>
                <A href="/projects" activeClass="is-active">
                  Projects
                </A>
                <A href="/knowledge" activeClass="is-active">
                  Knowledge
                </A>
                <A href="/blueprint" activeClass="is-active">
                  Blueprint
                </A>
                <A href="/discovery" activeClass="is-active">
                  Discovery
                </A>
                <A href="/events" activeClass="is-active">
                  Events
                </A>
                <A href="/admin" activeClass="is-active">
                  Admin
                </A>
                <A href="/sessions" activeClass="is-active">
                  Sessions
                </A>
              </nav>
            </header>
            <main class="app-main">
              <Suspense>{props.children}</Suspense>
            </main>
          </div>
        </MetaProvider>
      )}
    >
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
