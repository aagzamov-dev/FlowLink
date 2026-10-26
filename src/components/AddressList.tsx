import { useState } from "react";
import { ChevronDown, Copy, EthernetPort, Link2, Wifi } from "lucide-react";
import type { Snapshot } from "../types";
import { endpoint } from "../layout";

export function AddressList({
  state,
  copy,
  compact = false,
}: {
  state: Snapshot;
  copy: (ip: string, withPort?: boolean) => Promise<void>;
  compact?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const addresses = expanded ? state.addresses : state.addresses.slice(0, 1);
  return (
    <section className={`address-card ${compact ? "compact" : ""}`}>
      <div className="section-label">
        <span>THIS COMPUTER</span>
        {state.addresses.length > 1 && (
          <button
            className="text-button"
            onClick={() => setExpanded(!expanded)}
            aria-expanded={expanded}
          >
            {expanded ? "Less" : `${state.addresses.length} addresses`}
            <ChevronDown size={12} />
          </button>
        )}
      </div>
      {addresses.length === 0 ? (
        <div className="address-empty">
          <Wifi size={18} />
          <span>No local network address available</span>
        </div>
      ) : (
        addresses.map((address) => (
          <div
            className="address-row"
            key={`${address.interface}-${address.ip}`}
          >
            <EthernetPort size={18} />
            <div>
              <strong className="mono">
                {state.port !== 0 && state.port !== 47831
                  ? endpoint(address.ip, state.port)
                  : address.ip}
              </strong>
              <small>
                {address.interface} · {address.kind}
              </small>
            </div>
            <button
              className="icon-button"
              aria-label={`Copy ${state.port !== 47831 ? endpoint(address.ip, state.port) : address.ip}`}
              title={state.port !== 47831 ? "Copy IP and port" : "Copy IP address"}
              disabled={state.port === 0}
              onClick={() => void copy(address.ip, state.port !== 47831)}
            >
              <Copy size={16} />
            </button>
            <button
              className="icon-button"
              aria-label={`Copy connection address for ${address.ip}`}
              title="Copy IP and port"
              disabled={state.port === 0}
              onClick={() => void copy(address.ip, true)}
            >
              <Link2 size={16} />
            </button>
          </div>
        ))
      )}
      {state.port === 0 ? (
        <p className="footnote">Waiting for a connection port…</p>
      ) : state.port !== 47831 ? (
        <p className="footnote">
          Using TCP port {state.port}. For manual connections, copy the IP and
          port with the link button.
        </p>
      ) : !compact && (
        <p className="footnote">
          Addresses update automatically when your network changes.
        </p>
      )}
    </section>
  );
}
