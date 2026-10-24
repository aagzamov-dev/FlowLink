const rows: readonly (readonly [string, number])[] = [
  ["Escape", 1],
  ["1", 2],
  ["2", 3],
  ["3", 4],
  ["4", 5],
  ["5", 6],
  ["6", 7],
  ["7", 8],
  ["8", 9],
  ["9", 10],
  ["0", 11],
  ["-", 12],
  ["=", 13],
  ["Backspace", 14],
  ["Tab", 15],
  ["Q", 16],
  ["W", 17],
  ["E", 18],
  ["R", 19],
  ["T", 20],
  ["Y", 21],
  ["U", 22],
  ["I", 23],
  ["O", 24],
  ["P", 25],
  ["[", 26],
  ["]", 27],
  ["Enter", 28],
  ["A", 30],
  ["S", 31],
  ["D", 32],
  ["F", 33],
  ["G", 34],
  ["H", 35],
  ["J", 36],
  ["K", 37],
  ["L", 38],
  [";", 39],
  ["'", 40],
  ["`", 41],
  ["Backslash", 43],
  ["Z", 44],
  ["X", 45],
  ["C", 46],
  ["V", 47],
  ["B", 48],
  ["N", 49],
  ["M", 50],
  [",", 51],
  [".", 52],
  ["/", 53],
  ["CapsLock", 58],
  ["Space", 57],
  ["F1", 59],
  ["F2", 60],
  ["F3", 61],
  ["F4", 62],
  ["F5", 63],
  ["F6", 64],
  ["F7", 65],
  ["F8", 66],
  ["F9", 67],
  ["F10", 68],
  ["F11", 87],
  ["F12", 88],
  ["Home", 0x147],
  ["Up", 0x148],
  ["PageUp", 0x149],
  ["Left", 0x14b],
  ["Right", 0x14d],
  ["End", 0x14f],
  ["Down", 0x150],
  ["PageDown", 0x151],
  ["Insert", 0x152],
  ["Delete", 0x153],
];
export function parseChord(
  text: string,
): { key: number; modifiers: number } | null {
  const parts = text.split("+").map((s) => s.trim().toLowerCase());
  const key = rows.find(([name]) => name.toLowerCase() === parts.at(-1))?.[1];
  if (key === undefined) return null;
  let modifiers = 0;
  const names: Readonly<Record<string, number>> = {
    ctrl: 1,
    control: 1,
    shift: 2,
    alt: 4,
    option: 4,
    cmd: 8,
    command: 8,
    win: 8,
    meta: 8,
  };
  for (const part of parts.slice(0, -1)) {
    const bit = names[part];
    if (!bit || modifiers & bit) return null;
    modifiers |= bit;
  }
  return { key, modifiers };
}
export function formatChord(key: number, modifiers: number): string {
  return [
    ...(
      [
        [1, "Ctrl"],
        [2, "Shift"],
        [4, "Alt"],
        [8, "Meta"],
      ] as const
    )
      .filter(([bit]) => modifiers & bit)
      .map(([, name]) => name),
    rows.find(([, code]) => code === key)?.[0] ?? `Scan ${key}`,
  ].join("+");
}
