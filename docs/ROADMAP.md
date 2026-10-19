# Implementation status

## Implemented in development source

- Rust engine, Tauri tray/quick panel, React/TypeScript devices/displays/settings interface.
- Private-LAN discovery, manual IP/hostname connection, live assigned addresses and copy actions.
- Device names, favorites, trust revocation, device search, one active receiver, automatic trusted reconnect and pause persistence.
- Six-digit SPAKE2 pairing, OS-protected identity, mutually authenticated pinned TLS 1.3, frame/rate bounds.
- Win32 input hooks/display inventory and Quartz injection/session event tap, separate watchdog, emergency escape.
- Physical-key translation, preset editing, receiver-local terminal detection, source-aware cleanup, repeat/click-state synthesis.
- Single-Mac-display layout dragging, snapping, quick placement, Identify overlays, saved edges.
- Startup preference and single-instance guard. Local macOS packaging configuration and Windows/macOS CI build checks.

## Validation still required

- Installer installation and native execution on a physical Windows PC. Full release executable compilation, tests, and lint already passed on GitHub's Windows runner; CI now also builds and uploads an NSIS installer.
- Actual PC with two monitors ↔ MacBook workflow over Wi-Fi and Ethernet.
- English/Russian typing, layout switching, terminal behavior, pointer feel, mixed DPI, and high-polling-rate mice.
- Active local-Mac filtering and simultaneous local/remote key ownership under real OS event delivery.
- Lock/sleep, forced process termination, permission revocation, hotkey release, stale/in-flight events, and one-second recovery target.
- Public release security review, package signatures/notarization, dependency-license validation, and performance measurements.

## Deliberate first-build limits

- One Mac display and one active Mac receiver. Windows is the only input host.
- Input capture is deliberately fail-closed if native setup fails; granting permissions requires restarting the app.
- Fixed switch chord; emergency escape has two supported choices. Common shortcut rules can be enabled/disabled; no arbitrary macro engine or shortcut recorder.
- Mac input source is authoritative. No automatic Windows/Mac language-state synchronization; input-source shortcut must be enabled in macOS.
- Only configured terminal applications get automatic Control exceptions; integrated terminal panes are not automatically distinguished from their parent editor.
- Screen/address inventory uses a three-second refresh. Native display/adapter notifications and deeper idle optimization remain future work.
- The Windows local pointer is recentered during remote input; visual pointer hiding and high-rate relative motion need hardware validation.
- IPv4 and IPv6 ULA connection paths are implemented. Scoped IPv6/link-local and adapter failover remain unverified.
- Opt-in local diagnostic summaries are available. No persisted diagnostic log or diagnostic export yet.
- No public updater, signed release pipeline, cloud, Linux, clipboard content, files, gestures, login/FileVault, UAC/secure desktop, or reverse input sharing.

## Later milestones

After the physical acceptance gates pass: improve relative pointer capture/idle behavior, wider keyboard tests, diagnostics, signing, and release. Then consider Mac → Windows, additional receivers/displays, opt-in clipboard sharing, and direct wired-IP research. Ordinary Ethernet LAN is already in scope; USB-specific networking/drivers are not.
