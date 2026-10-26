# Future features and optimizations

Updated: 2026-10-25. Applies to the FlowLink 0.2 development source.

This backlog records the remaining proposals after the optimization and convenience-feature pass. Priorities reflect the current Mac ↔ Windows software KVM architecture. Effort is relative, not a delivery estimate. No numerical performance improvement is claimed without physical measurements.

## Completed in this pass

- Discovery shares cached local address inventory with maintenance, avoiding repeated native scans in its event loop and registration path.
- Typed comparisons replace JSON-based inventory and UI change detection. Hidden windows receive no engine snapshots; reopening refreshes each window independently.
- Latency updates are isolated from full app state and throttled to approximately once a second.
- Session input reads/writes reuse frame buffers; outgoing JSON serializes directly behind the frame prefix. Wire format, size bounds, motion barriers, and input leases are preserved.
- Settings/layout drafts survive background updates. Settings accept external changes to untouched fields; page navigation protects edits with an explicit discard choice.
- Saves have pending/success feedback, repeated commands are guarded, and failed pairing submissions become actionable again.
- Pairing shows its actual expiry countdown and disables expired code submission.
- Devices includes setup readiness and connection guidance using current app state.
- Shortcut recording supports the existing physical-key vocabulary, with reserved source-chord checks.
- Versioned JSON backups export saved input/switching preferences and import a validated draft. Device identity, trust, network, pause, local-input permission, updates, and startup settings are excluded.
- Opt-in redacted diagnostic health reports can be saved to Downloads.

## Highest priority: reliability and measurable performance

| Proposal                                         | Effort       | Scope and acceptance                                                                                                                                                                                                                                                                                         |
| ------------------------------------------------ | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Physical Mac ↔ Windows acceptance and benchmarks | Medium       | Run both control directions on actual hardware over Wi-Fi, Ethernet, and the intended VPN. Record p50/p95/p99 round-trip latency, jitter, cursor response, reconnect/recovery time, idle CPU/memory, and high-rate input behavior. Follow `TESTING.md`; simulated input tests cannot replace this.           |
| Native display/network notifications             | Medium–Large | Subscribe to OS inventory changes and debounce refreshes. Keep a fallback scan and independent input watchdog. Verify docking, display removal, DHCP/VPN changes, sleep/resume, and permission revocation before reducing polling.                                                                           |
| Windows relative pointer capture                 | Large        | Evaluate Raw Input and cursor visibility/recentering behavior with 1–8 kHz mice and mixed DPI. Preserve pointer restoration and local emergency release. Ship only after hardware comparison with the current path.                                                                                          |
| Transport profiling and bounded batching         | Medium       | Measure allocation rate and queue age under motion, scrolling, and typing. Evaluate bounded batching or a compact encoding only if profiling justifies it. Preserve key/button order, control-token barriers, immediate release, and heartbeat leases. Reusable buffers are already implemented.             |
| Broader UI workflow automation                   | Medium       | Automate native-command failure/retry, draft updates during pause, display topology changes, concurrent command handling, window reopen freshness, backup import/export, and accessible modal navigation. Existing pure/SSR tests cover part of this; add a DOM/native harness when needed.                  |
| Signed distribution and live updater acceptance  | Large        | Integrate production macOS signing/notarization and Windows signing with the existing release tooling. Validate two successive installed versions, interrupted upgrades, retained trust/preferences, and safe input release. Publish only after physical acceptance and the focused pairing/security review. |

## Helpful everyday features

| Proposal                            | Priority / effort | First useful version and acceptance                                                                                                                                                                                                                                                                                               |
| ----------------------------------- | ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Connection test tool                | High / Medium     | Add bounded DNS/port checks and structured error stages for the entered destination. Explain selected interface, unreachable port, peer busy, and protocol mismatch. Keep ordinary checks distinct from authenticated peer identity. Current connection help only reports app state and guidance; it does not probe reachability. |
| Bounded diagnostic event history    | High / Medium     | Keep an opt-in ring of redacted state transitions, reconnect reasons, queue faults, and timing measurements. Define capacity, retention, clearing, and export. Never record key content, clipboard contents, secrets, device names, or addresses. Current exports are health snapshots only.                                      |
| Opt-in plain-text clipboard sharing | High / Large      | Begin with a manual send action, per-peer opt-in, a payload limit, loop prevention, and explicit direction controls over the authenticated session. Handle ownership changes and stale sessions. Add negotiated protocol support before enabling automatic synchronization.                                                       |
| Per-device input preferences        | Medium / Medium   | Remember pointer speed, scrolling, and shortcut profiles by trusted device and control direction. Migrate existing global preferences with predictable defaults; forgetting a peer removes its profile.                                                                                                                           |
| Workspace display profiles          | Medium / Medium   | Save named home/office/docked arrangements against stable display identities and main-device identity. Offer to restore a matching valid profile after docking; never silently activate a stale arrangement.                                                                                                                      |
| App-specific shortcut mappings      | Medium / Medium   | Extend terminal exceptions into receiver-side application profiles with bundle IDs or executable identity. Retain a key-down mapping through key-up when the foreground application changes. Integrated terminal pane detection requires separate design.                                                                         |
| Keyboard-language awareness         | Medium / Large    | Show the receiver's active input source first. Add optional synchronization only for supported platform pairs, with IME/composition tests and a clear fallback. Avoid changing the user's local input source unexpectedly.                                                                                                        |
| Configurable switching shortcut     | Medium / Medium   | Reuse chord parsing/recording for the switch action, with collision checks on both platforms and a reliable emergency shortcut. Requires native hook/settings changes; the current recorder edits custom mappings only.                                                                                                           |
| Control ownership overlay           | Medium / Medium   | Offer an optional small overlay showing the receiving computer and local return shortcut. Respect fullscreen apps, accessibility, multiple displays, and paused/local states.                                                                                                                                                     |
| Advanced edge switching             | Medium / Medium   | Add active edge segments, optional modifier-assisted crossing, and temporary edge locking. Visualize active zones and test corners, held input, rapid returns, and mixed DPI.                                                                                                                                                     |
| Multiple controlled displays        | Medium / Large    | Extend the current one-selected-remote-display geometry to several remote displays, including return edges and saved layouts. Validate overlaps, gaps, docking, portrait modes, and topology changes.                                                                                                                             |

## Later expansion

| Proposal                     | Effort           | Prerequisites and acceptance                                                                                                                                                                                      |
| ---------------------------- | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multiple connected computers | Large            | Define session ownership, peer selection, per-peer layouts, and deterministic hotkeys. Test simultaneous connection attempts and guarantee only the intended receiver accepts input.                              |
| File transfer                | Large            | Add explicit send/receive, destination approval, size limits, progress/cancellation, and bandwidth fairness. Bulk data must never delay input release or heartbeat handling.                                      |
| Linux platform backend       | Large            | Start with a documented desktop/session support matrix and permission model. Match existing capture, injection, topology, watchdog, and recovery semantics. Prioritize after Mac/Windows reliability is measured. |
| Native multi-finger gestures | Large / Research | Determine which gestures can be represented safely and consistently across Quartz and Windows input APIs. Keep ordinary scrolling and momentum independent of experimental gesture forwarding.                    |
| Direct USB networking        | Large / Research | Establish supported network-adapter modes and driver requirements. Ordinary direct Ethernet already works as IP networking; avoid bundling custom drivers until there is a demonstrated need.                     |
| Macro engine                 | Large / Later    | Define bounded, cancellable execution and key/button cleanup first. Preserve emergency release through every step. Keep separate from single-chord remapping.                                                     |

## Suggested implementation order

1. Record physical acceptance and performance baselines; complete installed-app upgrade testing.
2. Add structured connection tests and bounded diagnostics; introduce native inventory notifications with a fallback scan.
3. Add per-device preferences, workspace profiles, configurable switching, and application mappings.
4. Introduce opt-in clipboard sharing with negotiated protocol support.
5. Revisit multi-display/multi-peer expansion, files, Linux, gestures, and USB networking after the earlier milestones are validated.

Every networking/input change should retain the existing fail-closed capture gate, handoff tokens, independent watchdog, and explicit control resumption after reconnect. Use measured results to decide whether more invasive optimizations are necessary.
