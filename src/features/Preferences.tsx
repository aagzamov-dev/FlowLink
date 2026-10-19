import { useEffect, useState } from "react";
import {
  ArrowUpRight,
  CircleHelp,
  Keyboard,
  MousePointer2,
  Settings2,
  ShieldCheck,
  Wifi,
} from "lucide-react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Device, Settings, Snapshot, Run } from "../types";
import { Toggle, Modal } from "../components/common";

export function Preferences({ state, run }: { state: Snapshot; run: Run }) {
  const [draft, setDraft] = useState<Settings>(state.settings);
  const [autostart, setAutostart] = useState(false);
  const [report, setReport] = useState<string | null>(null);
  const signature = JSON.stringify(state.settings);
  useEffect(() => {
    setDraft(JSON.parse(signature) as Settings);
  }, [signature]);
  useEffect(() => {
    if (isTauri())
      void invoke<boolean>("autostart_enabled")
        .then(setAutostart)
        .catch(() => setAutostart(false));
  }, []);
  const patch = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    setDraft((current) => ({ ...current, [key]: value }));
  const [confirmForget, setConfirmForget] = useState<Device | null>(null);
  return (
    <>
      <div className="page-heading">
        <div>
          <span className="eyebrow">MAKE IT FEEL LIKE YOURS</span>
          <h1>Settings</h1>
          <p>Simple controls for a smoother, safer workspace.</p>
        </div>
        <button
          className="primary"
          disabled={JSON.stringify(draft) === signature}
          onClick={() => void run("save_settings", { settings: draft })}
        >
          Save changes
        </button>
      </div>
      <section className="settings-section">
        <h2>
          <Settings2 size={18} />
          General
        </h2>
        <div className="settings-body">
          <label className="setting-row">
            <span>
              <strong>Device name</strong>
              <small>Shown to other FlowLink computers on your network.</small>
            </span>
            <input
              value={draft.deviceName}
              maxLength={80}
              onChange={(e) => patch("deviceName", e.target.value)}
            />
          </label>
          <Toggle
            label="Launch at login"
            detail="Start in the background when you sign in."
            checked={autostart}
            onChange={async (enabled) => {
              await run("set_autostart", { enabled });
              if (isTauri())
                setAutostart(await invoke<boolean>("autostart_enabled"));
            }}
          />
        </div>
      </section>
      <section className="settings-section">
        <h2>
          <Keyboard size={18} />
          Keyboard & shortcuts
        </h2>
        <div className="settings-body">
          <label className="setting-row">
            <span>
              <strong>Keyboard preset</strong>
              <small>Applied only when Windows controls the Mac.</small>
            </span>
            <select
              value={draft.preset}
              onChange={(e) =>
                patch("preset", e.target.value as Settings["preset"])
              }
            >
              <option value="windowsFamiliar">Windows familiar</option>
              <option value="macNative">Mac native</option>
            </select>
          </label>
          <div className="setting-explanation">
            Windows familiar converts common editing shortcuts to Command.
            Control shortcuts remain unchanged in recognized terminals. Windows
            key acts as Command in Mac native mode.
          </div>
          <label className="field padded">
            Terminal exceptions
            <small>
              One macOS application bundle ID per line. Includes Terminal and
              iTerm2.
            </small>
            <textarea
              rows={3}
              value={draft.terminalApps.join("\n")}
              onChange={(e) =>
                patch(
                  "terminalApps",
                  e.target.value.split("\n").filter(Boolean),
                )
              }
            />
          </label>
          <div className="shortcut-grid">
            {[
              { scan: 0x2e, label: "Copy", key: "C" },
              { scan: 0x2f, label: "Paste", key: "V" },
              { scan: 0x2d, label: "Cut", key: "X" },
              { scan: 0x1e, label: "Select all", key: "A" },
              { scan: 0x2c, label: "Undo", key: "Z" },
              { scan: 0x1f, label: "Save", key: "S" },
            ].map((rule) => (
              <label key={rule.scan}>
                <span>
                  {rule.label}
                  <small>
                    Ctrl+{rule.key} → ⌘{rule.key}
                  </small>
                </span>
                <input
                  type="checkbox"
                  checked={draft.shortcuts.some(
                    (s) => s.scanCode === rule.scan && s.command,
                  )}
                  disabled={draft.preset !== "windowsFamiliar"}
                  onChange={(e) =>
                    patch("shortcuts", [
                      ...draft.shortcuts.filter(
                        (s) => s.scanCode !== rule.scan,
                      ),
                      ...(e.target.checked
                        ? [{ scanCode: rule.scan, command: true }]
                        : []),
                    ])
                  }
                />
              </label>
            ))}
          </div>
          <label className="setting-row">
            <span>
              <strong>Switch Mac input language</strong>
              <small>
                English and Russian input sources must be enabled on the Mac.
              </small>
            </span>
            <select
              value={draft.languageSwitch}
              onChange={(e) =>
                patch(
                  "languageSwitch",
                  e.target.value as Settings["languageSwitch"],
                )
              }
            >
              <option value="winSpace">Win+Space</option>
              <option value="ctrlSpace">Ctrl+Space</option>
            </select>
          </label>
          <div className="setting-explanation">
            English / Russian: the Mac’s selected input language controls
            typing. Use{" "}
            <kbd>{draft.languageSwitch === "winSpace" ? "Win" : "Ctrl"}</kbd> +{" "}
            <kbd>Space</kbd> to switch its input source. Add both languages in
            macOS Keyboard settings.
          </div>
        </div>
      </section>
      <section className="settings-section">
        <h2>
          <MousePointer2 size={18} />
          Pointer & switching
        </h2>
        <div className="settings-body">
          <label className="setting-row">
            <span>
              <strong>Pointer speed</strong>
              <small>Speed on the Mac relative to incoming movement.</small>
            </span>
            <div className="range-field">
              <input
                aria-label="Pointer speed"
                type="range"
                min="0.25"
                max="3"
                step="0.05"
                value={draft.pointerSpeed}
                onChange={(e) => patch("pointerSpeed", Number(e.target.value))}
              />
              <output>{draft.pointerSpeed.toFixed(2)}×</output>
            </div>
          </label>
          <Toggle
            label="Reverse remote scrolling"
            checked={draft.reverseScroll}
            onChange={(v) => patch("reverseScroll", v)}
          />
          <label className="setting-row">
            <span>
              <strong>Edge resistance</strong>
              <small>Pause at an edge before switching computers.</small>
            </span>
            <select
              value={draft.edgeResistanceMs}
              onChange={(e) =>
                patch("edgeResistanceMs", Number(e.target.value))
              }
            >
              <option value={0}>Immediate</option>
              <option value={100}>100 ms · Default</option>
              <option value={200}>200 ms</option>
              <option value={350}>350 ms</option>
            </select>
          </label>
          <Toggle
            label="Hotkey-only switching"
            detail="Disable screen edges. Use Ctrl+Alt+Shift+Right to switch."
            checked={draft.hotkeyOnly}
            onChange={(v) => patch("hotkeyOnly", v)}
          />
          <label className="setting-row">
            <span>
              <strong>Emergency release</strong>
              <small>
                Always handled locally. Pauses sharing and releases remote
                input.
              </small>
            </span>
            <select
              value={draft.emergencyKey}
              onChange={(e) => patch("emergencyKey", Number(e.target.value))}
            >
              <option value={1}>Ctrl+Alt+Shift+Escape</option>
              <option value={88}>Ctrl+Alt+Shift+F12</option>
            </select>
          </label>
        </div>
      </section>
      {state.os === "macos" && (
        <section className="settings-section">
          <h2>
            <ShieldCheck size={18} />
            Local Mac input
          </h2>
          <div className="settings-body">
            <Toggle
              label="Allow local Mac keyboard and trackpad"
              detail="When off, block ordinary local input only while Windows controls this Mac. Emergency release remains available."
              checked={draft.allowLocalInput}
              onChange={(v) => patch("allowLocalInput", v)}
            />
            <div className="setting-explanation">
              {state.permissions.filtering
                ? "Input permissions are granted. Native filtering must also be healthy before remote control can start."
                : "Accessibility and Input Monitoring are required. Grant permissions and restart FlowLink."}
            </div>
            <div className="padded">
              <button onClick={() => void run("request_permissions")}>
                Review permissions <ArrowUpRight size={14} />
              </button>
            </div>
          </div>
        </section>
      )}
      <section className="settings-section">
        <h2>
          <Wifi size={18} />
          Network & trust
        </h2>
        <div className="settings-body">
          <label className="setting-row">
            <span>
              <strong>Network interface</strong>
              <small>
                Use this interface for connections and discovery. Healthy
                sessions stay on their current route.
              </small>
            </span>
            <select
              value={draft.interface ?? ""}
              onChange={(e) => patch("interface", e.target.value || null)}
            >
              <option value="">Available LAN interfaces</option>
              {[...new Set(state.addresses.map((a) => a.interface))].map(
                (name) => (
                  <option key={name}>{name}</option>
                ),
              )}
            </select>
          </label>
          {state.devices
            .filter((d) => d.trusted)
            .map((device) => (
              <div className="trusted-setting" key={device.id}>
                <div>
                  <strong>{device.name}</strong>
                  <small className="mono">
                    Identity {device.id.slice(0, 16)}
                  </small>
                </div>
                <label className="auto-connect">
                  <input
                    type="checkbox"
                    checked={device.autoConnect}
                    onChange={(e) =>
                      void run("set_device", {
                        id: device.id,
                        favorite: device.favorite,
                        autoConnect: e.target.checked,
                      })
                    }
                  />
                  Auto-connect
                </label>
                <button
                  className="danger-text"
                  onClick={() => setConfirmForget(device)}
                >
                  Forget
                </button>
              </div>
            ))}
          {!state.devices.some((d) => d.trusted) && (
            <p className="setting-explanation">
              No trusted devices yet. Every new computer requires pairing
              approval.
            </p>
          )}
        </div>
      </section>
      <section className="settings-section">
        <h2>
          <ShieldCheck size={18} />
          Diagnostics
        </h2>
        <div className="settings-body">
          <Toggle
            label="Enable local diagnostics"
            detail="Show a redacted health summary. Never records typed keys, pairing codes, names, IPs, or clipboard content."
            checked={draft.diagnostics}
            onChange={(value) => patch("diagnostics", value)}
          />
          {state.settings.diagnostics && (
            <div className="padded">
              <button
                onClick={async () => {
                  const value = await run<string>("diagnostics");
                  if (value) setReport(value);
                }}
              >
                Refresh health summary
              </button>
              {report && <pre className="diagnostic-report">{report}</pre>}
            </div>
          )}
        </div>
      </section>
      <section className="settings-section">
        <h2>
          <CircleHelp size={18} />
          About this build
        </h2>
        <div className="settings-body">
          <p className="setting-explanation">
            FlowLink 0.1 development build. Windows → macOS only. Secure OS
            screens, reverse control, clipboard sharing, and gestures are
            outside this release. Physical-device acceptance tests and a pairing
            security review are required before public release.
          </p>
        </div>
      </section>
      {confirmForget && (
        <Modal title="Forget this device?" close={() => setConfirmForget(null)}>
          <p>
            {confirmForget.name} will lose its saved trust and display
            arrangement. An active connection will end. Pair again to reconnect.
          </p>
          <div className="modal-actions">
            <button onClick={() => setConfirmForget(null)}>Cancel</button>
            <button
              className="danger"
              onClick={() => {
                void run("forget_device", { id: confirmForget.id });
                setConfirmForget(null);
              }}
            >
              Forget device
            </button>
          </div>
        </Modal>
      )}
    </>
  );
}
