# FlowLink

**One keyboard. One mouse. Your Windows PC and Mac.**

FlowLink is an open-source desktop app for sharing a Windows 11 computer's keyboard and mouse with a Mac over your local Wi-Fi or Ethernet network. Connect from the tray, arrange your screens, and move across a configured edge to switch computers.

Rust handles input, networking, authentication, and recovery. Tauri 2, React, and strict TypeScript provide the desktop interface. No account or cloud server is required.

> **Development preview:** the macOS app builds and launches, and Windows/macOS automated build checks pass. Windows test installers are built by GitHub Actions. Physical Windows → Mac input sharing still needs validation; there is no stable release yet.

## Features

These features are implemented in the development source. Native input and two-computer behavior still need the hardware testing described below.

- **Quick tray controls:** connect, disconnect, pause, resume, and return control to Windows.
- **LAN connections:** automatic FlowLink device discovery plus manual IP/hostname connection over Wi-Fi or Ethernet.
- **Easy addressing:** live local addresses grouped by interface, with separate actions to copy an IP or an address with its port.
- **Remembered devices:** trusted and nearby lists, search, favorites, availability, and one active Mac receiver at a time.
- **Six-digit pairing:** approve on the Mac, enter its temporary code on Windows, then reconnect using trusted device identities.
- **Display arrangement:** keep Windows monitors in their native arrangement and drag the Mac display beside them, with snapping, offsets, and partial shared edges.
- **Keyboard preferences:** physical-key forwarding, Windows-familiar and Mac-native presets, common shortcut mappings, and terminal exceptions.
- **Pointer preferences:** pointer speed, scroll direction, edge resistance, and hotkey-only switching.
- **Local Mac input:** allow the Mac's keyboard/trackpad alongside remote input, or filter ordinary local input during remote control.
- **Recovery controls:** local emergency shortcut, session watchdog, held-input cleanup, and persistent pause.
- **Desktop settings:** system light/dark appearance, optional launch at login, device naming, trust removal, and opt-in redacted diagnostics.

## Current status

| Area | Verified so far |
| --- | --- |
| macOS application | ARM64 development bundle builds and launches; real device name, addresses, and built-in display inventory checked |
| Interface | Devices, Displays, Settings, and manual connection dialog inspected |
| Automated checks | 21 Rust tests and 9 frontend tests passed; TypeScript, lint, formatting, Clippy, and frontend production build passed |
| Windows application | Full release executable, tests, and lint passed on GitHub's Windows runner; physical execution and input testing pending |
| Windows → Mac sharing | Physical pairing, typing, pointer behavior, filtering, reconnect, and recovery testing pending |
| Public release | Security review, packaged-device testing, signing/notarization, and performance measurements pending |

See [Testing](docs/TESTING.md) for recorded results and acceptance checks. Automated tests do not establish input reliability or latency on actual computers.

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

The app is written to `target/release/bundle/macos/FlowLink.app`. The current configuration uses ad-hoc signing for local development; Developer ID signing and notarization are not configured.

### Preview the interface

```sh
npm run dev
```

The frontend runs at `http://127.0.0.1:1420`. It does not emulate devices, permissions, connections, or input. Native features require `npm run desktop`.

## Connect your computers

1. Open the native FlowLink app on Windows and Mac, on the same reachable LAN.
2. On Mac, grant **Accessibility / Device Control** and **Input Monitoring** when prompted, then restart FlowLink. Permission names vary by macOS version.
3. On Windows, select the Mac under **Nearby**, or use **Connect by IP** with an address shown on the Mac. The default listener port is **TCP 47831**.
4. Approve pairing on the Mac. Enter its random six-digit code on Windows. Codes expire after two minutes; failed attempts are limited and require renewed approval.
5. Open **Displays** on Windows, position the Mac beside the Windows screen group, and save. FlowLink does not change the operating system's display arrangement.
6. Move across the configured shared edge. Alternatively, use **Control Mac** or `Ctrl+Alt+Shift+Right`; hotkey handoff waits for modifiers to be released.

Trusted devices can reconnect automatically. Reconnection restores connectivity without immediately transferring input. Explicit pause survives reconnection and restart. Deliberate **Disconnect** suspends automatic reconnect until another explicit connection or app restart.

Discovery lists FlowLink installations, rather than every device on your network. If discovery is blocked, use a manual address; routing and firewall rules must still permit the connection. FlowLink does not change firewall or VPN policies.

## Return control and pause

- **Emergency pause:** `Ctrl+Alt+Shift+Escape`, handled locally. Settings can replace Escape with F12.
- **Tray controls:** use **Return to Windows** or **Pause**.
- **Connection loss:** the watchdog targets local recovery after one second without valid peer progress. Actual timing still needs hardware measurement.

On Mac, **Allow local Mac keyboard and trackpad** is enabled by default. Disabling it filters ordinary local input only during active remote control, while keeping emergency escape available. If the filter or required permissions fail, remote control ends. This setting still needs physical-device verification and is not a security lock.

## Keyboard and languages

The **Mac's selected input source** determines typing. Add US English and Russian input sources on Mac for the planned bilingual workflow; full typing validation is still pending.

**Windows-familiar** mode maps common Ctrl editing shortcuts to Command equivalents while preserving Control combinations in configured terminal applications. Terminal and iTerm2 are included by default. Integrated terminal panes may need an application exception or Mac-native mode.

By default, Windows `Win+Space` requests Mac `Control+Space` input-source switching; the Windows trigger can be changed to `Ctrl+Space`. Enable the corresponding input-source shortcut in macOS. FlowLink does not automatically synchronize the two operating systems' language state.

## Security and privacy

- First-time pairing uses **SPAKE2** with a short-lived code and device-wide attempt limits.
- Trusted sessions use **mutually authenticated TLS 1.3** with pinned device identities.
- Private identity material uses the operating system's credential storage; secrets are excluded from frontend state and settings exports.
- No account, hosted relay, cloud server, analytics, or clipboard-content sharing is included.
- Device names are labels; authentication relies on the paired identity.

The Rust SPAKE2 implementation [documents that it has not been independently audited](https://docs.rs/spake2/latest/spake2/). FlowLink's pairing and input-ownership design also needs a focused security review before public release. Read the [security notes](docs/SECURITY.md) and [protocol documentation](docs/PROTOCOL.md).

## Limits and planned work

The first version supports **Windows → Mac**, **one active Mac**, and **one Mac display**. Returning the Windows-controlled pointer from Mac to Windows is included.

Not included yet: Mac → Windows control, Linux, multiple simultaneous receivers, clipboard sharing, file transfer, trackpad gestures, USB-specific networking, automatic updates, and arbitrary shortcut recording.

Login/FileVault screens, Windows secure desktops/UAC prompts, protected input, all keyboard layouts, media keys, and IME composition are not supported or validated. Mixed DPI, high-rate mice, simultaneous local/remote input, sleep, permission changes, and adapter failover require further testing.

See the [Roadmap](docs/ROADMAP.md) for full implementation limits and release gates.

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
