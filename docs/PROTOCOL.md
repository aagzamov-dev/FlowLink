# Protocol version 1

TCP port 47831. Frames: four-byte unsigned big-endian byte length followed by UTF-8 JSON. Accepted lengths: 1–65,536 bytes. No compression or clipboard/file payloads. Set TCP_NODELAY. Maintain one reader and one writer; a partial read is never cancelled and then resumed as a new frame. TLS 1.3 ALPN: `flowlink/1`.

## Establishment

1. Controller Hello: protocol version, non-secret display name, public certificate, pairing intent. Bounded read timeout; no input accepted.
2. A previously trusted receiver responds Known with its public certificate. Controller must already have that exact certificate pin.
3. Otherwise receiver asks local approval, generates a six-digit code, and sends Ready with public SPAKE2 message, public certificate, and name.
4. Controller enters the code locally, sends its public SPAKE2 message and controller HMAC proof. Receiver verifies and sends a receiver-role HMAC proof. Transcript includes the domain separator and length-delimited controller SPAKE message, receiver SPAKE message, controller certificate, receiver certificate.
5. Upgrade the same TCP connection to mutual pinned-certificate TLS. Exchange/validate Welcome inventories. Only then save new trust and establish a session.

## Session messages

- Welcome: authenticated device ID/name and physical display inventory.
- Configuration: validated keyboard/pointer preferences only; receiver safety preferences excluded.
- Layout: virtual Mac placement and Windows physical topology.
- Handoff / HandoffAck: target native display position and ownership confirmation.
- Input: monotonic sequence and motion, key, button, or scroll event.
- Return / ReturnAck: mapped Windows native position and return barrier.
- Heartbeat / Pong: sender's echoed monotonic timestamp; use for same-clock round-trip measurement only.
- Release: release remote ownership while retaining connection.
- Pause / Resume: sender's local availability; cannot override receiver-local pause.
- Identify: temporary local display-number overlay.
- Goodbye: orderly session close.

Reader/writer queues are bounded. No key/button transition is intentionally coalesced or dropped during active ownership. Queue exhaustion revokes control; it never buffers indefinitely. Revoked in-flight input is ignored. Motion coalescing is not implemented yet; future coalescing must not cross a non-motion event barrier.

This protocol is currently specific to FlowLink and is not Deskflow-compatible.
