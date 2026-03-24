import { buildPromptAnswers, presentSessionTitle } from "./workspace";

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
});
