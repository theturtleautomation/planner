import { Title } from "@solidjs/meta";
import { useNavigate, useSearchParams } from "@solidjs/router";
import { Match, Switch, createEffect, createMemo, createSignal } from "solid-js";
import { isServer } from "solid-js/web";

import { ProjectCreateForm } from "~/components/projects/ProjectCreateForm";
import { createProject } from "~/lib/api";
import { isFrontendMockEnabled, withFrontendMockSearch } from "~/lib/mock/runtime";
import { createMockProject, getMockProject } from "~/lib/mock/store";

const PENDING_MOCK_PROJECT_STORAGE_KEY = "planner.frontend-mock.pending-project";

function firstParam(value: string | string[] | undefined): string {
  return Array.isArray(value) ? value[0] ?? "" : value ?? "";
}

function homeFallbackSlug(input: string): string {
  return (
    input
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 48) || "project"
  );
}

export default function NewProjectPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [autoCreateError, setAutoCreateError] = createSignal<string | null>(null);
  const [autoCreating, setAutoCreating] = createSignal(false);
  const [autoCreateStarted, setAutoCreateStarted] = createSignal(false);
  const initialName = createMemo(() => firstParam(searchParams.name).trim());
  const initialDescription = createMemo(() => firstParam(searchParams.description).trim());

  if (isServer && isFrontendMockEnabled() && initialName()) {
    const slug = homeFallbackSlug(initialName());
    const response = (() => {
      try {
        return getMockProject(slug);
      } catch {
        return createMockProject({
          name: initialName(),
          description: initialDescription() || null,
          slug,
        });
      }
    })();
    const target = withFrontendMockSearch(`/projects/${response.project.slug}`);

    return (
      <section class="page page-scroll">
        <Title>Creating Project</Title>
        <div class="stack page-frame">
          <section class="section-panel form-panel page-intro-panel">
            <div class="empty-state">Creating project…</div>
            <script>
              {`window.sessionStorage.setItem(${JSON.stringify(PENDING_MOCK_PROJECT_STORAGE_KEY)}, ${JSON.stringify(
                JSON.stringify({
                  name: response.project.name,
                  description: response.project.description,
                  slug: response.project.slug,
                }),
              )}); window.location.replace(${JSON.stringify(target)});`}
            </script>
          </section>
        </div>
      </section>
    );
  }

  createEffect(() => {
    if (!initialName() || autoCreateStarted()) return;

    setAutoCreateStarted(true);
    setAutoCreating(true);
    setAutoCreateError(null);

    void createProject({
      name: initialName(),
      description: initialDescription() || null,
    })
      .then(response => {
        navigate(withFrontendMockSearch(`/projects/${response.project.slug}`), {
          replace: true,
        });
      })
      .catch(error => {
        setAutoCreateError(error instanceof Error ? error.message : "Unable to create project.");
        setAutoCreating(false);
      });
  });

  return (
    <section class="page page-scroll">
      <Title>New Project</Title>
      <div class="stack page-frame">
        <section class="section-panel form-panel page-intro-panel">
          <Switch>
            <Match when={autoCreating()}>
              <div class="empty-state">Creating project…</div>
            </Match>
            <Match when={autoCreateError()}>
              {message => (
                <>
                  <div class="error-copy">{message()}</div>
                  <ProjectCreateForm
                    class="inline-form"
                    initialDescription={firstParam(searchParams.description)}
                    initialName={firstParam(searchParams.name)}
                  />
                </>
              )}
            </Match>
            <Match when={true}>
              <ProjectCreateForm class="inline-form" />
            </Match>
          </Switch>
        </section>
      </div>
    </section>
  );
}
