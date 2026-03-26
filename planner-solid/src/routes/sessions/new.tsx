import { Title } from "@solidjs/meta";
import { A, useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";

import { createSession } from "~/lib/api";

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
      const created = await createSession({ description: trimmed });
      navigate(`/sessions/${created.session.id}`);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
      setSubmitting(false);
    }
  };

  return (
    <section class="page page-scroll">
      <Title>Direct Session</Title>
      <div class="stack page-frame">
        <section class="section-panel page-intro-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Direct session</div>
              <h1 class="page-title">Start a focused direct session</h1>
              <p class="page-copy">
                Projects remain the primary home for ongoing work. Use a direct session when you
                need a focused one-off detour and do not want to create a new project first.
              </p>
            </div>
          </div>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Direct brief</div>
              <h2 class="section-title">One-off session brief</h2>
              <p class="section-copy">
                Use a concise description when the work does not need a project container.
              </p>
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
                  {submitting() ? "Starting…" : "Start direct session"}
                </button>
                <A class="btn btn-subtle" href="/projects/new">
                  Start with a project instead
                </A>
              </div>
            </form>
          </div>
        </section>
      </div>
    </section>
  );
}
