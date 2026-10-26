import { describe, expect, it } from "vitest";
import { mergeDraft } from "./drafts";

describe("unsaved drafts", () => {
  it("keeps local edits while accepting external pause and preference changes", () => {
    const base = {
      deviceName: "Mac",
      paused: false,
      pointerSpeed: 1,
      bindings: [] as number[],
    };
    const draft = { ...base, deviceName: "Work Mac", bindings: [46] };
    expect(
      mergeDraft(base, draft, { ...base, paused: true, pointerSpeed: 2 }),
    ).toEqual({
      deviceName: "Work Mac",
      paused: true,
      pointerSpeed: 2,
      bindings: [46],
    });
  });
  it("accepts a saved draft as clean and keeps newer edits", () => {
    const base = { name: "Old", speed: 1 };
    const submitted = { name: "Saved", speed: 1 };
    expect(mergeDraft(base, submitted, submitted)).toEqual(submitted);
    expect(mergeDraft(base, { ...submitted, speed: 2 }, submitted)).toEqual({
      name: "Saved",
      speed: 2,
    });
  });
  it("preserves display edits until the saved arrangement arrives", () => {
    const base = { placements: [{ x: 0, y: 0 }] };
    const draft = { placements: [{ x: 100, y: -20 }] };
    expect(mergeDraft(base, draft, base)).toEqual(draft);
    expect(mergeDraft(base, draft, draft)).toEqual(draft);
  });
});
