import { useNavigate } from "@solidjs/router";
import type { JSX } from "solid-js";
import { createSignal } from "solid-js";

import { createProject } from "~/lib/api";
import { withFrontendMockSearch } from "~/lib/mock/runtime";

type ProjectCreateFormProps = {
  class?: string;
  submitLabel?: string;
  titlePlaceholder?: string;
  descriptionPlaceholder?: string;
  initialName?: string;
  initialDescription?: string;
  secondaryAction?: JSX.Element;
};

export function ProjectCreateForm(props: ProjectCreateFormProps) {
  const navigate = useNavigate();
  const [name, setName] = createSignal(props.initialName ?? "");
  const [description, setDescription] = createSignal(props.initialDescription ?? "");
  const [submitting, setSubmitting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const submitProject = async () => {
    const trimmedName = name().trim();
    if (!trimmedName || submitting()) return;

    setSubmitting(true);
    setError(null);

    try {
      const response = await createProject({
        name: trimmedName,
        description: description().trim() || null,
      });
      navigate(withFrontendMockSearch(`/projects/${response.project.slug}`));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to create project.");
      setSubmitting(false);
    }
  };

  return (
    <form
      class={props.class ?? "inline-form"}
      onSubmit={event => {
        event.preventDefault();
        void submitProject();
      }}
    >
      <label class="field-label">
        Project title
        <input
          class="field-input"
          name="name"
          value={name()}
          onInput={event => setName(event.currentTarget.value)}
          placeholder={props.titlePlaceholder ?? "Personal calendar"}
          required
        />
      </label>
      <label class="field-label">
        Description
        <textarea
          class="field-input textarea"
          name="description"
          value={description()}
          onInput={event => setDescription(event.currentTarget.value)}
          placeholder={
            props.descriptionPlaceholder ??
            "A local-first calendar app with task tracking and deep planning support."
          }
        />
      </label>
      {error() ? <div class="error-copy">{error()}</div> : null}
      <div class="button-row">
        <button
          class="btn btn-primary"
          type="submit"
          disabled={submitting()}
        >
          {submitting() ? "Creating…" : props.submitLabel ?? "Create project"}
        </button>
        {props.secondaryAction}
      </div>
    </form>
  );
}
