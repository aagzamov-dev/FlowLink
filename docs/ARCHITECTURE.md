# Architecture

One Rust engine library and one Tauri application. Modules separate platform callbacks, input state/mapping, geometry, persistence, discovery, pairing, TLS, and session orchestration. The React UI sends narrow commands and listens for state snapshots; it never transports raw input events.

```text
React settings / quick panel
          │ typed commands + snapshots
Tauri lifecycle / native tray / identify overlays
          │
Rust engine ── discovery + pairing ── protected credential store
     │
Win32 / Quartz capture → bounded queues → TLS → Quartz / Win32 injection
     │                         ↔ heartbeat / handoff / release
independent local watchdog          │
                              receiver native input backend
```

## Input ownership

Disconnected/Paused → Local → Requesting → Remote → Returning → Local. Pause/disconnect are legal from any state. Emergency cleanup disables the native atomic gate before network/UI work. Transitions into Remote require a valid native lease and an authenticated handoff; direct disconnected-to-remote transitions are rejected.

Windows hooks run on a dedicated message-loop thread. They use a bounded nonblocking queue and atomic capture gate. Overflow stops capture/filtering and sets a separate fault flag; the input router drains stale events and restores ownership without permanently disabling a healthy hook. Mouse movement is translated into relative deltas; the Windows pointer is recentered inside its current monitor while remote. Native emergency detection precedes suppression. Hooks ignore their own tagged synthetic input. Windows SendInput implements the receiving path.

Both devices can listen/connect. The connection initiator orders main-device changes. A revision token and per-handoff token bind input/acks/releases to the current owner; old queued traffic cannot re-enable capture. Changing main device clears topology and held-input state, preserves each computer's own preferences, and requires a fresh handoff. Main-only policy restricts role changes, never local pause/release. Native injection, cleanup, and lease changes are serialized.

Mac uses documented session-level CoreGraphics event taps, tagged synthetic events, physical scan-code translation, and source-aware key/button bookkeeping. The receiver decides whether local input is permitted. Mapping decisions remain attached to key-down until key-up. Windows menu modifiers are deferred until a chord or click needs them, preventing translated Command/Option shortcuts from briefly activating Start or application menus; standalone modifier taps are retained. Double/triple click state and repeat flags are explicitly synthesized. The Mac resolves terminal bundle identifiers locally.

A dedicated watchdog checks leases, native availability, secure-input state, and discontinuities after suspension/clock changes. Heartbeats run every 250 ms; only fresh round-trip responses renew the one-second native lease. An expired lease cannot be revived by a late response. Connectivity has a separate five-second timeout. These are engineering values, not hard real-time guarantees. The macOS implementation fails closed during secure event input; Windows checks that the interactive desktop is Default.

## Geometry

Windows is DPI-aware. Its screen rectangles use native virtual-desktop pixels to preserve the actual arrangement; Mac rectangles use Quartz logical points. These are deliberately distinct native coordinate domains, not a single globally DPI-divided space. The editor uses the current main device's native desktop units for placement and target native dimensions; edge fractions map between those domains. No physical display settings change.

Only positive-length shared edges of at least 16 units switch. Two-unit corner insets prevent diagonal corner transitions. Edge resistance defaults to 100 ms; returning applies a cooldown. Held keys/buttons delay handoff. Display identities are saved per main device; invalid arrangements require explicit review. Display changes are exchanged with the peer and invalidate cached return edges. One chosen controlled display defines switching edges. Pointer targets are clamped to actual monitor rectangles, including gaps and negative coordinates.

Display and address inventory currently refresh every three seconds, with mDNS service-removal events for availability. Discovery reuses the maintenance worker’s cached addresses, including when registering services; connection attempts still take fresh routing inventory. Inventory changes use typed equality rather than JSON serialization. OS-notification-based inventory updates and more aggressive idle optimization remain follow-up work.

The tray/UI worker operates independently of input callbacks, compares typed snapshots, and emits only to visible main/quick windows. Each window has its own state cache, cleared while hidden so reopening refreshes it. Latency is removed from state comparisons and delivered separately about once a second to an isolated footer component. Tray tooltips update only when their label changes.

## Persistence and interface

Versioned JSON stores non-secret preferences, public trusted certificates, last working routes, and placements. Private identity keys reside in Keychain/Windows Credential Manager. Writes replace a temporary configuration file under a serialization lock. Unknown configuration versions fail rather than silently reset user settings.

Public commands cover snapshot, connect/disconnect, pause/return/switch, layout/settings, pairing approval/cancellation, trust/favorites/auto-connect, copying assigned addresses, identify, guided permissions, and startup preference. Custom commands expose no arbitrary paths, shell execution, or raw input injection to the UI.

Settings and display drafts preserve edits across background snapshots; unchanged fields accept fresh values, including pause state. Leaving a page with edits requires an explicit discard. Display topology/ownership changes reset the layout editor so invalid geometry cannot be retained. Saves report success only after the native command completes; duplicate commands are suppressed while pending.

Portable preference backups use a separately versioned, bounded JSON format containing input/switching preferences only. Import is validated by Rust and loaded into an editable draft. Exports use fixed generated filenames in Downloads with create-new semantics; frontend commands accept no output paths. Diagnostic exports use the existing opt-in redacted health snapshot.

## Connection and event throughput

Automatic connections let the OS route traffic, including private VPN routes. Hostname resolution is bounded and concurrent; address/interface attempts are staggered by 75 ms with a two-second connect limit per attempt. The winning connection cancels the other probes. Incoming sockets reserve the session only after a bounded, valid Hello, so abandoned probes do not block the real connection. Trust still binds the final peer identity. Manual hostnames/custom ports and successful source addresses are remembered; adapter changes reset reconnect backoff.

The writer merges only consecutive motion events with the same control token, never across key/button/scroll or control barriers. Each frame is written as one header-plus-payload buffer. Session readers and writers retain their frame buffers across messages; serialization writes directly behind the length prefix. Frame limits are checked before reading payloads and before writing any bytes. Pairing retains the same wire format and size limits. Scroll deltas retain fractional remainders at native integer APIs. Mac capture uses pixel scroll fields and preserves scroll phases (including mayBegin) and momentum deltas. Windows receives these as high-resolution wheel events; applications decide how to render them. This is not native multi-finger gesture forwarding.
