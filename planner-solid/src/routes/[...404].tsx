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
          This SolidStart shell only carries the bounded Phase 00 route map.
        </p>
        <div class="button-row">
          <A class="btn btn-primary" href="/sessions">
            Open sessions
          </A>
        </div>
      </div>
    </section>
  );
}
