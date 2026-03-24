import { Title } from "@solidjs/meta";
import { HttpStatusCode } from "@solidjs/start";
import { A } from "@solidjs/router";

export default function NotFound() {
  return (
    <section class="page page-scroll">
      <Title>Not Found</Title>
      <HttpStatusCode code={404} />
      <div class="stack">
        <div class="eyebrow">404</div>
        <h1 class="page-title">Route not found</h1>
        <p class="page-copy">
          That route is outside the current Planner workspace. Return to the project-first surface and continue the active analysis there.
        </p>
        <div class="button-row">
          <A class="btn btn-primary" href="/projects">
            Open projects
          </A>
          <A class="btn btn-subtle" href="/">
            Go home
          </A>
        </div>
      </div>
    </section>
  );
}
