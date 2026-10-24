import { useState } from "react";
import type { FormEvent } from "react";
import { Link2, LoaderCircle, ShieldCheck } from "lucide-react";
import type { Snapshot, Run } from "../types";
import { Modal } from "./common";

export function ManualConnect({
  run,
  close,
  canConnect,
}: {
  run: Run;
  close: () => void;
  canConnect: boolean;
}) {
  const [address, setAddress] = useState("");
  const [sending, setSending] = useState(false);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!address.trim()) return;
    setSending(true);
    const result = await run<null>("connect", { address: address.trim() });
    setSending(false);
    if (result !== undefined) close();
  };
  return (
    <Modal title="Connect by address" close={close}>
      <p>
        Find the other computer’s IP in its FlowLink tray panel. Both computers
        must be reachable on your local network.
      </p>
      <form onSubmit={(e) => void submit(e)}>
        <label className="field">
          IP address or hostname
          <input
            autoFocus
            placeholder="192.168.1.20 or MacBook.local"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            maxLength={253}
            required
          />
        </label>
        <p className="footnote">
          The default port is 47831. You can also enter an address with a port.
        </p>
        {!canConnect && (
          <p className="info-note">
            Disconnect the current device before making another connection.
          </p>
        )}
        <div className="modal-actions">
          <button type="button" onClick={close}>
            Cancel
          </button>
          <button
            className="primary"
            disabled={sending || !canConnect || !address.trim()}
          >
            {sending ? (
              <LoaderCircle className="spin" size={16} />
            ) : (
              <Link2 size={16} />
            )}
            Connect
          </button>
        </div>
      </form>
    </Modal>
  );
}

export function Pairing({ state, run }: { state: Snapshot; run: Run }) {
  const pairing = state.pairing!;
  const [code, setCode] = useState("");
  const [sent, setSent] = useState(false);
  const answer = async (answer: string) => {
    setSent(true);
    await run("answer_pairing", { id: pairing.requestId, answer });
  };
  return (
    <Modal
      title={
        pairing.code
          ? "Your pairing code"
          : pairing.awaitingCode
            ? "Enter pairing code"
            : "Allow this computer to pair?"
      }
      close={() => {
        if (pairing.code) void run("cancel_pairing");
        else void answer("");
      }}
    >
      <div className="pair-device">
        <span className="empty-icon">
          <ShieldCheck />
        </span>
        <strong>{pairing.name}</strong>
        <small className="mono">{pairing.address}</small>
      </div>
      {pairing.code ? (
        <>
          <p>
            Enter this code on the connecting computer. Only approve a computer
            you recognize.
          </p>
          <div className="pair-code">{pairing.code}</div>
          <p className="footnote">
            Expires in two minutes. Keep this window open.
          </p>
        </>
      ) : pairing.awaitingCode ? (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void answer(code);
          }}
        >
          <p>Enter the six-digit code displayed on the other computer.</p>
          <input
            className="code-input"
            aria-label="Six-digit pairing code"
            inputMode="numeric"
            autoComplete="one-time-code"
            autoFocus
            pattern="[0-9]{6}"
            maxLength={6}
            value={code}
            onChange={(e) => setCode(e.target.value.replace(/\D/g, ""))}
          />
          <div className="modal-actions">
            <button type="button" onClick={() => void answer("")}>
              Cancel
            </button>
            <button className="primary" disabled={sent || code.length !== 6}>
              Pair securely
            </button>
          </div>
        </form>
      ) : (
        <>
          <p>
            This computer is asking to share keyboard and pointer control.
            Approve only if you started this connection.
          </p>
          <div className="modal-actions">
            <button onClick={() => void answer("")}>Decline</button>
            <button
              className="primary"
              disabled={sent}
              onClick={() => void answer("approve")}
            >
              Show pairing code
            </button>
          </div>
        </>
      )}
    </Modal>
  );
}
