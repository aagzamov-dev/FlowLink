# FlowLink

**One keyboard. One mouse. Your Windows PC and Mac.**

FlowLink is an open-source desktop app for sharing a Windows 11 or Mac computer's keyboard, mouse, and ordinary touchpad input with the other computer over a reachable local network or private VPN. Connect from the tray, arrange your screens, and move across a configured edge to switch computers.

Rust handles input, networking, authentication, and recovery. Tauri 2, React, and strict TypeScript provide the desktop interface. No account or cloud server is required.

> **Development preview, version 0.2:** bidirectional control and recovery changes are implemented and covered by automated tests. Install matching 0.2 builds on both computers. This protocol is incompatible with 0.1; physical Windows ↔ Mac and Kerio VPN testing remains required.

## Features

- **Connect from either computer:** discover nearby FlowLink devices or enter a private IP/hostname, optionally with a port. No account, cloud server, or RDP session is needed.
- **Choose the main device:** use Windows to control Mac or Mac to control Windows. Allow either computer to change the main device, or reserve changes for the current main device. Both computers can always release or pause input.
- **Remembered connections:** paired identities, saved hostname/custom port, multiple address candidates, operating-system routing by default, optional interface selection, and automatic reconnect after network changes.
- **Display arrangement:** preserve the main computer's native display group and position one selected controlled display beside it. Switch at shared edges or with the control button/hotkey.
- **Editable shortcuts:** directional key-to-key or chord-to-chord rules, Ctrl ↔ Command editing presets, physical-key mode, and Mac terminal exceptions.
- **Pointer and touchpad:** fractional pointer tracking, horizontal/vertical scrolling, small scroll-delta accumulation, Mac scroll phases/momentum deltas, pointer speed, and reverse scroll. OS-wide pinch/three-finger gestures are not forwarded.
- **Recovery:** ownership tokens reject stale input; an independent watchdog releases held input after a stalled input lease. Short network stalls release input while retaining the connection for recovery.
- **Local control:** local keyboard/trackpad permission on the controlled computer, persistent pause, emergency escape, tray controls, trust removal, and opt-in redacted diagnostics.

## Current status

The automated regression suite exercises two engine instances changing main device and handing off input in both directions, pause/recovery, stale input, shortcut mapping, fractional scrolling, and connection fallback. See [Testing](docs/TESTING.md) for current results and physical acceptance checks. Automated tests do not establish pointer feel, VPN reliability, or latency on actual computers.

## Download a Windows test installer

For **Windows 11 on Intel/AMD x64**, you can download an installer without installing development tools:

1. Sign in to GitHub and open [Check FlowLink in Actions](https://github.com/aagzamov-dev/FlowLink/actions/workflows/check.yml).
2. Select the latest successful run on **main**.
3. Under **Artifacts**, download **FlowLink-windows-x64-installer**. The Windows job summary also contains a download link.
4. Extract the ZIP and run the `FlowLink_..._x64-setup.exe` inside.

Installers are built on pushes and pull requests. To request another build, open the workflow and choose **Run workflow** on `main`. Downloads are retained for **30 days**; run the workflow again if an artifact expires.

These are **unsigned development installers**. Windows may show an unknown-publisher or SmartScreen notice. They are intended for testing; keyboard/mouse behavior still needs verification on your own computers. The installer may require internet access to install WebView2 if it is missing.

## Build and run

Build separately on each computer and run the native desktop app on both. The browser preview does not provide input sharing.

### Requirements

- **Node.js 22.12+** and npm.
- **Rust installed through rustup.** The repository pins its toolchain in [rust-toolchain.toml](rust-toolchain.toml).
- [Tauri's native prerequisites](https://v2.tauri.app/start/prerequisites/):
  - **Windows 11:** Microsoft C++ Build Tools with **Desktop development with C++** and a Windows SDK, Rust's MSVC toolchain, and Microsoft Edge WebView2 Runtime.
  - **macOS 13+:** Xcode Command Line Tools. The current local bundle has been tested on Apple Silicon; Intel Mac builds have not been verified.

Restart your terminal after installing build tools.

### Get the source

```sh
git clone https://github.com/aagzamov-dev/FlowLink.git
cd FlowLink
npm ci
```

### Run the desktop app

```sh
npm run desktop
```

Closing a window keeps FlowLink running in the tray/menu bar. Use **Quit** to stop the app.

### Create a Windows installer

Run on Windows from the project folder:

```powershell
npm run desktop:build -- --bundles nsis
```

The installer is written to `target/release/bundle/nsis/`. The standalone executable is written to `target/release/flowlink.exe` with the default native target.

GitHub Actions uses the same installer build with an explicit x64 target. See [Tauri's Windows installer guide](https://v2.tauri.app/distribute/windows-installer/) for packaging details.

### Create a macOS app

Run on Mac from the project folder:

```sh
npm run desktop:build -- --bundles app
```

The app is written to `target/release/bundle/macos/FlowLink.app`. Normal local builds use ad-hoc signing. Developer ID signing and notarization are configured in the manual installer-preparation workflow and require your Apple credentials.

### App updates and release configuration

Settings includes **Check for updates**, optional background checks, download progress, release notes, and **Install & restart**. Updates are verified before installation; sharing pauses and input is released before restarting. Install matching versions on both computers, then resume sharing.

The manual **Prepare installers (no publishing)** workflow prepares Apple Silicon Mac, Intel Mac, and Windows x64 installers plus signed updater packages. It cannot create or publish releases. Signing credentials and physical update tests are still required before distribution. See [update setup and signing configuration](docs/UPDATES.md), including the local private signing key that must be backed up.

### Preview the interface

```sh
npm run dev
```

The frontend runs at `http://127.0.0.1:1420`. It does not emulate devices, permissions, connections, or input. Native features require `npm run desktop`.

## Connect your computers

1. Open matching FlowLink 0.2 desktop builds on Windows and Mac.
2. On Mac, grant **Accessibility / Device Control** and **Input Monitoring**, then restart FlowLink.
3. On either computer, select the other under **Nearby**, or choose **Connect by IP** and enter the address displayed on the other computer. The default port is **TCP 47831**.
4. Approve the request on the receiving computer and enter its six-digit code on the initiating computer. Pairing is remembered.
5. In **Devices → Main device**, choose which computer supplies the keyboard/pointer. Leave **Either device can change the main device** enabled for switching from either side, or select main-only control.
6. On the main device, open **Displays**, select the controlled screen, place it beside the main display group, and save. You can also use **Control other computer** without an edge arrangement.
7. Move across a saved edge or press `Ctrl+Alt+Shift+Right`. Release the shortcut keys before handoff. On a receiving computer, the shortcut releases active control; when local, it makes that computer main if the policy permits. Starting control is a separate handoff.

For a same-network or Kerio VPN setup, start with **Automatic** network interface and use the peer's reachable private IP. Discovery may not cross the VPN, so manual IP is the reliable setup path. RDP connectivity is useful evidence of routing, but FlowLink needs its own TCP port 47831 permitted by the existing firewall/VPN policy. FlowLink does not alter those policies.

Trusted reconnect tries saved and discovered addresses and remembers a working route. It restores connectivity without automatically capturing input. A deliberate **Disconnect** suspends automatic reconnect until an explicit connection or app restart. Pause survives reconnect/restart.

## Return control and pause

- **Emergency pause:** `Ctrl+Alt+Shift+Escape`, handled locally. Settings can replace Escape with F12.
- **Release:** use **Release control**, **Return here**, the switch shortcut, or a configured return edge. Releasing active input works even in main-only mode.
- **Network interruption:** the native input lease targets local recovery within about one second. The connection has a separate five-second timeout, allowing a brief stall to recover without re-pairing. Resume control explicitly when the connection is responsive again.
- **Local input:** enabled by default on both computers. Disabling it filters ordinary local input only while that computer receives remote control. Emergency escape remains available. This is a usability setting, not a security lock.

## Keyboard and languages

The receiving computer's selected keyboard layout/input source determines typing. English/Russian and IME workflows still need physical verification.

**Adapt editing shortcuts** maps common Windows Ctrl editing combinations to Mac Command combinations and Mac Command editing combinations to Windows Ctrl. Mac terminal exceptions preserve Control. Ctrl+Alt/AltGr combinations are not matched as ordinary Ctrl editing shortcuts.

Under **Settings → Custom shortcuts**, choose the direction and enter a source/target such as `Cmd+Shift+C` → `Ctrl+Shift+C` or `Ctrl+/` → `Cmd+/`. Rules match all held modifiers, override the preset, and stay attached through key-up even if settings change. Add or update the rule, then click **Save changes**. Configure preferences on the computer that will supply input; each device keeps its own settings. Switch/emergency shortcuts are reserved.

For Windows → Mac, `Win+Space` can map to `Control+Space`; enable that input-source shortcut in macOS. FlowLink does not synchronize language state between computers.

## Security and privacy

- First-time pairing uses **SPAKE2** with a short-lived code and device-wide attempt limits.
- Trusted sessions use **mutually authenticated TLS 1.3** with pinned device identities.
- Private identity material uses the operating system's credential storage; secrets are excluded from frontend state and settings exports.
- No account, hosted relay, cloud server, analytics, or clipboard-content sharing is included.
- Device names are labels; authentication relies on the paired identity.

The Rust SPAKE2 implementation [documents that it has not been independently audited](https://docs.rs/spake2/latest/spake2/). FlowLink's pairing and input-ownership design also needs a focused security review before public release. Read the [security notes](docs/SECURITY.md) and [protocol documentation](docs/PROTOCOL.md).

## Limits and planned work

Version 0.2 supports one active peer, either input direction, all native displays on the main computer, and one selected controlled display for edge switching.

Not included: Linux, multiple simultaneous peers, clipboard/file transfer, native pinch/rotate/three-finger gestures, USB-specific networking, automatic updates, macros, or shortcut recording. Windows touchpad support uses the pointer/wheel events exposed by Windows rather than raw finger contacts. High-rate Windows pointer capture still uses the existing recentering hook path.

Login/FileVault, UAC/secure desktops, protected input, all keyboard layouts, media keys, and IME composition are not supported or validated. Full Windows builds and physical pairing, scrolling, sleep/recovery, mixed DPI, and VPN performance must be checked with this revision before release.

See the [Roadmap](docs/ROADMAP.md) for remaining validation work.

## Development

```sh
npm run typecheck
npm run lint
npm test
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Read-only native inventory probe:

```sh
cargo run -p flowlink-engine --example inspect
```

It reports display/network inventory and permission readiness without installing input hooks, pairing, or injecting events. Shell/headless sessions may not expose the active desktop's displays.

The desktop launcher uses an ignored project-local Rust installation in `.tools/` when present; otherwise it uses system Rust. Do not copy `.tools/`, `node_modules/`, or `target/` between operating systems.

| Location | Purpose |
| --- | --- |
| `crates/flowlink-engine/` | Rust input, networking, pairing, topology, settings, and platform integrations |
| `src-tauri/` | Native app, tray controls, commands, and packaging |
| `src/` | React and TypeScript interface |
| `docs/` | Architecture, protocol, security notes, testing, and roadmap |

[Architecture](docs/ARCHITECTURE.md) · [Protocol](docs/PROTOCOL.md) · [Security](docs/SECURITY.md) · [Testing](docs/TESTING.md) · [Roadmap](docs/ROADMAP.md)

## Contributing

Issues, fixes, and test reports are welcome. Include your OS version, hardware, network connection, and clear reproduction steps. Remove pairing codes, private keys, and other sensitive information from reports. Keep physical-device observations separate from automated test results.

## License

FlowLink is available under the [MIT License](LICENSE). You can use, modify, and redistribute the code under its terms.
