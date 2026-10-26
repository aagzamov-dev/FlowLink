import { useState } from "react";
import {
  ArrowRight,
  ArrowUpRight,
  Check,
  Monitor,
  MousePointer2,
  Pause,
  Play,
  Plus,
  Radio,
  Search,
  Star,
  Unplug,
} from "lucide-react";
import type { Device, Page, Snapshot, Run } from "../types";
import { sortedDevices } from "../layout";
import { Badge, Empty } from "../components/common";
import { ConnectionGuide } from "../components/ConnectionGuide";
import { AddressList } from "../components/AddressList";

export function Devices({
  state,
  run,
  working,
  manual,
  connectDevice,
  navigate,
  copy,
}: {
  state: Snapshot;
  run: Run;
  working: boolean;
  manual: () => void;
  connectDevice: (d: Device) => void;
  navigate: (p: Page) => void;
  copy: (ip: string, withPort?: boolean) => Promise<void>;
}) {
  const [query, setQuery] = useState("");
  const devices = sortedDevices(state.devices, query);
  const connected = state.devices.find((d) => d.id === state.connectedId);
  const isMain = state.controllerId === state.deviceId;
  return (
    <>
      <div className="page-heading">
        <div>
          <span className="eyebrow">ONE DESK. ONE SET OF CONTROLS.</span>
          <h1>Your devices</h1>
          <p>Move between your computers, without missing a beat.</p>
        </div>
        <button className="primary" onClick={manual}>
          <Plus size={17} />
          Connect by IP
        </button>
      </div>
      <section
        className={`connection-card ${state.ownership === "remote" ? "remote" : ""}`}
      >
        <div className="connection-art">
          <span className="computer main-computer">
            <Monitor size={31} />
          </span>
          <div className="connection-line">
            <i />
            <i />
            <i />
          </div>
          <span
            className={`computer target-computer ${connected ? "available" : ""}`}
          >
            <Monitor size={27} />
          </span>
        </div>
        <div className="connection-copy">
          <Badge active={!!connected}>
            {state.settings.paused
              ? "Paused"
              : connected
                ? "Connected securely"
                : "A little less switching"}
          </Badge>
          <h2>
            {connected
              ? `Connected to ${connected.name}`
              : "Two computers. One smooth flow."}
          </h2>
          <p>
            {connected
              ? state.ownership === "remote"
                ? isMain
                  ? "Your keyboard and pointer control the other computer."
                  : "The other computer is controlling this device."
                : isMain
                  ? "This is the main device. Switch control using a screen edge or the button below."
                  : "This device receives input. You can change the main device below."
              : "Open FlowLink on both computers. Select a nearby device or enter its IP address."}
          </p>
          <div className="card-actions">
            {connected ? (
              <>
                <button
                  className="primary"
                  disabled={state.peerPaused && !state.settings.paused}
                  onClick={() =>
                    void run("pause", { paused: !state.settings.paused })
                  }
                >
                  {state.settings.paused ? (
                    <Play size={15} />
                  ) : (
                    <Pause size={15} />
                  )}
                  {state.settings.paused
                    ? "Resume here"
                    : state.peerPaused
                      ? "Paused on other computer"
                      : "Pause sharing"}
                </button>
                {(isMain ||
                  state.controlFromEither ||
                  state.ownership === "remote") && (
                  <button onClick={() => void run("switch_control")}>
                    <MousePointer2 size={15} />
                    {state.ownership === "remote"
                      ? "Release control"
                      : isMain
                        ? "Control other computer"
                        : "Make this the main device"}
                  </button>
                )}
                <button
                  className="text-button"
                  onClick={() => void run("disconnect")}
                >
                  <Unplug size={15} />
                  Disconnect
                </button>
              </>
            ) : state.settings.paused ? (
              <button
                className="primary"
                onClick={() => void run("pause", { paused: false })}
              >
                <Play size={15} /> Resume here
              </button>
            ) : (
              <span className="inline-hint">
                <Radio size={15} />
                {state.status === "Connecting…"
                  ? "Connecting to your computer…"
                  : "Looking for FlowLink devices on your network"}
              </span>
            )}
          </div>
        </div>
      </section>
      <ConnectionGuide
        state={state}
        run={run}
        navigate={navigate}
        manual={manual}
      />
      {connected && (
        <section className="settings-section">
          <h2>Main device</h2>
          <div className="settings-body">
            <label className="setting-row">
              <span>
                <strong>Keyboard and pointer source</strong>
                <small>
                  Changing the main device releases input first. Switch control
                  when ready.
                </small>
              </span>
              <select
                aria-label="Main device"
                disabled={!isMain && !state.controlFromEither}
                value={state.controllerId ?? state.deviceId}
                onChange={(e) =>
                  void run("configure_control", {
                    controllerId: e.target.value,
                    fromEither: state.controlFromEither,
                  })
                }
              >
                <option value={state.deviceId}>
                  {state.settings.deviceName} (this device)
                </option>
                <option value={connected.id}>{connected.name}</option>
              </select>
            </label>
            <label className="setting-row">
              <span>
                <strong>Who can change the main device?</strong>
              </span>
              <select
                aria-label="Control switching policy"
                disabled={!isMain && !state.controlFromEither}
                value={state.controlFromEither ? "either" : "main"}
                onChange={(e) =>
                  void run("configure_control", {
                    controllerId: state.controllerId,
                    fromEither: e.target.value === "either",
                  })
                }
              >
                <option value="either">Either computer</option>
                <option value="main">Main device only</option>
              </select>
            </label>
          </div>
        </section>
      )}
      <div className="section-heading">
        <h2>
          Computers <span className="count">{devices.length}</span>
        </h2>
        <label className="search">
          <Search size={16} />
          <input
            aria-label="Search devices"
            placeholder="Search name or IP"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
      </div>
      {(["trusted", "nearby"] as const).map((group) => {
        const list = devices.filter((d) => d.trusted === (group === "trusted"));
        return (
          list.length > 0 && (
            <section key={group} className="device-group">
              <div className="section-label">
                {group === "trusted"
                  ? "TRUSTED DEVICES"
                  : "NEARBY · PAIRING REQUIRED"}
              </div>
              {list.map((device) => (
                <div className="device-row" key={device.id}>
                  <span
                    className={`device-icon ${device.online ? "available" : ""}`}
                  >
                    <Monitor size={23} />
                  </span>
                  <div className="device-description">
                    <strong>
                      {device.name}
                      <span className="platform-tag">
                        {device.os === "macos" ? "macOS" : "Windows"}
                      </span>
                    </strong>
                    <small>
                      {device.addresses[0] ?? "Address unavailable"}{" "}
                      <span>·</span> {device.id.slice(0, 8)} <span>·</span>{" "}
                      {state.connectedId === device.id
                        ? "Connected"
                        : device.busy
                          ? "Busy"
                          : device.online
                            ? "Available"
                            : "Offline"}
                    </small>
                  </div>
                  {device.trusted && (
                    <button
                      className={`icon-button favorite ${device.favorite ? "is-favorite" : ""}`}
                      aria-label={`${device.favorite ? "Unfavorite" : "Favorite"} ${device.name}`}
                      aria-pressed={device.favorite}
                      onClick={() =>
                        void run("set_device", {
                          id: device.id,
                          favorite: !device.favorite,
                          autoConnect: device.autoConnect,
                        })
                      }
                    >
                      <Star
                        size={17}
                        fill={device.favorite ? "currentColor" : "none"}
                      />
                    </button>
                  )}
                  <button
                    disabled={working || !!state.connectedId || device.busy}
                    onClick={() => connectDevice(device)}
                  >
                    {state.connectedId === device.id ? (
                      <>
                        <Check size={14} />
                        Connected
                      </>
                    ) : device.trusted ? (
                      <>
                        Connect <ArrowUpRight size={14} />
                      </>
                    ) : (
                      "Pair device"
                    )}
                  </button>
                </div>
              ))}
            </section>
          )
        );
      })}
      {devices.length === 0 && (
        <div className="empty-bordered">
          <Empty
            icon={<Radio />}
            title={
              query
                ? "No matching computers"
                : "Your next computer belongs here"
            }
          >
            {query ? (
              "Try another name or local IP address."
            ) : (
              <>
                Keep FlowLink open on both computers and use the same network.
                <br />
                Can’t find the other computer?{" "}
                <button className="text-button inline" onClick={manual}>
                  Connect by IP address
                </button>
              </>
            )}
          </Empty>
        </div>
      )}
      <div className="bottom-grid">
        <AddressList state={state} copy={copy} />
        <button className="layout-promo" onClick={() => navigate("displays")}>
          <span className="device-icon">
            <Monitor size={21} />
          </span>
          <span>
            <strong>Make it match your desk</strong>
            <small>Arrange your screens and choose where to cross.</small>
          </span>
          <ArrowRight size={18} />
        </button>
      </div>
    </>
  );
}
