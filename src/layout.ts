import type { Rect, Display, Placement, Device } from "./types";
export type Side = "left" | "right" | "top" | "bottom";
export function placeBeside(screen: Rect, target: Rect, side: Side): Rect {
  switch (side) {
    case "left":
      return { ...target, x: screen.x - target.width, y: screen.y };
    case "right":
      return { ...target, x: screen.x + screen.width, y: screen.y };
    case "top":
      return { ...target, x: screen.x, y: screen.y - target.height };
    case "bottom":
      return { ...target, x: screen.x, y: screen.y + screen.height };
  }
}
export function snap(rect: Rect, screens: Display[], threshold = 60): Rect {
  const result = { ...rect };
  let xDistance = threshold;
  let yDistance = threshold;
  for (const { rect: r } of screens) {
    for (const x of [
      r.x - rect.width,
      r.x + r.width,
      r.x,
      r.x + r.width - rect.width,
    ]) {
      if (Math.abs(rect.x - x) < xDistance) {
        xDistance = Math.abs(rect.x - x);
        result.x = x;
      }
    }
    for (const y of [
      r.y - rect.height,
      r.y + r.height,
      r.y,
      r.y + r.height - rect.height,
    ]) {
      if (Math.abs(rect.y - y) < yDistance) {
        yDistance = Math.abs(rect.y - y);
        result.y = y;
      }
    }
  }
  return result;
}
export function layoutError(
  screens: Display[],
  placements: Placement[],
): string | null {
  if (placements.length !== 1)
    return "Choose one controlled display to arrange your screens.";
  const b = placements[0].rect;
  if (![b.x, b.y, b.width, b.height].every(Number.isFinite))
    return "Display position is invalid.";
  let adjacent = false;
  for (const { rect: a } of screens) {
    if (
      a.x < b.x + b.width - 1 &&
      a.x + a.width > b.x + 1 &&
      a.y < b.y + b.height - 1 &&
      a.y + a.height > b.y + 1
    )
      return "Screens overlap. Drag the controlled screen beside a main display.";
    const vertical =
      Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y);
    const horizontal =
      Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x);
    if (
      vertical >= 16 &&
      (Math.abs(a.x - b.x - b.width) <= 1 || Math.abs(a.x + a.width - b.x) <= 1)
    )
      adjacent = true;
    if (
      horizontal >= 16 &&
      (Math.abs(a.y - b.y - b.height) <= 1 ||
        Math.abs(a.y + a.height - b.y) <= 1)
    )
      adjacent = true;
  }
  return adjacent
    ? null
    : "Snap the controlled screen to an edge of a main display.";
}
export function sortedDevices(devices: Device[], query = ""): Device[] {
  const text = query.trim().toLocaleLowerCase();
  return devices
    .filter((d) =>
      `${d.name} ${d.addresses.join(" ")}`.toLocaleLowerCase().includes(text),
    )
    .sort(
      (a, b) =>
        Number(b.favorite) - Number(a.favorite) ||
        Number(b.online) - Number(a.online) ||
        b.lastSeen - a.lastSeen ||
        a.name.localeCompare(b.name),
    );
}
export function endpoint(ip: string, port: number): string {
  return ip.includes(":") ? `[${ip}]:${port}` : `${ip}:${port}`;
}

export function sharedEdges(
  screens: Display[],
  target: Rect,
): { x1: number; y1: number; x2: number; y2: number }[] {
  const edges = [];
  for (const { rect: r } of screens) {
    const top = Math.max(r.y, target.y);
    const bottom = Math.min(r.y + r.height, target.y + target.height);
    const left = Math.max(r.x, target.x);
    const right = Math.min(r.x + r.width, target.x + target.width);
    if (bottom - top >= 16) {
      if (Math.abs(r.x + r.width - target.x) <= 1)
        edges.push({ x1: target.x, y1: top, x2: target.x, y2: bottom });
      if (Math.abs(r.x - target.x - target.width) <= 1)
        edges.push({ x1: r.x, y1: top, x2: r.x, y2: bottom });
    }
    if (right - left >= 16) {
      if (Math.abs(r.y + r.height - target.y) <= 1)
        edges.push({ x1: left, y1: target.y, x2: right, y2: target.y });
      if (Math.abs(r.y - target.y - target.height) <= 1)
        edges.push({ x1: left, y1: r.y, x2: right, y2: r.y });
    }
  }
  return edges;
}
