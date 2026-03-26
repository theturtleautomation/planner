import { Title } from "@solidjs/meta";
import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";

import { createProject } from "~/lib/api";

export default function NewProjectPage() {
  const navigate = useNavigate();
  const [name, setName] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [submitting, setSubmitting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const handleSubmit = async (event: SubmitEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);

    try {
      const response = await createProject({
        name: name().trim(),
        description: description().trim() || null,
      });
      navigate(`/projects/${response.project.slug}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to create project.");
      setSubmitting(false);
    }
  };

  return (
    <section class="page page-scroll">
      <Title>New Project</Title>
      <div class="stack page-frame">
        <section class="section-panel form-panel page-intro-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">New project</div>
              <h1 class="page-title">Create the primary container for the next analysis.</h1>
              <p class="page-copy">
                Projects are the main home for ongoing work. Name the project, add a short idea
                description, and move straight into the workspace.
              </p>
            </div>
          </div>

          <form class="inline-form" onSubmit={handleSubmit}>
            <label class="field-label">
              Project name
              <input
                class="field-input"
                value={name()}
                onInput={event => setName(event.currentTarget.value)}
                placeholder="Personal calendar"
                required
              />
            </label>
            <label class="field-label">
              What are you shaping?
              <textarea
                class="field-input textarea"
                value={description()}
                onInput={event => setDescription(event.currentTarget.value)}
                placeholder="A local-first calendar app with task tracking and deep planning support."
              />
            </label>
            {error() ? <div class="error-copy">{error()}</div> : null}
            <div class="button-row">
              <button class="btn btn-primary" type="submit" disabled={submitting()}>
                {submitting() ? "Creating…" : "Create project"}
              </button>
            </div>
          </form>
        </section>
      </div>
    </section>
  );
}
