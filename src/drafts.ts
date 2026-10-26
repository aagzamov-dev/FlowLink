import { useState } from "react";

export function equal(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

export function mergeDraft<T extends object>(
  base: T,
  draft: T,
  incoming: T,
): T {
  const next = { ...incoming };
  for (const key of Object.keys(draft) as (keyof T)[]) {
    if (!equal(draft[key], base[key])) next[key] = draft[key];
  }
  return next;
}

export function useDraft<T extends object>(incoming: T) {
  const [edit, setEdit] = useState({ base: incoming, value: incoming });
  let current = edit;
  if (!equal(edit.base, incoming)) {
    current = {
      base: incoming,
      value: mergeDraft(edit.base, edit.value, incoming),
    };
    setEdit(current);
  }
  return {
    draft: current.value,
    dirty: !equal(current.value, incoming),
    setDraft: (value: T | ((previous: T) => T)) =>
      setEdit((previous) => ({
        ...previous,
        value: typeof value === "function" ? value(previous.value) : value,
      })),
    reset: () => setEdit({ base: incoming, value: incoming }),
  };
}
