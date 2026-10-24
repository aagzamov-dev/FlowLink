# Security and privacy

**Not independently audited or approved for public release.** Native filtering is a usability setting, not a security lock. OS protections take precedence.

## Trust

Device identity is the SHA-256 fingerprint of its self-signed TLS certificate. Discovery names, IDs, addresses, and initial handshake metadata are unauthenticated hints. They cannot enable input or silently replace a stored pin.

First pairing requires receiving-device approval, a random six-digit code, SPAKE2 exchange, role-separated HMAC confirmations binding both SPAKE messages and certificates, then mutual TLS 1.3 proof of possession of the paired private keys. Public identity material is exchanged before TLS; private keys and the code are never sent across the wire. This is a new application-level composition and requires focused review, including transcript binding, role reflection, denial of service, and state transitions.

RustCrypto SPAKE2 0.4 states that it has not received an independent security audit and documents timing limitations. See https://docs.rs/spake2/0.4.0/spake2/. Do not describe the implementation or entire application as audited.

Pair codes expire in two minutes. Three approved or declined inbound attempts exhaust the global two-minute attempt window. Each retry requires a new approval/code; retries cannot use a retained PIN. Trust is written only after PAKE confirmation, mutual TLS, and valid peer inventory. Reconnection pins exact certificate DER and verifies TLS handshake signatures. Certificate hostname/CA validation is deliberately replaced by the paired exact certificate; certificate rotation requires explicit re-pairing.

Private keys use OS credential storage. If it is unavailable, startup fails rather than writing plaintext private keys. Public certificate pins and settings are local JSON; compromise of the same OS user/configuration is outside the network-attacker threat model.

## Boundaries

Only one connection/pairing owns a receiver at a time. Discovery is restricted to selected operational LAN interfaces; addresses exclude loopback and include private VPN interfaces. TCP listeners accept only private/link-local/shared-address-space peers arriving on allowed LAN addresses. The listener is wildcard-bound with application-level address filtering; it is not a firewall replacement. No broad firewall exceptions, UPnP, VPN bypass, or public relay.

Frames are length-bounded, protocol-versioned, and structurally validated. Active input sequences are monotonic per connection and bound to ownership revision/handoff tokens. Unexpected roles, bad geometry, invalid input, excessive message rates, and queue saturation terminate or safely release control. In-flight input after ownership revocation is discarded until another authenticated handoff; it is never replayed after reconnection.

Local pause and local-input suppression on both platforms remain receiver-controlled. Controller configuration contains only keyboard/pointer preferences. Incoming control cannot change local identity, emergency key, permissions, or local pause.

## Recovery and limits

Native atomic gates stop capture/filtering before cleanup. Emergency shortcuts are recognized before suppression. A watchdog is separate from network work. Remote-owned pressed states are released without intentionally releasing a physically held local key/button. Failure of native filtering stops remote control. Hard process termination removes OS-installed hooks/taps, but synthetic key-up recovery after a killed receiver cannot be guaranteed; this remains a physical-device release gate.

The engine does not log typed keys, clipboard contents, codes, private keys, or packet payloads. Clipboard commands copy only assigned local network addresses. No telemetry or cloud input transport. Foreground application identification stays on Mac and is used only for keyboard mapping.

## App updates

Optional background checks and user-requested downloads contact GitHub over HTTPS using the normal OS route/proxy. GitHub receives ordinary request metadata, including the public network address and updater user agent; typed input, pairing material and device identities are not included. Checks can be disabled in Settings. No GitHub token is embedded.

Only native commands can check/download/install an update. The webviews have no generic updater or process-plugin permissions. The official Tauri updater verifies package signatures against the bundled public key and requires the signed version to match the manifest. Downgrades are disabled. Release notes render as plain text. Installation requires a separate user action after a verified download, persists sharing as paused, disconnects and cleans up native input before exit. Concurrent checks cannot discard a ready package or overlap a download/install; settings and tray resume cannot unpause sharing during installation.

Private release keys stay outside tracked source. The manual preparation workflow has read-only repository permission and cannot publish. See [UPDATES.md](UPDATES.md) for key backup, platform certificates and the remaining physical update acceptance checks.

Remaining gates: physical crash/lock/permission tests, concurrent-input modifier tests, PAKE/TLS composition review, dependency/license review, packaged permissions, signing/notarization, and observed recovery-time measurements.
