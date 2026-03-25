import { Title } from "@solidjs/meta";
import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";

import { createSession, startSocratic } from "~/lib/api";

export default function NewSessionPage() {
  const navigate = useNavigate();
  const [description, setDescription] = createSignal("");
  const [submitting, setSubmitting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const handleStart = async (event: SubmitEvent) => {
    event.preventDefault();
    const trimmed = description().trim();
    if (!trimmed || submitting()) return;

    setSubmitting(true);
    setError(null);

    try {
      const created = await createSession();
      await startSocratic(created.session.id, trimmed);
      navigate(`/sessions/${created.session.id}`);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
      setSubmitting(false);
    }
  };

  return (
    <section class="page page-scroll">
      <Title>New Session</Title>
      <div class="stack page-frame">
        <section class="section-panel page-intro-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">New session</div>
              <h1 class="page-title">Start a Socratic planning session</h1>
              <p class="page-copy">
                Write the brief, start the session, and land in the workspace immediately without
                extra setup surface competing for attention.
              </p>
            </div>
          </div>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Session brief</div>
              <h2 class="section-title">Project brief</h2>
              <p class="section-copy">Use a concise description of what you want Planner to shape.</p>
            </div>
          </div>

          <div class="stack">
            <form class="inline-form" onSubmit={handleStart}>
              <label class="field-label">
                What are you building?
                <textarea
                  class="field-input textarea"
                  value={description()}
                  onInput={(event) => setDescription(event.currentTarget.value)}
                  placeholder="Personal calendar app with task tracking, recurring reminders, and a clean weekly view."
                />
              </label>

              {error() ? <div class="error-copy">{error()}</div> : null}

              <div class="button-row">
                <button class="btn btn-primary" type="submit" disabled={submitting()}>
                  {submitting() ? "Starting…" : "Start session"}
                </button>
              </div>
            </form>
          </div>
        </section>
      </div>
    </section>
  );
}
