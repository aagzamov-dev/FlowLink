import { describe, expect, it } from "vitest";
import { formatChord, parseChord } from "./shortcuts";
describe("shortcut editing", () => {
  it("roundtrips complete combinations", () => {
    const chord = parseChord("Ctrl+Shift+C");
    expect(chord).toEqual({ key: 46, modifiers: 3 });
    expect(formatChord(chord!.key, chord!.modifiers)).toBe("Ctrl+Shift+C");
  });
  it("supports editor punctuation shortcuts", () => {
    expect(parseChord("Cmd+/")).toEqual({ key: 53, modifiers: 8 });
    expect(parseChord("Ctrl+Shift+[")).toEqual({ key: 26, modifiers: 3 });
  });
  it("supports Mac and Windows modifier names", () => {
    expect(parseChord("Cmd+V")).toEqual(parseChord("Win+V"));
    expect(parseChord("Option+Tab")).toEqual(parseChord("Alt+Tab"));
  });
  it("rejects ambiguous or incomplete combinations", () => {
    for (const value of ["Ctrl+", "Ctrl+Ctrl+C", "Cmd+Win+V", "Banana+C", ""])
      expect(parseChord(value)).toBeNull();
  });
});
