FlowLink 1.1.0 — input responsiveness and keyboard mappings

- Combine queued incoming mouse movement before native injection, preserving displacement and click/key ordering.
- Enumerate local interfaces once per connection route selection.
- Add configurable Command ↔ Ctrl, Option ↔ Win, Control ↔ Alt mappings, including right-side modifiers.
- Preserve existing saved keyboard modes; select Settings → Keyboard preset → Custom modifier keys to enable the new mapping. New installations use it by default.
- Show which computer is paused and reconcile peer pause status through heartbeats.
- Include the Connect crash fix, automatic listener recovery, and fallback to a free TCP port when the default port is unavailable.

Update both computers to 1.1.0 before connecting. This version uses protocol 3 and cannot connect to 1.0.x.

Windows installers are unsigned. macOS builds are ad-hoc signed and not notarized. Operating-system security warnings may appear. Updater packages are cryptographically signed.

Automated regression checks pass locally; physical Mac/Windows performance, shortcut and input acceptance testing remains outstanding. Latency improvements have not been measured on physical device pairs.
