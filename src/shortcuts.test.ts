import { describe, expect, it } from "vitest";
import {
  formatChord,
  parseChord,
  recordChord,
  reservedChord,
} from "./shortcuts";
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

describe("shortcut recording", () => {
  const event = {
    code: "KeyC",
    ctrlKey: true,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    isComposing: false,
    repeat: false,
  };
  it("uses physical codes for punctuation and non-Latin layouts", () => {
    expect(recordChord(event)).toBe("Ctrl+C");
    expect(
      recordChord({ ...event, code: "Slash", ctrlKey: false, metaKey: true }),
    ).toBe("Meta+/");
    expect(recordChord({ ...event, code: "ArrowRight" })).toBe("Ctrl+Right");
    expect(recordChord({ ...event, code: "Digit2", shiftKey: true })).toBe(
      "Ctrl+Shift+2",
    );
  });
  it("ignores composition, repeats, modifiers, and unsupported keys", () => {
    expect(recordChord({ ...event, isComposing: true })).toBeNull();
    expect(recordChord({ ...event, repeat: true })).toBeNull();
    expect(recordChord({ ...event, code: "ControlLeft" })).toBeNull();
    expect(recordChord({ ...event, code: "Numpad1" })).toBeNull();
  });
  it("reserves emergency and switch chords only with the exact modifiers", () => {
    for (const key of [1, 88, 0x14d])
      expect(reservedChord({ key, modifiers: 7 })).toBe(true);
    expect(reservedChord({ key: 1, modifiers: 0 })).toBe(false);
    expect(reservedChord({ key: 46, modifiers: 7 })).toBe(false);
  });
});
