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

Tests cover bidirectional two-engine ownership changes/handoffs, main-only policy, receiver release, pause races, stale tokens/capture generations, settings-change cleanup, endpoint fallback, fractional scrolling, custom shortcut editing, and partial/corner/negative-coordinate edge geometry, overlap rejection, key mapping stability across app changes, terminal Ctrl+C, held-state cleanup, legal ownership transitions, invalid input, frame length bounds, PAKE wrong-code/identity/role rejection, global pairing throttling, pinned mutual TLS exchange, changed server identity, mouse click sequencing, device sorting/search, IPv6 address formatting, and manual address validation.

A temporary Windows-only check harness under the ignored `.tools/` directory was used to type-check the real Windows platform module against windows-sys on `x86_64-pc-windows-gnu`. Subsequently, the full Windows release executable, Rust tests, Clippy, and frontend checks passed on GitHub's native Windows runner in [run 39236134349](https://github.com/aagzamov-dev/FlowLink/actions/runs/39236134349). This confirms compilation and automated tests, not physical input behavior.

The Check FlowLink workflow now builds an x64 NSIS setup executable on Windows after the checks pass, uploads it as `FlowLink-windows-x64-installer` for 30 days, and adds a download link to the Windows job summary. It runs on pushes, pull requests, and manual dispatch. Build success does not certify installation or keyboard/mouse sharing on a physical PC.

Read-only probe: `cargo run -p flowlink-engine --example inspect`. It does not start an input tap, inject events, or write device credentials. Shell sandbox/headless sessions may return no displays even when the physical computer has a screen; record that limitation rather than inventing inventory.

## Version 0.2 regression acceptance

Run every case in **both directions**, with each computer taking a turn as main:

1. Select main from either device, then enable main-only mode. The non-main computer cannot take over, but can always release or pause. Changing roles while typing/dragging releases remote input and requires a new handoff.
2. Pause both computers, resume each independently, and confirm one local resume cannot clear peer pause. Save settings while remote, disconnect during connection setup, and reject delayed handoff acknowledgements.
3. Add/edit/delete directional rules (including punctuation), verify Ctrl/Cmd and Shift combinations, repeat and release, AltGr, and terminal exceptions. Confirm local settings survive receiving another computer's configuration. Confirm multiline terminal entries remain editable.
4. Select a different controlled monitor and use both the button and edges. Add/remove a monitor while connected; both devices must discard old return edges until the arrangement is reviewed.
5. Try the reachable address after an unavailable address, reconnect after DHCP/VPN changes, cancel pairing/connecting, and confirm reconnect never captures input automatically. Check custom-port and hostname persistence.
6. Scroll slowly and flick in both axes, then move and click/drag immediately. Test a Mac trackpad and Windows precision touchpad. Native pinch/rotate/three-finger gestures are outside this release.
7. Repeat on the same LAN, with Kerio connected but local LAN allowed, and across the VPN using a permitted private IP. Confirm TCP 47831 is reachable separately from RDP. Test brief packet stalls: input must release safely while connectivity can recover; longer outages must reconnect without re-pairing.

The first session initiator has auto-connect enabled by default; the receiver does not initiate competing automatic connections. When upgrading an older pair, keep auto-connect enabled on the preferred initiating computer.

## Physical acceptance

Use Windows 11 with two monitors and MacBook Pro with its built-in display; add a Mac external display when checking target selection. Start with local input allowed and a visible emergency shortcut.

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

## Update configuration verification — 2026-10-24 (version 0.2)

The update configuration has been checked locally without publishing a release, uploading assets, installing an update, or restarting the user's app.

| Check | Result |
| --- | --- |
| Rust workspace tests | 55 passed: 49 engine tests and 6 updater tests |
| Update verification | Official Tauri verifier accepted the fixture and rejected modified bytes, a relabelled version and a downgrade |
| Update lifecycle | Duplicate/overlapping operations blocked; failed installation retains verified bytes; settings/tray resume blocked during installation; old settings migrate |
| Frontend tests | 16 passed, including download/install state, unknown-size progress, preview controls, failure messages and escaped release notes |
| Release preparation tests | 5 passed: matching versions, missing signing credentials, signature key/version metadata, all-platform manifest and invalid/incomplete package rejection |
| TypeScript, ESLint, Rust formatting, Clippy, frontend build | Passed |
| UI | Settings update card inspected in the browser preview; automatic-check toggle and disabled preview update button visible |
| macOS Apple Silicon package | Local app and updater archive built; bundle signature checked; archive signature verified with the configured public key and modified archive rejected |
| Workflow | YAML parsed/formatted; manual dispatch only, read-only repository permission, no publishing step; not dispatched |

The updater archive is about 4.4 MiB and the app about 9.9 MiB in this local build. The Mac bundle is ad-hoc signed, not Developer ID signed/notarized. Windows/Intel release builds, production certificate integration and a real installed-app upgrade have not been run for this change. The configured GitHub repository was confirmed public; no feed or release was published.

Before publication, install two successive production-signed versions on each supported platform. Check available/current/unavailable results; interrupted downloads over LAN and VPN; changed/corrupt signatures; install cancellation, permissions and disk-space failure; download while sharing; safe key/button release before restart; paused state after restart; old trust/settings retention; and both devices reconnecting after an upgrade. On Windows verify NSIS relaunch and per-user install location; on Mac verify replacement in Applications, notarization and retained/re-requested input permissions. Use an isolated test feed/build, never repoint existing users to test artifacts. See [UPDATES.md](UPDATES.md).

## Recorded local verification — 2026-10-24 (version 0.2 input/network work)

The 0.2 changes are checked locally on macOS. Native runtime behavior on a physical Windows peer and a real Kerio connection has not been measured. The Windows platform source is cross-checked against windows-sys; this is not a complete Windows installer build. Earlier CI results below apply to the earlier revision.

| Check | Result |
| --- | --- |
| Rust regression suite | 49 passed, including loopback endpoint fallback/TLS and simulated two-engine control in both directions |
| Frontend tests | 13 passed |
| Strict TypeScript, ESLint, frontend production build | Passed |
| Rust formatting and Clippy with warnings denied | Passed |
| Windows platform module | Cross-target type/lint check passed against windows-sys 0.61.2; not a complete Windows executable build |
| Interface preview | Adding/editing a directional punctuation shortcut and Mac → Windows rule verified; multiline terminal entries preserved; editor layout inspected |
| macOS arm64 app | Release bundle built with ad-hoc signing; not notarized |
| Physical Mac ↔ Windows / Kerio | Not run; no physical Windows peer available in this session |

Tests use a simulated native input backend; they do not install live hooks or inject system input. Native key translation and Mac callback bookkeeping also have isolated tests. The final bundle is `target/release/bundle/macos/FlowLink.app`; both computers need matching version 0.2 builds.

## Historical local verification — 2026-10-19 (version 0.1)

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
| Windows/macOS CI | Full release executable builds, tests, and lint passed in run 39236134349 |
| Public release review | Not completed |

These results do not certify keyboard/mouse injection, simultaneous input, OS-protected screens, network failover, or measured recovery/latency on physical machines.

After the user granted Device Control, the packaged app was restarted. System Settings showed FlowLink enabled under Device Control and Data Access, but absent from the separate Input Monitoring list. Input readiness remains pending that user-controlled grant and a subsequent restart. Retina backing-pixel enumeration was corrected using CGDisplayModeGetPixelWidth/Height; the corrected source passes the Rust checks and is included in the final bundle.

## Optimization and convenience-feature verification — 2026-10-25

- Rust workspace: 59 tests passed (53 engine, 6 updater). Added backup format/validation/local-setting isolation checks and reusable-frame tests covering varying lengths, rejected oversized payloads, and no partial writes. Existing handoff, cleanup, motion-ordering, and endpoint-fallback regressions pass.
- Frontend: 25 tests passed; release tooling: 5 tests passed. Added draft-merge checks, physical-key recording/reserved-chord checks, setup readiness, and expired pairing markup checks.
- Strict TypeScript, ESLint, frontend production build, release configuration check, Rust formatting, Clippy with warnings denied, and the macOS native release executable build passed.
- Browser preview: recorded a shortcut, added its rule, triggered the unsaved-navigation prompt, kept editing, retained multiline terminal edits after a failed preview save, discarded edits, and returned to Devices. Setup checklist layout was inspected.
- Downloads export commands compile in the native app. Backup parsing and validation are tested without applying preferences or accessing real credentials. Physical Mac/Windows Downloads behavior, native shortcut interception, and actual installed-app execution remain acceptance checks.
- No physical Mac ↔ Windows/VPN latency or idle-CPU measurements were taken. Cached discovery inventory, per-window snapshots, separate latency events, and frame reuse remove redundant work; numerical improvements remain unmeasured.

Additional manual checks for this pass:

1. Keep settings edits open while pausing/resuming from the tray. Untouched pause state must update while typed preferences remain intact. Save and reopen settings; confirm normalized multiline terminal entries and persisted shortcuts.
2. Open/close main and quick windows independently while latency changes. Hidden windows must receive no periodic engine events; newly shown windows must refresh and report current latency. Pairing should still bring the main window forward.
3. Change a display arrangement, cancel navigation, then save it. Reset after selecting another remote display should return to the saved display. Unplug a display or change the main device; stale drafts must not re-enable old switching edges.
4. Export preferences to Downloads, change input preferences, and import the backup. Review before saving. Reject future-version, oversized, malformed, invalid-key, and duplicate-rule files. Preserve device name, trust, pause, local-input policy, network interface, diagnostics, updates, and autostart.
5. Enable and save diagnostics, then export a report. Verify the file opens as JSON, contains no names/addresses/codes/typed input, and never replaces an existing file. Disable diagnostics and confirm native export is rejected.
6. Test pairing countdown in approval/code-entry/code-display stages. Expired codes must not be submitted or displayed as usable. A failed native submission must allow recovery; closing must cancel the outstanding request.

## 1.0.1 connection fixes

- 63 Rust tests and 30 frontend/release-tool tests passed locally on macOS.
- Added coverage for missing-runtime calls, invalid-address state preservation, simultaneous IPv4/IPv6 listeners, port-conflict recovery, and listener shutdown.
- TypeScript, ESLint, Clippy, formatting, and the macOS app/DMG build passed; bundle signature and DMG integrity verified.
- The packaged 1.0.1 app opened successfully. UI automation could not complete the Connect smoke test because the computer-use session became unavailable.
- Accessibility/Input Monitoring were not granted to this build. Real Mac/Windows pairing and input sharing remain unverified; Windows needs the shared fixes too.
