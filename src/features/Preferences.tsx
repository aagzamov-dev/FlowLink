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
import { ShortcutEditor } from "../components/ShortcutEditor";
import { Toggle, Modal } from "../components/common";
import { Updates } from "../components/Updates";
import { equal, useDraft } from "../drafts";
import type { UpdateCommand, UpdateStatus } from "../updates";

export function Preferences({
  state,
  run,
  update,
  runUpdate,
  onDirty,
}: {
  state: Snapshot;
  run: Run;
  update: UpdateStatus;
  runUpdate: (command: UpdateCommand) => Promise<void>;
  onDirty: (dirty: boolean) => void;
}) {
  const { draft, setDraft, dirty, reset } = useDraft(state.settings);
  const [saving, setSaving] = useState(false);
  const [autostart, setAutostart] = useState(false);
  const [startupBusy, setStartupBusy] = useState(false);
  const [report, setReport] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [backupMessage, setBackupMessage] = useState<string | null>(null);
  const [importError, setImportError] = useState<string | null>(null);
  const [backupBusy, setBackupBusy] = useState(false);
  useEffect(() => {
    onDirty(dirty);
    return () => onDirty(false);
  }, [dirty, onDirty]);
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
    <fieldset className="settings-fields" disabled={saving || backupBusy}>
      <div className="page-heading">
        <div>
          <span className="eyebrow">MAKE IT FEEL LIKE YOURS</span>
          <h1>Settings</h1>
          <p>Simple controls for a smoother, safer workspace.</p>
        </div>
        <div className="card-actions">
          {dirty && (
            <button disabled={saving || backupBusy} onClick={reset}>
              Discard edits
            </button>
          )}
          <button
            className="primary"
            disabled={!dirty || saving || backupBusy}
            onClick={async () => {
              setSaving(true);
              const settings = {
                ...draft,
                terminalApps: draft.terminalApps
                  .map((app) => app.trim())
                  .filter(Boolean),
              };
              const result = await run<null>("save_settings", { settings });
              if (result !== undefined)
                setDraft((current) =>
                  equal(current, draft) ? settings : current,
                );
              setSaving(false);
            }}
          >
            {saving ? "Saving…" : "Save changes"}
          </button>
        </div>
      </div>
      <Updates
        update={update}
        desktop={isTauri()}
        automatic={draft.autoCheckUpdates}
        onAutomatic={(enabled) => patch("autoCheckUpdates", enabled)}
        run={runUpdate}
      />
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
            disabled={startupBusy || !isTauri()}
            onChange={async (enabled) => {
              setStartupBusy(true);
              const result = await run<null>("set_autostart", { enabled });
              if (result !== undefined) setAutostart(enabled);
              setStartupBusy(false);
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
              <small>
                Applied when this device controls the other computer.
              </small>
            </span>
            <select
              value={draft.preset}
              onChange={(e) =>
                patch("preset", e.target.value as Settings["preset"])
              }
            >
              <option value="windowsFamiliar">Adapt editing shortcuts</option>
              <option value="macNative">Physical keys</option>
              <option value="customModifiers">Custom modifier keys</option>
            </select>
          </label>
          <div className="setting-explanation">
            Adapt editing shortcuts converts Ctrl to Command on Mac and Command
            to Ctrl on Windows for enabled editing keys. Mac terminal exceptions
            preserve Control. Physical keys forwards the original keys.
          </div>
          {draft.preset === "customModifiers" && (
            <div className="padded">
              <p>
                Map Mac modifiers to Windows keys in both directions. Shift
                stays Shift. Explicit shortcut rules take priority.
              </p>
              {(["command", "option", "control"] as const).map((key) => (
                <label className="field" key={key}>
                  Mac {key} ↔ Windows
                  <select
                    value={draft.modifierMapping[key]}
                    onChange={(event) => {
                      const value = Number(event.target.value) as 1 | 4 | 8;
                      const mapping = { ...draft.modifierMapping };
                      const other = (
                        ["command", "option", "control"] as const
                      ).find((candidate) => mapping[candidate] === value);
                      if (other) mapping[other] = mapping[key];
                      mapping[key] = value;
                      patch("modifierMapping", mapping);
                    }}
                  >
                    <option value={1}>Ctrl</option>
                    <option value={4}>Alt</option>
                    <option value={8}>Win</option>
                  </select>
                </label>
              ))}
            </div>
          )}
          <label className="field padded">
            Terminal exceptions
            <small>
              One macOS application bundle ID per line. Includes Terminal and
              iTerm2.
            </small>
            <textarea
              rows={3}
              value={draft.terminalApps.join("\n")}
              onChange={(e) => {
                patch("terminalApps", e.target.value.split("\n"));
              }}
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
              { scan: 0x21, label: "Find", key: "F" },
              { scan: 0x18, label: "Open", key: "O" },
              { scan: 0x19, label: "Print", key: "P" },
              { scan: 0x31, label: "New", key: "N" },
            ].map((rule) => (
              <label key={rule.scan}>
                <span>
                  {rule.label}
                  <small>
                    Ctrl+{rule.key} ↔ ⌘{rule.key}
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
          <ShortcutEditor
            bindings={draft.bindings}
            onChange={(bindings) => patch("bindings", bindings)}
          />
          <label className="setting-row">
            <span>
              <strong>Windows → Mac language shortcut</strong>
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
            The receiving computer’s selected input language controls typing.
            When Windows controls Mac, use{" "}
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
              <small>
                Speed on the receiving device relative to incoming movement.
              </small>
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
      {state.os !== "preview" && (
        <section className="settings-section">
          <h2>
            <ShieldCheck size={18} />
            Local input
          </h2>
          <div className="settings-body">
            <Toggle
              label="Allow local keyboard and touchpad"
              detail="When off, block ordinary local input only while the other computer controls this device. Emergency release remains available."
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
              <option value="">Automatic · LAN / VPN</option>
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
              <button
                onClick={async () => {
                  const path = await run<string>("export_diagnostics");
                  if (path) setExportPath(path);
                }}
              >
                Save report to Downloads
              </button>
              {exportPath && (
                <p role="status" className="muted">
                  Saved to {exportPath}
                </p>
              )}
              {report && <pre className="diagnostic-report">{report}</pre>}
            </div>
          )}
        </div>
      </section>
      <section className="settings-section">
        <h2>
          <Settings2 size={18} />
          Preference backup
        </h2>
        <div className="settings-body padded">
          <p className="muted">
            Back up saved keyboard, pointer, and switching preferences. Import
            loads an editable draft; click Save changes to apply it. Device
            identity, trust, network, and startup settings stay on this
            computer.
          </p>
          <div className="card-actions">
            <button
              disabled={!isTauri() || backupBusy}
              onClick={async () => {
                setBackupBusy(true);
                setImportError(null);
                const path = await run<string>("export_preferences");
                if (path) setBackupMessage(`Saved to ${path}`);
                setBackupBusy(false);
              }}
            >
              Save backup to Downloads
            </button>
            <label className="backup-import">
              Import backup
              <input
                type="file"
                accept=".json,application/json"
                disabled={!isTauri() || backupBusy || saving}
                onChange={async (event) => {
                  const file = event.target.files?.[0];
                  event.target.value = "";
                  if (!file) return;
                  setImportError(null);
                  setBackupMessage(null);
                  if (file.size > 65536) {
                    setImportError("Preference backup exceeds 64 KiB.");
                    return;
                  }
                  setBackupBusy(true);
                  try {
                    const settings = await run<Settings>(
                      "preview_preferences_import",
                      { data: await file.text(), settings: state.settings },
                    );
                    if (settings) {
                      setDraft((current) => ({
                        ...current,
                        preset: settings.preset,
                        pointerSpeed: settings.pointerSpeed,
                        reverseScroll: settings.reverseScroll,
                        terminalApps: settings.terminalApps,
                        shortcuts: settings.shortcuts,
                        languageSwitch: settings.languageSwitch,
                        hotkeyOnly: settings.hotkeyOnly,
                        bindings: settings.bindings,
                        modifierMapping: settings.modifierMapping,
                        edgeResistanceMs: settings.edgeResistanceMs,
                        emergencyKey: settings.emergencyKey,
                      }));
                      setBackupMessage(
                        "Backup loaded. Review the draft, then save your changes.",
                      );
                    }
                  } catch {
                    setImportError(
                      "Could not read this backup file. Choose a JSON preference backup.",
                    );
                  } finally {
                    setBackupBusy(false);
                  }
                }}
              />
            </label>
          </div>
          {backupMessage && (
            <p role="status" className="muted">
              {backupMessage}
            </p>
          )}
          {importError && (
            <p role="alert" className="alert">
              {importError}
            </p>
          )}
        </div>
      </section>
      <section className="settings-section">
        <h2>
          <CircleHelp size={18} />
          About FlowLink
        </h2>
        <div className="settings-body">
          <p className="setting-explanation">
            Mac ↔ Windows keyboard, pointer, and scrolling. Choose the main
            device from Devices. Secure OS screens, clipboard sharing, and
            system multi-finger gestures are not forwarded. Both computers must
            use this protocol version.
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
              onClick={async () => {
                const result = await run<null>("forget_device", {
                  id: confirmForget.id,
                });
                if (result !== undefined) setConfirmForget(null);
              }}
            >
              Forget device
            </button>
          </div>
        </Modal>
      )}
    </fieldset>
  );
}
