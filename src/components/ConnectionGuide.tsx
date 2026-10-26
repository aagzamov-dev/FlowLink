import { Check, CircleHelp } from "lucide-react";
import type { Page, Run, Snapshot } from "../types";
import { layoutError } from "../layout";

export function setupSteps(state: Snapshot) {
  const mainId = state.controllerId ?? state.deviceId;
  const screens = state.displays.filter(
    (display) => display.deviceId === mainId,
  );
  return [
    {
      label: "Allow input permissions",
      done:
        state.permissions.accessibility &&
        state.permissions.monitoring &&
        state.permissions.filtering,
    },
    { label: "Pair and connect a computer", done: !!state.connectedId },
    {
      label: "Choose the main device",
      done: !!state.connectedId && !!state.controllerId,
    },
    {
      label: "Save a screen arrangement or use hotkey-only switching",
      done:
        !!state.connectedId &&
        (state.settings.hotkeyOnly ||
          layoutError(screens, state.placements) === null),
    },
  ];
}

export function ConnectionGuide({
  state,
  run,
  navigate,
  manual,
}: {
  state: Snapshot;
  run: Run;
  navigate: (page: Page) => void;
  manual: () => void;
}) {
  const steps = setupSteps(state);
  const firstSetup = !state.devices.some((device) => device.trusted);
  return (
    <section className="connection-guide">
      <details open={firstSetup}>
        <summary>
          Get started{" "}
          <span>
            {steps.filter((step) => step.done).length} of {steps.length} ready
          </span>
        </summary>
        <ol className="setup-steps">
          {steps.map((step) => (
            <li key={step.label}>
              {step.done ? (
                <Check size={16} className="accent" />
              ) : (
                <CircleHelp size={16} />
              )}
              <span>{step.label}</span>
              {!step.done && (
                <button
                  className="text-button"
                  onClick={() => {
                    if (step === steps[0]) void run("request_permissions");
                    else if (step === steps[1]) manual();
                    else if (step === steps[3]) navigate("displays");
                    else navigate("devices");
                  }}
                >
                  {step === steps[0]
                    ? "Review"
                    : step === steps[1]
                      ? "Connect"
                      : "Configure"}
                </button>
              )}
            </li>
          ))}
        </ol>
        <p className="muted">
          Test switching in both directions. Emergency return: Ctrl+Alt+Shift+
          {state.settings.emergencyKey === 1 ? "Escape" : "F12"}.
        </p>
      </details>
      <details>
        <summary>Connection help</summary>
        <dl className="connection-facts">
          <div>
            <dt>Input permissions</dt>
            <dd>
              {steps[0].done
                ? "Ready"
                : "Review permissions and restart FlowLink"}
            </dd>
          </div>
          <div>
            <dt>Network interface</dt>
            <dd>{state.settings.interface ?? "Automatic · LAN / VPN"}</dd>
          </div>
          <div>
            <dt>Local addresses</dt>
            <dd>
              {state.addresses.length
                ? `${state.addresses.length} available`
                : "No private address detected. Check Wi-Fi, Ethernet, or VPN."}
            </dd>
          </div>
          <div>
            <dt>Connection port</dt>
            <dd>TCP {state.port}</dd>
          </div>
          <div>
            <dt>Sharing</dt>
            <dd>
              {state.settings.paused
                ? "Paused here"
                : state.peerPaused
                  ? "Paused on the other computer"
                  : state.connectedId
                    ? "Connected"
                    : "Not connected"}
            </dd>
          </div>
        </dl>
        <p className="muted">
          Open matching FlowLink builds on both computers. If Nearby is empty,
          connect by private IP or hostname. Your firewall or VPN must allow the
          destination port; an RDP connection uses a different port.
        </p>
        {state.error && (
          <p role="status" className="alert">
            {state.error}
          </p>
        )}
        <div className="card-actions">
          <button onClick={manual} disabled={!!state.connectedId}>
            Connect by IP
          </button>
          <button onClick={() => navigate("settings")}>Open diagnostics</button>
        </div>
      </details>
    </section>
  );
}
