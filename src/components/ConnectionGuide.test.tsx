import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { preview } from "../types";
import { ConnectionGuide, setupSteps } from "./ConnectionGuide";
import { Pairing } from "./Pairing";

describe("connection guidance", () => {
  it("never reports an unconnected or permission-denied device as ready", () => {
    expect(setupSteps(preview).map((step) => step.done)).toEqual([
      false,
      false,
      false,
      false,
    ]);
    const state = {
      ...preview,
      connectedId: "peer",
      controllerId: "local",
      permissions: { accessibility: true, monitoring: true, filtering: true },
      settings: { ...preview.settings, hotkeyOnly: true },
    };
    expect(setupSteps(state).every((step) => step.done)).toBe(true);
    expect(setupSteps({ ...state, peerPaused: true })[3].done).toBe(true);
  });
  it("offers network recovery guidance without implying a port probe succeeded", () => {
    const html = renderToStaticMarkup(
      <ConnectionGuide
        state={preview}
        run={async () => undefined}
        navigate={() => {}}
        manual={() => {}}
      />,
    );
    expect(html).toContain("No private address detected");
    expect(html).toContain("TCP 47831");
    expect(html).toContain("Connect by IP");
    expect(html).not.toContain("Port reachable");
  });
  it("disables expired pairing submissions and hides expired codes", () => {
    const state = {
      ...preview,
      pairing: {
        requestId: "test",
        name: "Other computer",
        address: "192.168.1.2",
        code: "123456",
        expiresAt: 1,
        awaitingCode: false,
      },
    };
    const code = renderToStaticMarkup(
      <Pairing state={state} run={async () => undefined} />,
    );
    expect(code).toContain("Pairing expired");
    expect(code).not.toContain("123456");
    const entry = renderToStaticMarkup(
      <Pairing
        state={{
          ...state,
          pairing: { ...state.pairing, code: null, awaitingCode: true },
        }}
        run={async () => undefined}
      />,
    );
    expect(entry).toContain('disabled=""');
  });
});
