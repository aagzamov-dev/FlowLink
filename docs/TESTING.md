# Testing

Automated checks cannot certify a software KVM. Keep physical-device results separate from unit tests and interface preview checks.

## Automated

```sh
npm ci
npm run typecheck
npm run lint
npm test
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p flowlink
```

Tests cover partial/corner/negative-coordinate edge geometry, overlap rejection, key mapping stability across app changes, terminal Ctrl+C, held-state cleanup, legal ownership transitions, invalid input, frame length bounds, PAKE wrong-code/identity/role rejection, global pairing throttling, pinned mutual TLS exchange, changed server identity, mouse click sequencing, device sorting/search, IPv6 address formatting, and manual address validation.

A temporary Windows-only check harness under the ignored `.tools/` directory was used to type-check the real Windows platform module against windows-sys on `x86_64-pc-windows-gnu`. It is not a complete Windows engine build or executable test. CI/native Windows checks remain necessary.

Read-only probe: `cargo run -p flowlink-engine --example inspect`. It does not start an input tap, inject events, or write device credentials. Shell sandbox/headless sessions may return no displays even when the physical computer has a screen; record that limitation rather than inventing inventory.

## Physical acceptance

Use Windows 11 with two monitors and MacBook Pro with its built-in display. Start with local input allowed and a visible emergency shortcut.

1. Fresh install: permission-denied state, permission guidance, native restart readiness, names, addresses, copy exact IP vs IP:port, first pairing, code expiry and cancellation.
2. Pair over Wi-Fi, then Ethernet. Block multicast and verify manual address fallback. Test DHCP/IP changes, VPN on/off without policy bypass, interface preference, unreachable device, duplicate names, many advertisements, and occupied receiver.
3. Type an English and Russian paragraph in a browser/editor. Verify punctuation, Shift/Caps Lock, layout switching, repeat, left/right modifiers, Ctrl+C/V/X/A/Z/S and terminal Control+C/D/Z. Change preset/app between down/up.
4. Click, double/triple-click, drag-select, right/middle/extra buttons, vertical/horizontal wheel. Test speed and direction at common and high mouse polling rates.
5. Place Mac right/left/above/below each Windows display with offsets and mixed DPI/portrait layouts. Test corner motion, native Windows-to-Windows crossing, edge resistance, rapid returns, and saved layout recovery.
6. Hold each modifier/button during network failure, pause, return, sleep, app quit, or crash. Verify no stuck input and no accidental local shortcut. Test both local-input settings with simultaneous physical Mac input.
7. While remote, revoke Accessibility/Input Monitoring, lock either OS, enter secure input, suspend/resume each machine, unplug displays, kill the network worker, and force-kill either app. Verify physical local emergency escape; document any receiver key-up limitation after hard termination.
8. Keep the connection open for an hour: it must not disconnect because of a handshake-duration timer. Close all windows and verify tray controls and input still work.
9. Restart while paused, reconnect while local, deliberately disconnect, forget an active peer, and change its identity. No unexpected remote activation or automatic re-trust.

## Measurement

Record hardware, OS/build versions, interface/adapter, Wi-Fi conditions, monitor modes, and input polling rate. Capture p50/p95/p99 round-trip delay, jitter, reconnect time, watchdog recovery, idle CPU/memory, and event-to-observed-cursor response with a high-frame-rate camera where possible. Compare clocks only within the same process unless synchronization error is measured.

Release requires recorded physical results and a pairing/security review. Never label the application production-ready on the basis of these automated checks alone.

## Recorded local verification — 2026-10-19

| Check | Observed result |
| --- | --- |
| `npm run typecheck` | Passed, strict TypeScript |
| `npm run lint` | Passed |
| `npm test` | 9 tests passed |
| `npm run build` | Passed; production interface bundled |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 21 Rust tests passed |
| Windows native-module cross-check | Passed against windows-sys 0.61.2 for x86_64-pc-windows-gnu; not a complete Windows application build |
| `npm run desktop:build -- --bundles app` | macOS arm64 application built, approximately 8 MiB |
| `codesign --verify --deep --strict` | Passed after explicit ad-hoc bundle signing; not Developer ID signed or notarized |
| Native inventory probe | Real Wi-Fi IPv4/IPv6 addresses and interface classification verified; tunnel/AirDrop interfaces omitted |
| Packaged native launch | Passed: actual device name and live Wi-Fi addresses shown; missing permissions reported correctly |
| Mac display inventory | Packaged app reports the real 1512 × 982 logical built-in screen. Standalone shell probe has no active display inventory |
| Physical Windows → Mac input workflow | Blocked: no Windows machine available; typing, filtering, and failover remain unverified |
| Interface inspection | Devices, Displays, Settings, and manual-address dialog inspected in browser preview; no simulated device inventory |
| CI and public release review | Configured/documented; not executed or approved |

These results do not certify keyboard/mouse injection, simultaneous input, OS-protected screens, network failover, or measured recovery/latency on physical machines.

After the user granted Device Control, the packaged app was restarted. System Settings showed FlowLink enabled under Device Control and Data Access, but absent from the separate Input Monitoring list. Input readiness remains pending that user-controlled grant and a subsequent restart. Retina backing-pixel enumeration was corrected using CGDisplayModeGetPixelWidth/Height; the corrected source passes the Rust checks and is included in the final bundle.
