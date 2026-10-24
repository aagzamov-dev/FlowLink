import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { version } from "../package.json";

export const appVersion = version;
export type UpdatePhase =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "ready"
  | "installing"
  | "error";
export interface UpdateStatus {
  phase: UpdatePhase;
  currentVersion: string;
  version: string | null;
  notes: string | null;
  downloaded: number;
  total: number | null;
  checkedAt: number | null;
  error: string | null;
}
export type UpdateCommand =
  "check_for_updates" | "download_update" | "install_update";
export const initialUpdate: UpdateStatus = {
  phase: "idle",
  currentVersion: appVersion,
  version: null,
  notes: null,
  downloaded: 0,
  total: null,
  checkedAt: null,
  error: null,
};

export function useUpdates() {
  const [update, setUpdate] = useState<UpdateStatus>(initialUpdate);
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let receivedEvent = false;
    const events = listen<UpdateStatus>("update-state", (event) => {
      receivedEvent = true;
      if (!disposed) setUpdate(event.payload);
    });
    void events
      .then(() => invoke<UpdateStatus>("update_status"))
      .then((value) => {
        if (!disposed && !receivedEvent) setUpdate(value);
      })
      .catch(() => {
        if (!disposed)
          setUpdate((value) => ({
            ...value,
            error:
              "Update status is unavailable. Reopen FlowLink and try again.",
          }));
      });
    return () => {
      disposed = true;
      void events.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);
  const runUpdate = async (command: UpdateCommand) => {
    if (!isTauri()) return;
    try {
      await invoke(command);
    } catch (error: unknown) {
      setUpdate((current) => ({ ...current, error: String(error) }));
    }
  };
  return { update, runUpdate };
}
