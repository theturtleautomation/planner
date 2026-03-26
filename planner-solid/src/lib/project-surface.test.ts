import {
  buildExecutionToneForState,
  buildPathToneForState,
  readinessToneForState,
  resolveProjectSurfaceTab,
  reviewToneForState,
} from "./project-surface";

describe("project surface helpers", () => {
  it("returns null when no attached tab is present", () => {
    expect(resolveProjectSurfaceTab(undefined)).toBeNull();
    expect(resolveProjectSurfaceTab("")).toBeNull();
  });

  it("returns the exact attached tab when it is valid", () => {
    expect(resolveProjectSurfaceTab("knowledge")).toBe("knowledge");
    expect(resolveProjectSurfaceTab("readiness")).toBe("readiness");
  });

  it("falls back invalid attached tabs to review", () => {
    expect(resolveProjectSurfaceTab("unknown")).toBe("review");
  });

  it("maps readiness and review states onto shared badge tones", () => {
    expect(readinessToneForState("ready")).toBe("active");
    expect(readinessToneForState("needs-review")).toBe("attention");
    expect(reviewToneForState("pending")).toBe("attention");
    expect(reviewToneForState("applied")).toBe("active");
  });

  it("maps build-path and execution states onto shared badge tones", () => {
    expect(buildPathToneForState("staging")).toBe("recent");
    expect(buildExecutionToneForState("idle")).toBe("quiet");
  });
});
