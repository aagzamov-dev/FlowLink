import {
  Check,
  Download,
  LoaderCircle,
  RefreshCw,
  RotateCw,
} from "lucide-react";
import type { UpdateCommand, UpdateStatus } from "../updates";
import { Toggle } from "./common";

export function Updates({
  update,
  desktop,
  automatic,
  onAutomatic,
  run,
}: {
  update: UpdateStatus;
  desktop: boolean;
  automatic: boolean;
  onAutomatic: (enabled: boolean) => void;
  run: (command: UpdateCommand) => Promise<void>;
}) {
  const busy = ["checking", "downloading", "installing"].includes(update.phase);
  const titles: Record<UpdateStatus["phase"], string> = {
    idle: "Keep FlowLink up to date",
    checking: "Checking for updates…",
    current: "You’re up to date",
    available: `FlowLink ${update.version} is available`,
    downloading: "Downloading your update…",
    ready: "Your update is ready",
    installing: "Installing and restarting…",
    error: "Couldn’t check for updates",
  };
  const action =
    update.phase === "available"
      ? ({
          label: "Download update",
          command: "download_update",
          icon: <Download size={15} />,
        } as const)
      : update.phase === "ready"
        ? ({
            label: "Install & restart",
            command: "install_update",
            icon: <RotateCw size={15} />,
          } as const)
        : ({
            label: "Check for updates",
            command: "check_for_updates",
            icon: <RefreshCw size={15} />,
          } as const);
  const percent =
    update.total && update.total > 0
      ? Math.min(100, Math.round((update.downloaded / update.total) * 100))
      : undefined;
  return (
    <section className="settings-section" aria-labelledby="updates-title">
      <h2 id="updates-title">
        <RefreshCw size={18} /> App updates
      </h2>
      <div className="settings-body">
        <div className="update-summary">
          <span
            className={`update-symbol ${update.phase === "current" ? "current" : ""}`}
            aria-hidden="true"
          >
            {busy ? (
              <LoaderCircle className="spin" size={23} />
            ) : update.phase === "current" ? (
              <Check size={23} />
            ) : (
              <Download size={23} />
            )}
          </span>
          <div className="update-copy" aria-live="polite" role="status">
            <strong>{titles[update.phase]}</strong>
            <small>Installed version {update.currentVersion}</small>
            {update.checkedAt && (
              <small>
                Last checked{" "}
                {new Date(update.checkedAt * 1000).toLocaleString()}
              </small>
            )}
          </div>
          <button
            className={
              update.phase === "available" || update.phase === "ready"
                ? "primary"
                : ""
            }
            disabled={!desktop || busy}
            onClick={() => void run(action.command)}
          >
            {busy ? <LoaderCircle className="spin" size={15} /> : action.icon}
            {busy
              ? update.phase === "checking"
                ? "Checking…"
                : update.phase === "downloading"
                  ? "Downloading…"
                  : "Installing…"
              : action.label}
          </button>
        </div>
        {update.error && (
          <div className="update-error" role="alert">
            <p>{update.error}</p>
            {update.phase === "available" && (
              <button
                className="text-button"
                onClick={() => void run("check_for_updates")}
              >
                Check again
              </button>
            )}
          </div>
        )}
        {update.phase === "downloading" && (
          <div className="update-progress">
            <progress aria-label="Update download" max={100} value={percent} />
            <span>
              {(update.downloaded / 1_048_576).toFixed(1)} MB
              {update.total
                ? ` of ${(update.total / 1_048_576).toFixed(1)} MB`
                : " downloaded"}{" "}
              ·{" "}
              {percent === 100
                ? "Verifying…"
                : percent !== undefined
                  ? `${percent}%`
                  : "Downloading…"}
            </span>
          </div>
        )}
        {update.notes && (
          <details className="update-notes">
            <summary>What’s new in {update.version}</summary>
            <p>{update.notes}</p>
          </details>
        )}
        <Toggle
          label="Automatically check for updates"
          detail="Checks at startup and every 6 hours. You choose when to download and install."
          checked={automatic}
          onChange={onAutomatic}
        />
        <p className="setting-explanation">
          {!desktop ? "Open the desktop app to check for updates. " : ""}
          Updates are verified before installation. Installing pauses sharing
          and restarts this computer’s app. Update both computers, then resume
          sharing.
        </p>
      </div>
    </section>
  );
}
