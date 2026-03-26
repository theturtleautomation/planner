// @ts-nocheck
import { expect, test } from "@playwright/test";

const sessionPayload = {
  session: {
    id: "session-1",
    title: "Calendar analysis",
    archived: false,
    created_at: "2026-03-25T00:00:00Z",
    last_activity_at: "2026-03-25T00:10:00Z",
    pipeline_running: false,
    intake_phase: "interviewing",
    project_description: "Personal calendar analysis",
    project_id: "project-1",
    project_slug: "personal-calendar",
    project_name: "Personal Calendar",
    current_step: "socratic.prompt.generated",
    error_message: null,
    can_resume_live: false,
    can_resume_checkpoint: true,
    can_restart_from_description: true,
    can_retry_pipeline: false,
    has_checkpoint: true,
    resume_status: "interview_checkpoint_resumable",
    workspace_status: {
      state: "awaiting_response",
      label: "Waiting for your response",
      detail: "The first prompt bank is ready for local switching and answers.",
      tone: "neutral",
    },
  },
};

const promptBankPayload = {
  session_id: "session-1",
  active_thread_id: "success_criteria",
  banked_threads: [
    {
      category_id: "success_criteria",
      title: "Success criteria",
      summary: "Define what makes the first release a success.",
      question_count: 2,
      prompt: {
        prompt_id: "prompt-success",
        title: "Success criteria",
        kind: "question_batch",
        origin_category_id: "success_criteria",
        items: [
          {
            item_id: "success-q1",
            kind: "discovery",
            text: "How will you judge the first release as successful?",
            options: [
              { option_id: "main-flow", label: "Main flow works", semantic_value: "Main flow works" },
              { option_id: "time-saved", label: "Time saved", semantic_value: "Time saved" },
            ],
            required: true,
          },
          {
            item_id: "success-q2",
            kind: "discovery",
            text: "What failure would make this release a miss?",
            options: [],
            required: false,
          },
        ],
        allow_partial_submit: true,
      },
    },
    {
      category_id: "core_workflows",
      title: "Core workflows",
      summary: "Clarify the minimum must-work flows for the first release.",
      question_count: 1,
      prompt: {
        prompt_id: "prompt-workflows",
        title: "Core workflows",
        kind: "question_batch",
        origin_category_id: "core_workflows",
        items: [
          {
            item_id: "workflow-q1",
            kind: "discovery",
            text: "Which actions must work end to end on day one?",
            options: [],
            required: true,
          },
        ],
        allow_partial_submit: true,
      },
    },
  ],
  queued_threads: [
    {
      category_id: "integrations",
      title: "Integrations",
      summary: "Wait for decisions on core workflows first.",
      status: "pending",
      question_count: 2,
    },
  ],
  build_ready: false,
  build_readiness_message: null,
  initial_bank_complete: true,
  saved_drafts: {
    "success-q1": {
      prompt_id: "prompt-success",
      item_id: "success-q1",
      selected_option_id: "main-flow",
      custom_text: "Main flow means scheduling and task completion feel reliable.",
      skipped: false,
      updated_at: "2026-03-25T00:05:00Z",
    },
  },
};

async function mockSessionWorkspace(page, draftSaves) {
  await page.route("**/api/sessions/session-1", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(sessionPayload),
    });
  });

  await page.route("**/api/sessions/session-1/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(promptBankPayload),
    });
  });

  await page.route("**/api/sessions/session-1/prompt-drafts", async route => {
    const payload = route.request().postDataJSON();
    draftSaves.push(payload);

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-1",
        prompt_id: payload.promptId,
        saved_count: 1,
        cleared_count: 0,
        saved_at: "2026-03-25T00:06:00Z",
      }),
    });
  });
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    class MockWebSocket {
      static OPEN = 1;
      readyState = 1;
      onopen = null;
      onclose = null;
      onerror = null;
      onmessage = null;

      constructor(_url) {
        setTimeout(() => this.onopen?.(), 0);
      }

      send() {}

      close() {
        this.onclose?.();
      }
    }

    window.WebSocket = MockWebSocket;
  });
});

test("phase 34 shows the full question bank without a redundant artifact pane", async ({ page }) => {
  const draftSaves = [];
  await mockSessionWorkspace(page, draftSaves);

  await page.goto("/sessions/session-1");

  await expect(page.getByRole("link", { name: "Back to project" })).toBeVisible();
  await expect(page.locator(".session-question-header")).toContainText("Question-bank workspace");
  await expect(page.locator(".session-question-header-actions")).toContainText("Project import");
  await expect(page.locator(".session-question-header-actions")).toContainText("Duplicate");
  await expect(page.locator(".session-question-header-actions")).toContainText("Export");
  await expect(page.locator(".session-question-header-actions")).toContainText("Restart");
  await expect(page.locator(".session-question-summary-strip")).toContainText("Questions");
  await expect(page.locator(".session-question-summary-strip")).toContainText("Committed");
  await expect(page.locator(".session-question-jumpbar")).toContainText("Question bank");
  await expect(page.locator(".session-thread-chip")).toHaveCount(2);
  await expect(page.locator(".session-question-card")).toHaveCount(3);
  await expect(page.locator("textarea")).toHaveCount(3);
  await expect(page.getByText("How will you judge the first release as successful?")).toBeVisible();
  await expect(page.getByText("What failure would make this release a miss?")).toBeVisible();
  await expect(page.getByText("Which actions must work end to end on day one?")).toBeVisible();
  await expect(page.locator(".session-queued-panel")).toContainText("Queued threads");
  await expect(page.locator(".session-queued-panel")).toContainText("Integrations");
  await expect(page.locator(".session-artifact-pane")).toHaveCount(0);
  await expect(page.locator(".session-surface-tab")).toHaveCount(0);
  await expect(page.getByText("Prompt anchor")).toHaveCount(0);
  await expect(page.getByText("Working blueprint")).toHaveCount(0);
});

test("phase 34 keeps the same all-questions-visible workspace on narrow widths", async ({ page }) => {
  const draftSaves = [];
  await mockSessionWorkspace(page, draftSaves);
  await page.setViewportSize({ width: 840, height: 900 });

  await page.goto("/sessions/session-1");

  await expect(page.locator(".session-surface-tab")).toHaveCount(0);
  await expect(page.locator(".session-question-card")).toHaveCount(3);
  await expect(page.locator("textarea")).toHaveCount(3);

  await page.getByRole("button", { name: /Core workflows 0\/1/i }).click();
  await expect(page.locator(".session-thread-section.is-active")).toContainText("Core workflows");
  await expect(page.locator(".session-question-card.is-active")).toContainText(
    "Which actions must work end to end on day one?",
  );
  await expect(page.locator(".session-queued-panel")).toContainText("Integrations");
});
