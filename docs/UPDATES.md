# Updates and installer preparation

This configuration does not publish a release. The **Prepare installers (no publishing)** workflow is manual only, has `contents: read`, and contains no tag, push, release creation or asset publication step. Ordinary CI continues producing development installers.

## In the app

Open **Settings → App updates**, or choose **Check for updates…** from the tray/menu bar menu. Packaged builds check after a 30-second startup delay and every six hours while running. Turn off **Automatically check for updates** and save settings to stop background checks. Debug builds only check when requested.

A check displays the available version and release notes. **Download update** retrieves and verifies the package; **Install & restart** is a separate action. Downloads do not stop sharing. Installation pauses sharing, releases held input and disconnects before replacing the app. Sharing stays paused after restart; update both computers and resume explicitly. Failed checks and downloads can be retried. Verified downloads remain available in memory while the app runs, including across window closes and page changes; quitting discards them.

Checks have a 20-second timeout and downloads a five-minute limit. The updater follows system routing and macOS/Windows system proxy configuration, without changing the VPN or firewall. GitHub must be reachable for updates; device sharing remains LAN/VPN only and does not depend on the update server. A missing/unpublished feed is shown as unavailable, not as “up to date”.

## Update signing

The public verification key is configured in `src-tauri/tauri.conf.json`. A matching private key was generated locally at `.tools/release/updater.key`, with owner-only file permissions in an ignored, owner-only folder. It is unencrypted at rest and is **not** committed or uploaded. Back it up securely in a password manager or another protected location before deleting `.tools` or moving this project. Losing this key breaks the update path for builds using the current public key. Do not generate a replacement for each build.

For a future workflow run, add the private key file's complete contents as the repository Actions secret `TAURI_SIGNING_PRIVATE_KEY`. For this locally generated key, leave `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` empty. If you later encrypt the same key, store its password in that secret. Never store the private key or password in source, workflow YAML, release artifacts, or a public issue.

Every updater artifact is signed and bound to its version by the pinned Tauri CLI. The app enforces `requireSignedVersion`, rejects modified files and relabelled versions, uses HTTPS, and does not permit downgrades. Updater signature verification is independent of Apple/Windows publisher signing.

The configured stable endpoint is:

`https://github.com/aagzamov-dev/FlowLink/releases/latest/download/latest.json`

It must be publicly readable without authentication. Do not embed a GitHub token in the app. If this repository becomes private, use a public HTTPS release host and update the configured endpoint and manifest asset URLs before distributing a build.

## Prepared platforms

| Computer | Installer | Updater package | Manifest platform |
| --- | --- | --- | --- |
| Apple Silicon Mac, macOS 13+ | DMG | `.app.tar.gz` + `.sig` | `darwin-aarch64` |
| Intel Mac, macOS 13+ | DMG | `.app.tar.gz` + `.sig` | `darwin-x86_64` |
| Windows x64 | Per-user NSIS setup | `.exe` + `.sig` | `windows-x86_64` |

Windows uses a per-user installation and a visible progress bar during updates. WebView2 may need an internet connection on first installation. Windows ARM64 native builds are not included. Intel Mac and Windows packages must be validated on their respective machines.

## Platform signing credentials

Choose **development** to prepare updater-signed test packages with ad-hoc Mac signing and no Windows publisher signature. These files can show OS warnings and must not be published as production updates.

Choose **production** only after configuring the following repository Actions secrets. Missing credentials fail preparation. This mode still only prepares files; it cannot publish.

| Platform | Secrets |
| --- | --- |
| Both | `TAURI_SIGNING_PRIVATE_KEY`, optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |
| Mac | `APPLE_CERTIFICATE` (base64 Developer ID Application `.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` (full Developer ID Application identity), `APPLE_ID`, `APPLE_PASSWORD` (app-specific notarization password), `APPLE_TEAM_ID` |
| Windows | `WINDOWS_CERTIFICATE` (base64 code-signing PFX), `WINDOWS_CERTIFICATE_PASSWORD` |

The Mac build enables hardened runtime, signs, notarizes through Tauri, and verifies the bundle signature and stapled ticket. The Windows workflow imports the PFX into the temporary runner's current-user certificate store, signs with SHA-256 and an RFC 3161 timestamp, and verifies the installer signature. Modern hardware-backed or cloud Windows certificates generally cannot be exported as PFX; if your certificate uses one of those services, adapt the Windows signing step and Tauri `signCommand` to that provider before selecting production. Publisher signing does not guarantee that Windows SmartScreen immediately trusts a new application.

No Apple account, certificate, paid signing service or repository secret has been created/configured by this code change.

## Preparing files later

1. Keep the same new version in `Cargo.toml`, `package.json`, `package-lock.json` and `src-tauri/tauri.conf.json`; update `Cargo.lock` using Cargo. Run `npm run release:check`.
2. Review `docs/RELEASE_NOTES.md` and complete physical-device acceptance checks.
3. Run the manual preparation workflow and select the signing mode. It tests, builds all three targets and signs the update packages.
4. Download `FlowLink-prepared-development` or `FlowLink-prepared-production`. It contains the DMGs, Windows installer, Mac updater archives, `.sig` files, `latest.json`, platform metadata and `SHA256SUMS.txt`. Artifacts expire after 14 days; keep the exact tested files in protected storage.

The assembly step refuses incomplete platform sets, mixed versions, mismatched signing-key IDs, missing signatures or incorrect signed-version metadata. Cryptographic signature verification occurs in the app through the official updater before installation; assembly metadata checks are not a replacement for it.

If publication is explicitly approved later, upload the tested production files together to a stable GitHub release tagged `v<version>`. `latest.json` points to that immutable tag's filenames. Publishing a new stable release without a valid `latest.json` breaks checks for installed users. Do not replace published artifacts with different bytes under the same version, and do not mark a partially uploaded release as latest. Nothing in this repository automatically performs this publication step.

For local Mac update-package validation only (no upload), set `TAURI_SIGNING_PRIVATE_KEY` to the absolute path of the local key and run:

```sh
npm run desktop:build -- --ci --bundles app --config src-tauri/tauri.release.conf.json -- --locked
```

This builds a local app archive and signature. It does not install an update or publish a feed. Normal `desktop:build` still works without a signing key.

References: [Tauri updater](https://v2.tauri.app/plugin/updater/), [Mac signing and notarization](https://v2.tauri.app/distribute/sign/macos/), [Windows signing](https://v2.tauri.app/distribute/sign/windows/), [GitHub artifact downloads](https://github.com/actions/download-artifact).
