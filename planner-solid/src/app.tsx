import { MetaProvider, Title } from "@solidjs/meta";
import { A, Route, Router } from "@solidjs/router";
import { Suspense, lazy } from "solid-js";
import "./app.css";

const HomePage = lazy(() => import("./routes/index"));
const ProjectsPage = lazy(() => import("./routes/projects/index"));
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
                <span class="app-brand-copy">SolidStart project-first workspace</span>
              </div>
              <nav class="app-nav" aria-label="Primary">
                <A href="/" end activeClass="is-active">
                  Work
                </A>
                <A href="/projects" activeClass="is-active">
                  Projects
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
      <Route path="/projects" component={ProjectsPage} />
      <Route path="/projects/new" component={NewProjectPage} />
      <Route path="/projects/:projectSlug" component={ProjectWorkspacePage} />
      <Route path="/sessions" component={SessionsPage} />
      <Route path="/sessions/new" component={NewSessionPage} />
      <Route path="/sessions/:sessionId" component={SessionWorkspacePage} />
      <Route path="*404" component={NotFoundPage} />
    </Router>
  );
}
