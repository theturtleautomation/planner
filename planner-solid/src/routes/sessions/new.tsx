import { Title } from "@solidjs/meta";
import { A, useNavigate } from "@solidjs/router";
import { createEffect } from "solid-js";

import { withFrontendMockSearch } from "~/lib/mock/runtime";

export default function NewSessionPage() {
  const navigate = useNavigate();
  const target = withFrontendMockSearch("/projects/new");

  createEffect(() => {
    navigate(target, { replace: true });
  });

  return (
    <section class="page page-scroll">
      <Title>Redirecting To Projects</Title>
      <div class="stack page-frame">
        <section class="section-panel page-intro-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Projects</div>
              <h1 class="page-title">Projects are required for all new work</h1>
              <p class="page-copy">
                Planner no longer supports projectless direct sessions. Redirecting you to project
                creation now.
              </p>
            </div>
          </div>
          <div class="button-row">
            <A class="btn btn-primary" href={target}>
              Go to project creation
            </A>
          </div>
        </section>
      </div>
    </section>
  );
}
