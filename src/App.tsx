import { useEffect, useState } from "react";
import {
  ArrowUpRight,
  Check,
  ChevronRight,
  CircleHelp,
  Link2,
  Monitor,
  MousePointer2,
  Pause,
  Play,
  Plus,
  Settings2,
  ShieldCheck,
  Unplug,
  X,
} from "lucide-react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Device, Page, Snapshot, Run } from "./types";
import { endpoint, sortedDevices } from "./layout";
import { Badge, Brand } from "./components/common";
import { AddressList } from "./components/AddressList";
import { ManualConnect, Pairing } from "./components/Pairing";
import { Devices } from "./features/Devices";
import { Displays } from "./features/Displays";
import { Preferences } from "./features/Preferences";
import { preview } from "./types";
const isDesktop = isTauri();
const quick = new URLSearchParams(location.search).get("panel") === "quick";

export default function App() {
  const [state, setState] = useState<Snapshot>(preview);
  const [page, setPage] = useState<Page>("devices");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  const [manual, setManual] = useState(false);
  useEffect(() => {
    if (!isDesktop) return;
    let disposed = false;
    const events = listen<Snapshot>("engine-state", (event) => {
      if (!disposed) setState(event.payload);
    });
    const navigation = listen<string>("navigate", (event) => {
      if (["devices", "displays", "settings"].includes(event.payload))
        setPage(event.payload as Page);
    });
    void invoke<Snapshot>("snapshot")
      .then((value) => {
        if (!disposed) setState(value);
      })
      .catch((e: unknown) => setError(String(e)));
    return () => {
      disposed = true;
      void events.then((unlisten) => unlisten());
      void navigation.then((unlisten) => unlisten());
    };
  }, []);
  const run: Run = async <T,>(
    command: string,
    args?: Record<string, unknown>,
  ): Promise<T | undefined> => {
    if (!isDesktop) {
      setError(
        "Open the FlowLink desktop app to use this feature. The browser shows the interface only.",
      );
      return;
    }
    setError(null);
    setWorking(true);
    try {
      return await invoke<T>(command, args);
    } catch (e: unknown) {
      setError(String(e));
      return undefined;
    } finally {
      setWorking(false);
    }
  };
  const navigate = (next: Page) => {
    if (quick) void run("open_main", { page: next });
    else setPage(next);
  };
  const copy = async (ip: string, withPort = false) => {
    if (!isDesktop) return;
    try {
      await invoke("copy_address", { ip, withPort });
      setNotice(withPort ? "Connection address copied" : "IP address copied");
    } catch (e: unknown) {
      setError(String(e));
    }
  };
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(null), 2200);
    return () => clearTimeout(timer);
  }, [notice]);
  const connected = state.devices.find((d) => d.id === state.connectedId);
  const remote = state.ownership === "remote";
  const paused = state.settings.paused || state.peerPaused;
  const connectDevice = (device: Device) => {
    const ip = device.addresses[0];
    if (ip) void run("connect", { address: endpoint(ip, device.port) });
  };
  const banner = error ?? state.error;
  if (quick)
    return (
      <div className="quick-panel">
        <header className="quick-header">
          <Brand />
          <div className="quick-header-actions">
            <button
              className="icon-button"
              aria-label="Disconnect device"
              disabled={!state.connectedId}
              onClick={() => void run("disconnect")}
            >
              <Unplug size={18} />
            </button>
            <button
              className="icon-button"
              aria-label="Open settings"
              onClick={() => navigate("settings")}
            >
              <Settings2 size={18} />
            </button>
          </div>
        </header>
        <div className="quick-status">
          <span
            className={`connection-dot ${state.connectedId ? "connected" : ""}`}
          />
          <div>
            <strong>{connected?.name ?? "Ready when you are"}</strong>
            <small>
              {paused
                ? "Sharing paused"
                : remote
                  ? "Controlling Mac"
                  : state.connectedId
                    ? "Connected · Windows has control"
                    : "Choose a computer below"}
            </small>
          </div>
        </div>
        {banner && (
          <div className="alert compact" role="alert">
            {banner}
          </div>
        )}
        <div className="quick-actions">
          <button
            className="primary"
            disabled={!state.connectedId || state.peerPaused}
            onClick={() => void run("pause", { paused: !paused })}
          >
            {paused ? <Play size={16} /> : <Pause size={16} />}
            {state.peerPaused ? "Paused on peer" : paused ? "Resume" : "Pause"}
          </button>
          <button
            disabled={!state.connectedId}
            onClick={() => void run("return_local")}
          >
            <MousePointer2 size={16} />
            Return here
          </button>
        </div>
        <div className="section-label">
          YOUR COMPUTERS
          <button className="text-button" onClick={() => navigate("devices")}>
            View all <ChevronRight size={13} />
          </button>
        </div>
        <div className="quick-devices">
          {sortedDevices(state.devices.filter((d) => d.trusted))
            .slice(0, 5)
            .map((device) => (
              <button
                className="quick-device"
                key={device.id}
                disabled={!!state.connectedId || state.os !== "windows"}
                onClick={() => connectDevice(device)}
              >
                <span className="device-icon">
                  <Monitor size={19} />
                </span>
                <span>
                  <strong>{device.name}</strong>
                  <small>
                    {device.id === state.connectedId
                      ? "Connected"
                      : device.online
                        ? "Available"
                        : "Offline · try connection"}
                  </small>
                </span>
                {device.id === state.connectedId ? (
                  <Check size={17} className="accent" />
                ) : (
                  <ArrowUpRight size={16} />
                )}
              </button>
            ))}
          {!state.devices.some((d) => d.trusted) && (
            <p className="muted quick-empty">
              Your paired computers will appear here.
            </p>
          )}
        </div>
        <AddressList state={state} copy={copy} compact />
        <footer className="quick-footer">
          <button
            className="text-button"
            onClick={() => {
              setManual(true);
            }}
          >
            <Plus size={16} />
            Connect by IP
          </button>
          <button className="text-button" onClick={() => navigate("displays")}>
            <Monitor size={16} />
            Displays
          </button>
        </footer>
        {notice && (
          <div className="toast" role="status">
            <Check size={16} />
            {notice}
          </div>
        )}
        {manual && (
          <ManualConnect
            run={run}
            close={() => setManual(false)}
            canConnect={state.os === "windows"}
          />
        )}
        {state.pairing && (
          <Pairing key={state.pairing.requestId} state={state} run={run} />
        )}
      </div>
    );
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <Brand />
        <div className="workspace-label">YOUR WORKSPACE</div>
        <nav aria-label="Main navigation">
          {(
            [
              { id: "devices", label: "Devices", icon: <Link2 /> },
              { id: "displays", label: "Displays", icon: <Monitor /> },
              { id: "settings", label: "Settings", icon: <Settings2 /> },
            ] as const
          ).map((item) => (
            <button
              key={item.id}
              className={page === item.id ? "selected" : ""}
              aria-current={page === item.id ? "page" : undefined}
              onClick={() => navigate(item.id)}
            >
              {item.icon}
              {item.label}
              {page === item.id && <span className="nav-dot" />}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="privacy">
            <ShieldCheck size={16} />
            <span>
              Private by design<small>Local network. No cloud.</small>
            </span>
          </div>
          <div className="version">
            <span className="mini-logo">F</span>FlowLink{" "}
            <span>0.1 · Development</span>
          </div>
        </div>
      </aside>
      <main>
        <div className="topbar">
          <span>
            <i className={`tiny-dot ${state.connectedId ? "online" : ""}`} />
            {state.os === "preview"
              ? "Interface preview"
              : state.settings.deviceName}
          </span>
          <Badge active={!!state.connectedId && !paused}>
            {paused
              ? "Paused"
              : state.connectedId
                ? "Connected"
                : "Not connected"}
          </Badge>
        </div>
        <div className="page-content">
          {!isDesktop && (
            <div className="preview-note">
              <CircleHelp size={16} />
              Interface preview. Real devices and input control are available in
              the desktop app.
            </div>
          )}
          {banner && (
            <div className="alert" role="alert">
              <CircleHelp size={18} />
              <span>{banner}</span>
              {error && (
                <button
                  className="icon-button"
                  aria-label="Dismiss error"
                  onClick={() => setError(null)}
                >
                  <X size={15} />
                </button>
              )}
            </div>
          )}
          {state.os === "macos" &&
            (!state.permissions.accessibility ||
              !state.permissions.monitoring) && (
              <div className="permission-banner">
                <ShieldCheck />
                <div>
                  <strong>One more step to receive input</strong>
                  <p>
                    Allow Accessibility and Input Monitoring in System Settings,
                    then restart FlowLink.
                  </p>
                </div>
                <button onClick={() => void run("request_permissions")}>
                  Open permissions <ArrowUpRight size={14} />
                </button>
              </div>
            )}
          {page === "devices" && (
            <Devices
              state={state}
              run={run}
              working={
                working || state.status === "Connecting…" || !!state.pairing
              }
              manual={() => setManual(true)}
              connectDevice={connectDevice}
              navigate={navigate}
              copy={copy}
            />
          )}
          {page === "displays" && <Displays state={state} run={run} />}
          {page === "settings" && <Preferences state={state} run={run} />}
        </div>
        <footer className="statusbar">
          <span>
            <ShieldCheck size={13} />
            {state.connectedId
              ? "Authenticated · encrypted connection"
              : "Your input stays on this computer"}
          </span>
          <span>
            {remote ? "Mac has control" : "Local control"}
            <span className="status-divider">·</span>
            {state.latencyMs === null
              ? "LAN only"
              : `${state.latencyMs.toFixed(1)} ms round trip`}
          </span>
        </footer>
      </main>
      {manual && (
        <ManualConnect
          run={run}
          close={() => setManual(false)}
          canConnect={state.os === "windows"}
        />
      )}
      {state.pairing && (
        <Pairing key={state.pairing.requestId} state={state} run={run} />
      )}
      {notice && (
        <div className="toast" role="status">
          <Check size={16} />
          {notice}
        </div>
      )}
    </div>
  );
}
