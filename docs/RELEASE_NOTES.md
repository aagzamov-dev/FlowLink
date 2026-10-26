FlowLink 1.0.1 — connection fixes

- Prevent the Connect action from crashing when invoked from the desktop UI.
- Retry a failed TCP listener every two seconds and clear its error after recovery.
- Use separate IPv4 and IPv6 listeners without overlapping address families.
- Release listening sockets when the engine shuts down.
- Reject invalid connection addresses before changing connection state.

macOS builds are ad-hoc signed and not notarized. Windows publisher signing is not configured. These fixes do not establish the original cause of the reported Windows bind failure; persistent failures retain the OS error for diagnosis.

Install the patched version on both computers. Physical Mac-to-Windows connection acceptance testing is still required.

---

FlowLink 1.0.0

- Control Windows from Mac or Mac from Windows, with a choice of main device.
- Customize directional shortcuts and tune pointer and touchpad scrolling.
- Recover local input after stalled connections and reconnect over permitted LAN/VPN routes.
- Check for verified app updates, download them, and install when you are ready.
- Follow a setup checklist and connection guidance, record shortcut combinations, and protect unsaved edits.
- Export/import portable input preferences and save opt-in redacted diagnostic health reports.
- Reduce redundant discovery scans, UI state traffic, and input frame allocations; retain the existing input ordering and recovery rules.

Install matching versions on both computers. Version 1.0 cannot connect to 0.1.
These installers are not publisher-signed: Windows may show an unknown-publisher warning; macOS builds are ad-hoc signed and not notarized. Updater packages are cryptographically signed. Physical Mac/Windows, VPN, installation and update acceptance testing remains outstanding.
