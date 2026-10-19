import { describe, expect, it } from "vitest";
import {
  endpoint,
  layoutError,
  placeBeside,
  snap,
  sortedDevices,
  sharedEdges,
} from "./layout";
import type { Display, Device } from "./types";
const screen: Display = {
  id: "a",
  deviceId: "w",
  name: "Monitor",
  number: 1,
  rect: { x: -1920, y: 0, width: 1920, height: 1080 },
  pixelWidth: 1920,
  pixelHeight: 1080,
  scale: 1,
  rotation: 0,
  online: true,
};
describe("display arrangement", () => {
  it("rejects corner-only adjacency", () =>
    expect(
      layoutError(
        [screen],
        [{ displayId: "m", rect: { x: 0, y: 1080, width: 1512, height: 982 } }],
      ),
    ).toContain("Snap"));
  it("accepts a partial edge above a negative-origin screen", () =>
    expect(
      layoutError(
        [screen],
        [
          {
            displayId: "m",
            rect: { x: -1000, y: -982, width: 1512, height: 982 },
          },
        ],
      ),
    ).toBeNull());
  it("snaps a near edge precisely", () =>
    expect(snap({ x: 12, y: 0, width: 1512, height: 982 }, [screen]).x).toBe(
      0,
    ));
  it("places a Mac to the left without moving Windows", () =>
    expect(
      placeBeside(screen.rect, { x: 0, y: 0, width: 1512, height: 982 }, "left")
        .x,
    ).toBe(-3432));
});
describe("device selection", () => {
  const device = (id: string, favorite: boolean): Device => ({
    id,
    name: "MacBook",
    favorite,
    addresses: ["192.168.1.3"],
    os: "macos",
    port: 47831,
    online: true,
    busy: false,
    trusted: true,
    autoConnect: true,
    lastSeen: 0,
  });
  it("keeps duplicate names as distinct identities and puts favorites first", () =>
    expect(
      sortedDevices([device("a", false), device("b", true)]).map((d) => d.id),
    ).toEqual(["b", "a"]));
  it("searches addresses", () =>
    expect(sortedDevices([device("a", false)], "192.168").length).toBe(1));
  it("formats IPv6 connection addresses", () =>
    expect(endpoint("fd00::1", 47831)).toBe("[fd00::1]:47831"));
});

describe("shared edge indicators", () => {
  it("highlights only the overlapping segment", () => {
    expect(
      sharedEdges([screen], { x: 0, y: 200, width: 1512, height: 982 }),
    ).toEqual([{ x1: 0, y1: 200, x2: 0, y2: 1080 }]);
  });
  it("does not highlight a corner contact", () => {
    expect(
      sharedEdges([screen], { x: 0, y: 1080, width: 1512, height: 982 }),
    ).toEqual([]);
  });
});
