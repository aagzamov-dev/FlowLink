# Protocol version 2

TCP port 47831. Frames: four-byte unsigned big-endian byte length followed by UTF-8 JSON. Accepted lengths: 1–65,536 bytes. No compression or clipboard/file payloads. Set TCP_NODELAY. Maintain one reader and one writer; a partial read is never cancelled and then resumed as a new frame. TLS 1.3 ALPN: `flowlink/2`.

## Establishment

1. Initiator Hello: protocol version, non-secret display name, public certificate, pairing intent. Bounded read timeout; no input accepted.
2. A previously trusted receiver responds Known with its public certificate. Initiator must already have that exact certificate pin.
3. Otherwise receiver asks local approval, generates a six-digit code, and sends Ready with public SPAKE2 message, public certificate, and name.
4. Initiator enters the code locally, sends its public SPAKE2 message and controller HMAC proof. Receiver verifies and sends a receiver-role HMAC proof. Transcript includes the domain separator and length-delimited controller SPAKE message, receiver SPAKE message, controller certificate, receiver certificate.
5. Upgrade the same TCP connection to mutual pinned-certificate TLS. Exchange/validate Welcome inventories. Only then save new trust and establish a session.

## Session messages

- Welcome: authenticated device ID/name, OS, and physical display inventory.
- ControlRequest / ControlState: coordinator-ordered main device and either/main-only policy, with increasing revision.
- Configuration: revision and validated source keyboard/pointer preferences; receiver safety preferences excluded.
- Topology: changed physical display inventory; invalidates the saved active arrangement.
- Layout: revision, selected controlled-display placement, and main-device physical topology.
- Handoff / HandoffAck: control token (revision and handoff), target native display position, and ownership confirmation.
- Input: control token, monotonic sequence, and motion, key, button, or scroll event. Scroll includes pixel deltas, CG scroll phase, and momentum phase; Windows converts pixel deltas to wheel units with fractional carry.
- Return / ReturnAck: control token, mapped main-device native position, and return barrier.
- Heartbeat / Pong: sender's echoed monotonic timestamp; use for same-clock round-trip measurement only.
- Release: control token; release ownership while retaining connection.
- Pause / Resume: sender's local availability; cannot override receiver-local pause.
- Identify: temporary local display-number overlay.
- Goodbye: orderly session close.

Reader/writer queues are bounded. No key/button transition is intentionally coalesced or dropped during active ownership. Queue exhaustion revokes control; it never buffers indefinitely. Revoked in-flight input is ignored. Only consecutive motion events with an identical control token may be combined. Coalescing cannot cross any non-motion event. Native callback generations prevent already queued capture events from being forwarded during a later handoff.

This protocol is currently specific to FlowLink and is not Deskflow-compatible.

Both peers must use protocol 2. Version 1 sessions are rejected with upgrade guidance. Existing local settings/trust can be read with defaults for the new control-policy and custom-shortcut fields.
