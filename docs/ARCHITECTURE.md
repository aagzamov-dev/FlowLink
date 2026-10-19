# Architecture

One Rust engine library and one Tauri application. Modules separate platform callbacks, input state/mapping, geometry, persistence, discovery, pairing, TLS, and session orchestration. The React UI sends narrow commands and listens for state snapshots; it never transports raw input events.

```text
React settings / quick panel
          │ typed commands + snapshots
Tauri lifecycle / native tray / identify overlays
          │
Rust engine ── discovery + pairing ── protected credential store
     │
Win32 hooks → bounded queues → framed TLS → Quartz injection
     │                         ↔ heartbeat / handoff / release
independent local watchdog          │
                              Mac session event tap
```

## Input ownership

Disconnected/Paused → Local → Requesting → Remote → Returning → Local. Pause/disconnect are legal from any state. Emergency cleanup disables the native atomic gate before network/UI work. Transitions into Remote require a valid native lease and an authenticated handoff; direct disconnected-to-remote transitions are rejected.

Windows hooks run on a dedicated message-loop thread. They use a bounded nonblocking queue and atomic capture gate. Overflow disables forwarding and marks hooks unhealthy. Mouse movement is translated into relative deltas; the Windows pointer is recentered inside its current monitor while remote. Native emergency detection precedes suppression. Hooks ignore injected Windows events.

Mac uses documented session-level CoreGraphics event taps, tagged synthetic events, physical scan-code translation, and source-aware key/button bookkeeping. The receiver decides whether local input is permitted. Mapping decisions remain attached to key-down until key-up. Double/triple click state and repeat flags are explicitly synthesized. The Mac resolves terminal bundle identifiers locally.

A dedicated watchdog checks leases, native availability, secure-input state, and discontinuities after suspension/clock changes. Heartbeats run every 250 ms; the nominal lease is one second. These are engineering values, not hard real-time guarantees. The macOS implementation fails closed during secure event input; Windows checks that the interactive desktop is Default.

## Geometry

Windows is DPI-aware. Its screen rectangles use native virtual-desktop pixels to preserve the actual arrangement; Mac rectangles use Quartz logical points. These are deliberately distinct native coordinate domains, not a single globally DPI-divided space. The editor uses Windows desktop units for placement and target logical dimensions; edge fractions map between those domains. No physical display settings change.

Only positive-length shared edges of at least 16 units switch. Two-unit corner insets prevent diagonal corner transitions. Edge resistance defaults to 100 ms; returning applies a cooldown. Held keys/buttons delay handoff. Display identities are saved; invalid arrangements require explicit review.

Display and address inventory currently refresh every three seconds, with mDNS service-removal events for availability. OS-notification-based inventory updates and more aggressive idle optimization remain follow-up work. The tray/UI snapshot worker operates independently of input callbacks and avoids emitting unchanged state to hidden windows.

## Persistence and interface

Versioned JSON stores non-secret preferences, public trusted certificates, last working routes, and placements. Private identity keys reside in Keychain/Windows Credential Manager. Writes replace a temporary configuration file under a serialization lock. Unknown configuration versions fail rather than silently reset user settings.

Public commands cover snapshot, connect/disconnect, pause/return/switch, layout/settings, pairing approval/cancellation, trust/favorites/auto-connect, copying assigned addresses, identify, guided permissions, and startup preference. Custom commands expose no arbitrary paths, shell execution, or raw input injection to the UI.
