import { useState } from "react";
import type { Direction, ShortcutBinding } from "../types";
import { formatChord, parseChord } from "../shortcuts";

export function ShortcutEditor({
  bindings,
  onChange,
}: {
  bindings: ShortcutBinding[];
  onChange: (value: ShortcutBinding[]) => void;
}) {
  const [direction, setDirection] = useState<Direction>("windowsToMac");
  const [source, setSource] = useState("");
  const [target, setTarget] = useState("");
  const [editing, setEditing] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const save = () => {
    const from = parseChord(source),
      to = parseChord(target);
    if (!from || !to) {
      setError(
        "Use a key or combination such as Ctrl+Shift+C, Cmd+V, Alt+Tab, or F5.",
      );
      return;
    }
    if (
      bindings.some(
        (b, i) =>
          i !== editing &&
          b.direction === direction &&
          b.sourceKey === from.key &&
          b.sourceModifiers === from.modifiers,
      )
    ) {
      setError("That source shortcut already has a rule in this direction.");
      return;
    }
    const binding: ShortcutBinding = {
      direction,
      sourceKey: from.key,
      sourceModifiers: from.modifiers,
      targetKey: to.key,
      targetModifiers: to.modifiers,
    };
    onChange(
      editing === null
        ? [...bindings, binding]
        : bindings.map((b, i) => (i === editing ? binding : b)),
    );
    setSource("");
    setTarget("");
    setEditing(null);
    setError(null);
  };
  return (
    <div className="padded">
      <strong>Custom shortcuts</strong>
      <p className="muted">
        Rules match the complete combination and override the preset. Meta means
        Command on Mac and Windows key on PC. The switch and emergency shortcuts
        are reserved. Click Save changes above to apply.
      </p>
      {bindings.map((binding, index) => (
        <div
          className="trusted-setting"
          key={`${binding.direction}-${binding.sourceKey}-${binding.sourceModifiers}`}
        >
          <span>
            {binding.direction === "windowsToMac"
              ? "Windows → Mac"
              : "Mac → Windows"}
            <br />
            {formatChord(binding.sourceKey, binding.sourceModifiers)} →{" "}
            {formatChord(binding.targetKey, binding.targetModifiers)}
          </span>
          <button
            onClick={() => {
              setEditing(index);
              setDirection(binding.direction);
              setSource(
                formatChord(binding.sourceKey, binding.sourceModifiers),
              );
              setTarget(
                formatChord(binding.targetKey, binding.targetModifiers),
              );
            }}
          >
            Edit
          </button>
          <button
            onClick={() => {
              onChange(bindings.filter((_, i) => i !== index));
              setEditing(null);
            }}
          >
            Remove
          </button>
        </div>
      ))}
      <div className="shortcut-editor">
        <select
          aria-label="Shortcut direction"
          value={direction}
          onChange={(e) => setDirection(e.target.value as Direction)}
        >
          <option value="windowsToMac">Windows → Mac</option>
          <option value="macToWindows">Mac → Windows</option>
        </select>
        <input
          aria-label="Source shortcut"
          placeholder={
            direction === "windowsToMac" ? "Ctrl+Shift+C" : "Cmd+Shift+C"
          }
          value={source}
          onChange={(e) => setSource(e.target.value)}
        />
        <span>→</span>
        <input
          aria-label="Target shortcut"
          placeholder={
            direction === "windowsToMac" ? "Cmd+Shift+C" : "Ctrl+Shift+C"
          }
          value={target}
          onChange={(e) => setTarget(e.target.value)}
        />
        <button
          disabled={editing === null && bindings.length >= 64}
          onClick={save}
        >
          {editing === null ? "Add rule" : "Update rule"}
        </button>
        {editing !== null && (
          <button
            onClick={() => {
              setEditing(null);
              setSource("");
              setTarget("");
            }}
          >
            Cancel
          </button>
        )}
      </div>
      {error && (
        <p role="alert" className="alert">
          {error}
        </p>
      )}
    </div>
  );
}
