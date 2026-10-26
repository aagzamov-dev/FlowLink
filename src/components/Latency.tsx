import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Snapshot } from "../types";

export function Latency() {
  const [latency, setLatency] = useState<number | null>(null);
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let received = false;
    const events = listen<number | null>("engine-latency", ({ payload }) => {
      received = true;
      if (!disposed) setLatency(payload);
    });
    void events
      .then(() => invoke<Snapshot>("snapshot"))
      .then((state) => {
        if (!disposed && !received) setLatency(state.latencyMs);
      })
      .catch(() => {});
    return () => {
      disposed = true;
      void events.then((unlisten) => unlisten());
    };
  }, []);
  return (
    <>
      {latency === null
        ? "LAN / direct IP"
        : `${latency.toFixed(1)} ms round trip`}
    </>
  );
}
