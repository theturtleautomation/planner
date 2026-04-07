import {
  buildPromptAnswer,
  buildPromptAnswers,
  buildSessionExportFilename,
  countAnsweredPromptItems,
  countProcessedPromptItems,
  describePromptItemProjection,
  draftEntryFromSavedDraft,
  draftHasContent,
  firstUnprocessedPromptItemId,
  presentSessionTitle,
} from "./workspace";

describe("workspace helpers", () => {
  it("prefers explicit titles and falls back to the description", () => {
    expect(
      presentSessionTitle({
        id: "12345678-aaaa-bbbb-cccc-123456789012",
        title: "Calendar intake",
        project_description: "Personal calendar app",
      }),
    ).toBe("Calendar intake");

    expect(
      presentSessionTitle({
        id: "12345678-aaaa-bbbb-cccc-123456789012",
        title: null,
        project_description: "Personal calendar app with task tracking",
      }),
    ).toContain("Personal calendar app");
  });

  it("builds prompt answers truthfully from local drafts", () => {
    const answers = buildPromptAnswers(
      {
        prompt_id: "prompt-1",
        title: "Verify Platform",
        kind: "verification_batch",
        items: [
          {
            item_id: "item-1",
            kind: "verification",
            text: "Should this ship as a web app first?",
            options: [{ option_id: "web", label: "Web app", semantic_value: "Web app" }],
            required: true,
          },
          {
            item_id: "item-2",
            kind: "verification",
            text: "Anything else?",
            options: [],
            required: false,
          },
        ],
        allow_partial_submit: true,
      },
      {
        "item-1": {
          selectedOptionId: "web",
          customText: "Start on the web.",
        },
      },
    );

    expect(answers[0]).toEqual({
      item_id: "item-1",
      selected_option_id: "web",
      custom_text: "Start on the web.",
    });
    expect(answers[1]).toEqual({
      item_id: "item-2",
      skipped: true,
    });
  });

  it("builds one prompt answer for immediate commit semantics", () => {
    expect(
      buildPromptAnswer("item-1", {
        selectedOptionId: "web",
        customText: "Start on the web.",
      }),
    ).toEqual({
      item_id: "item-1",
      selected_option_id: "web",
      custom_text: "Start on the web.",
    });

    expect(buildPromptAnswer("item-2", undefined)).toEqual({
      item_id: "item-2",
      skipped: true,
    });
  });

  it("builds a stable session export filename from the visible title", () => {
    expect(
      buildSessionExportFilename({
        id: "12345678-aaaa-bbbb-cccc-123456789012",
        title: "Calendar intake",
        project_description: "Personal calendar app with task tracking",
      }),
    ).toBe("calendar-intake.json");

    expect(
      buildSessionExportFilename({
        id: "12345678-aaaa-bbbb-cccc-123456789012",
        title: null,
        project_description: "Personal calendar app with task tracking",
      }),
    ).toContain("personal-calendar-app");
  });

  it("maps saved drafts into local draft entries and counts answered prompt items", () => {
    const restored = draftEntryFromSavedDraft({
      prompt_id: "prompt-1",
      item_id: "item-1",
      selected_option_id: "web",
      custom_text: "Ship as a desktop web app first.",
      skipped: false,
      updated_at: "2026-03-25T00:00:00Z",
    });

    expect(restored).toEqual({
      selectedOptionId: "web",
      customText: "Ship as a desktop web app first.",
    });
    expect(draftHasContent(restored)).toBe(true);
    expect(draftHasContent({ customText: "   " })).toBe(false);

    expect(
      countAnsweredPromptItems(
        {
          prompt_id: "prompt-1",
          title: "Verify Platform",
          kind: "verification_batch",
          items: [
            {
              item_id: "item-1",
              kind: "verification",
              text: "Should this ship as a web app first?",
              options: [],
              required: true,
            },
            {
              item_id: "item-2",
              kind: "verification",
              text: "Anything else?",
              options: [],
              required: false,
            },
          ],
          allow_partial_submit: true,
        },
        {
          "item-1": restored,
          "item-2": { customText: "Support keyboard-first scheduling." },
        },
      ),
    ).toBe(2);
  });

  it("treats structured payload drafts as real content", () => {
    const restored = draftEntryFromSavedDraft({
      prompt_id: "prompt-2",
      item_id: "item-structured",
      selected_option_id: null,
      custom_text: null,
      structured_payload: {
        ordered_option_ids: ["path-a", "path-b"],
        field_values: { rationale: "Prioritize the primary path first." },
      },
      skipped: false,
      updated_at: "2026-03-25T00:00:00Z",
    });

    expect(draftHasContent(restored)).toBe(true);
    expect(
      buildPromptAnswers(
        {
          prompt_id: "prompt-2",
          title: "Tradeoff",
          kind: "question_batch",
          items: [
            {
              item_id: "item-structured",
              kind: "discovery",
              text: "Which path should we take?",
              options: [],
              required: true,
            },
          ],
          allow_partial_submit: true,
        },
        {
          "item-structured": restored!,
        },
      )[0],
    ).toMatchObject({
      item_id: "item-structured",
      structured_payload: {
        ordered_option_ids: ["path-a", "path-b"],
        field_values: { rationale: "Prioritize the primary path first." },
      },
    });
  });

  it("builds truthful artifact projection text and tracks processed prompt items", () => {
    const prompt = {
      prompt_id: "prompt-1",
      title: "Success criteria",
      kind: "question_batch" as const,
      items: [
        {
          item_id: "item-1",
          kind: "discovery" as const,
          text: "How will you judge the first release as successful?",
          options: [
            { option_id: "main-flow", label: "Main flow works", semantic_value: "Main flow works" },
            { option_id: "time-saved", label: "Time saved", semantic_value: "Time saved" },
          ],
          required: true,
        },
        {
          item_id: "item-2",
          kind: "discovery" as const,
          text: "What failure would make this release a miss?",
          options: [],
          required: false,
        },
      ],
      allow_partial_submit: true,
    };

    expect(
      describePromptItemProjection(prompt.items[0], {
        selectedOptionId: "main-flow",
        customText: "Reliable scheduling and task completion.",
      }),
    ).toEqual([
      "Main flow works",
      "Reliable scheduling and task completion.",
    ]);

    expect(describePromptItemProjection(prompt.items[1], undefined)).toEqual([]);

    expect(
      countProcessedPromptItems(prompt, {
        "item-1": true,
      }),
    ).toBe(1);

    expect(
      firstUnprocessedPromptItemId(prompt, {
        "item-1": true,
      }),
    ).toBe("item-2");

    expect(
      firstUnprocessedPromptItemId(prompt, {
        "item-1": true,
        "item-2": true,
      }),
    ).toBeNull();
  });
});
