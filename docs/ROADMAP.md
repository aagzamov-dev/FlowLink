# Implementation status

## Implemented in development source

- Rust engine, Tauri tray/quick panel, React/TypeScript devices/displays/settings interface.
- Setup checklist and connection help; guarded unsaved drafts, save feedback, pairing expiry countdown and failed-submission recovery.
- Physical-key shortcut recording and validated portable preference backups; opt-in diagnostic health exports.
- Cached discovery address inventory, typed snapshot/inventory comparisons, separate throttled latency events, and reusable session frame buffers.
- Private-LAN discovery, manual IP/hostname connection, live assigned addresses and copy actions.
- Device names, favorites, trust revocation, device search, one active receiver, automatic trusted reconnect and pause persistence.
- Six-digit SPAKE2 pairing, OS-protected identity, mutually authenticated pinned TLS 1.3, frame/rate bounds.
- Win32 and Quartz input capture/injection and display inventory, separate watchdog, emergency escape.
- Bidirectional physical-key translation, directional custom shortcut editing, receiver-local terminal detection, source-aware cleanup, repeat/click-state synthesis.
- Selected controlled-display layout dragging, snapping, quick placement, Identify overlays, saved edges.
- Startup preference and single-instance guard. Local macOS packaging configuration and Windows/macOS CI build checks.
- Parallel address resolution and staggered connect attempts, private VPN adapters, retained endpoints, network-change reconnect, and distinct input/connection timeouts.
- Stale-event barriers, synchronized native cleanup, settings-change release, receiver pause recovery, and topology invalidation.
- Consecutive motion coalescing, fractional cursor/scroll accumulation, Mac pixel scrolling and momentum deltas, and Windows high-resolution wheel injection.

## Validation still required

- Installer installation and native execution on a physical Windows PC. A previous revision passed GitHub's Windows runner; version 0.2 requires a fresh full native build. CI builds and uploads an NSIS installer.
- Actual PC with two monitors ↔ MacBook workflow over Wi-Fi and Ethernet.
- English/Russian typing, layout switching, terminal behavior, pointer feel, mixed DPI, and high-polling-rate mice.
- Active local-input filtering on both platforms and simultaneous local/remote key ownership under real OS event delivery.
- Lock/sleep, forced process termination, permission revocation, hotkey release, stale/in-flight events, and one-second recovery target.
- Public release security review, package signatures/notarization, dependency-license validation, and performance measurements.

## Deliberate first-build limits

- One active peer. Either Mac or Windows is main, with an either/main-only role policy. One selected controlled display defines switching edges.
- Input capture is deliberately fail-closed if native setup fails; granting permissions requires restarting the app.
- Fixed switch chord; emergency escape has two supported choices. Common shortcuts and arbitrary supported key/chord mappings are editable in both directions; no macro engine. A local shortcut recorder is available for supported physical keys; OS-reserved combinations can still be intercepted.
- The receiving computer's input source is authoritative. No automatic Windows/Mac language-state synchronization; input-source shortcut must be enabled in macOS.
- Only configured terminal applications get automatic Control exceptions; integrated terminal panes are not automatically distinguished from their parent editor.
- Screen/address inventory uses a three-second refresh. Native display/adapter notifications and deeper idle optimization remain future work.
- The Windows local pointer is recentered during remote input; visual pointer hiding and high-rate relative motion need hardware validation.
- IPv4 and IPv6 ULA connection paths are implemented. Scoped IPv6/link-local and adapter failover remain unverified.
- Opt-in local diagnostic summaries and Downloads exports are available. No persisted diagnostic event log yet.
- No public updater, signed release pipeline, cloud, Linux, clipboard content, files, login/FileVault, UAC/secure desktop, or native multi-finger gesture forwarding.

## Later milestones

After the physical acceptance gates pass: improve relative pointer capture/idle behavior, wider keyboard tests, diagnostics, signing, and release. Then consider additional receivers/displays, opt-in clipboard sharing, and direct wired-IP research. Ordinary Ethernet LAN is already in scope; USB-specific networking/drivers are not.

## Prioritized follow-up work

See [Future features and optimizations](FUTURE_FEATURES.md) for the remaining backlog, dependencies, effort estimates, and acceptance criteria. Physical input and release acceptance remain gates before public distribution.
