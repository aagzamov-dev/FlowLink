import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { Devices } from "./Devices";
import { preview, type Snapshot } from "../types";

function render(state: Snapshot) {
  return renderToStaticMarkup(
    <Devices
      state={state}
      run={async () => undefined}
      working={false}
      manual={() => {}}
      connectDevice={() => {}}
      navigate={() => {}}
      copy={async () => {}}
    />,
  );
}

it("shows remote pause instead of claiming the connection is active", () => {
  const state: Snapshot = {
    ...preview,
    connectedId: "peer",
    peerPaused: true,
    devices: [
      {
        id: "peer",
        name: "Mac",
        os: "macos",
        addresses: [],
        port: 47831,
        online: true,
        busy: true,
        trusted: true,
        favorite: false,
        autoConnect: false,
        lastSeen: 0,
      },
    ],
  };
  const html = render(state);
  expect(html).toContain("Paused on the other computer");
  expect(html).toContain("Resume there to continue");
  expect(html).not.toContain("Connected securely");
  const resumed = render({ ...state, peerPaused: false });
  expect(resumed).toContain("Connected securely");
  expect(resumed).not.toContain("Resume there to continue");
});
