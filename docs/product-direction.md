# Product direction: native desktop and Google Quick Share parity

_Recorded: 2026-10-12. Status: tray-panel and descriptor decisions accepted;
file/core boundary, coordinator, native runtime, fifteen authenticated Manager1
methods and directed state invalidations implemented; the native daemon executable
and hidden visibility recovery are now implemented. Bounded current-run history
and seven additive Operations1 methods are implemented. A dedicated native
operation client now reconciles replies and acknowledges automatically; its state
projections, invalidations and consent/cancel controls are implemented; the
graphical shell and bounded transfer detail contract remain pending._

Coordinator implementation and one-run development verification are recorded in [native-service-coordinator.md](native-service-coordinator.md). Bounded TCP workers and the owned native dispatch/cancellation/startup seam are implemented in [native-transfer-supervision.md](native-transfer-supervision.md) and [native-managed-dispatch.md](native-managed-dispatch.md). Common TCP/BLE session capabilities and consent processing receipts are implemented in [native-session-controls.md](native-session-controls.md). The single-owner coordinator/core runtime is implemented in [native-core-runtime.md](native-core-runtime.md), using [ordered lifecycle events](native-core-events.md). Authenticated selection IPC is implemented in [native-selection-transactions.md](native-selection-transactions.md), with additive [transfer controls](native-dbus-controls.md) and [bounded state pages/invalidation](native-dbus-state.md). A registered daemon and native UI remain pending. Historical stage descriptions below must be read with these checkpoints.

The subsequent [native daemon startup](native-daemon-startup.md) implements the
manual executable, shared launch guard, read-only existing settings and hidden
visibility recovery. Manager1 now has fifteen methods, including startup warnings.
The daemon is not installed/activated, and GTK/settings/Dolphin remain pending;
the old desktop remains the current UI and cannot attach to the native process.

[Native retention and ordered receipts](native-retention.md) now implement the
approved 64-record safely closed current-run history and additive Operations1.
[Native client operations](native-client.md) now automatically reconcile and
acknowledge these receipts on a dedicated, daemon-pinned connection.
History is not persisted and this does not establish Google feature parity.

## Authoritative correction

The user clarified that the intended product includes:

1. A redesigned experience in the notification/system-tray area near the clock, replacing the current main-window presentation.
2. Migration away from Tauri to the technology direction chosen for NativePair.
3. Feature parity and full interoperability with Google Quick Share, not only basic Android file transfer.
4. A file-manager context-menu action to send selected files/folders through rQuickShare.

These goals supersede the old roadmap order that put custom device naming next and treated native desktop migration as absent from scope. The completed offline maintenance slice is infrastructure work, not completion of this product goal. Do not silently reduce parity to LAN files or relabel local substitutes as Google-compatible features.

## NativePair reference, verified locally

Reference repository: `/home/morozhenko/Documents/GitHub/NativePair`, inspected at `da8f4811940bccd9026e86ba20a68d46140dd1a8`.

- `docs/ARCHITECTURE.md` and `docs/adr/0001-headless-rust-daemon.md` select Rust, a headless daemon, versioned D-Bus clients, and GTK4/gtk-rs for the future desktop GUI.
- NativePair has core/daemon/CLI crates; its GTK4 GUI is a planned milestone, not an existing UI to copy.
- rQuickShare should follow that architecture direction. It remains a separate project and service; this instruction does not authorize merging repositories, copying Apache-licensed NativePair into a GPL fork without review, or changing either project's license.
- NativePair was inspected read-only. No changes were made there.

## Architecture direction and invariants

- Keep the Rust Quick Share core and its interoperability/security invariants.
- Move background ownership out of the GUI lifecycle into a Rust service; use a thin native GTK4 client and a versioned local D-Bus boundary following NativePair's direction.
- The service owns discovery, transfer sessions, consent state, bounded queues and graceful mDNS/BlueZ shutdown. GUI clients must not create competing transport sessions.
- Closing the panel/client must not terminate a transfer. Explicit Quit/shutdown semantics must be specified and tested.
- Preserve existing settings/downloads/ephemeral-file ownership during migration; no destructive implicit reset.
- Preserve one notification/consent lifecycle per transfer and clear stale actions after terminal state. A persistent popup without cancellation cleanup is insufficient.
- Account identity, visibility and auto-accept must be authenticated. Display-name matching or a local trust list does not implement Google's account/contact identity.
- Keep ordinary local sharing usable without mandatory cloud sign-in; implement Google's account-dependent capabilities explicitly where required for parity.
- Migration and protocol expansion are separate change sets so regressions can be attributed and rolled back.

Exact public D-Bus methods/versioning, caller authorization, persistence migration and desktop integration require a concrete design review before implementation. This document establishes direction, not an implicitly approved detailed API/security contract.

## Accepted desktop interaction

The user selected option A on 2026-10-12 and explicitly rejected the current oversized drop-zone window. The primary experience is a compact tray panel with native desktop notifications. The context-menu sender must enter the same recipient-selection flow; it must not open the old full-size window or start an independent protocol instance.

| Option | Description | Pros | Cons | Risks |
|---|---|---|---|---|
| A — tray panel plus native notifications (recommended) | Tray icon opens a compact device/send/receive/progress panel; incoming requests and background status use desktop notifications; settings are secondary | Persistent place for device selection, transfer controls and recovery; fits full feature surface | Requires panel placement/focus/keyboard/multi-monitor design | GTK4/Wayland anchoring near an external tray must be proven on target KDE; never promise it from a positioned ordinary window |
| B — notification-led interaction | Incoming operations use system notifications; a tray menu exposes controls and a separate native sender/settings view when needed | Strong desktop notification integration; small always-on surface | Device selection/history/advanced operations need an additional view | Notification expiry, Do Not Disturb and action availability can hide controls; these need explicit fallback semantics |

Decision: A, because the parity target needs a durable device chooser and transfer controls as well as notifications. Option B is retained above only as decision history. A dominant empty drop zone is not part of the accepted design. Drag-and-drop may remain a secondary input on the compact panel. Settings are a secondary surface.

The concrete service, file-selection trust boundary and migration proposal is in [native-desktop-design.md](native-desktop-design.md). The selected interaction and descriptor boundary no longer require an A/B question. Record the concrete wire contract before publishing the service API; ask only about new significant decisions outside the accepted design.

The user subsequently approved descriptor/capability input. The reusable Linux acquisition/owned-file core route is implemented and validated in [native-file-input.md](native-file-input.md); the old path-opening D-Bus proposal must not be implemented. The coordinator is implemented; the live daemon/D-Bus adapter and GTK4 shell remain pending.

GTK4 removed cross-backend global positioning APIs; a generic GtkWindow cannot be assumed to provide a tray-anchored panel. Validate supported compositor/desktop integration first. Source: [GTK4 migration guide](https://docs.gtk.org/gtk4/migrating-3to4.html). The current host does not expose `gtk4` through pkg-config; dependencies are not installed by this documentation pass.

## Parity and compatibility matrix

The target is the Google Quick Share capability surface, mapped by platform/version/hardware. A platform-specific or service-dependent feature remains in the target with its dependency recorded. This table is an initial baseline, not a claim that every current Google capability has already been inventoried.

| Capability | rQuickShare evidence now | Required next evidence/work |
|---|---|---|
| Nearby file/image/video/document transfer | Implemented; dated Pixel evidence exists | Current Android and Google Windows bidirectional matrix |
| Multiple files and folder hierarchy | Implemented; current folder smoke open | Peer tree/bytes/collision verification |
| Clipboard/text/link sharing | Helpers/UI implemented; REMOTE_COPY emitted | Text state-machine tests and actual peer clipboard semantics |
| Sender/receiver names and pairing PIN | Basic metadata present; custom name pending | Consistent editable identity and peer PIN validation UX |
| Background receive, progress/cancel, request actions | Implemented partially in Tauri; hardware lifecycle gaps remain | Native service/client lifecycle and synchronized peer outcomes |
| Everyone/temporary/hidden visibility | Partial local modes | Google-compatible timing, restoration and authenticated identity modes |
| Google Account, Contacts, Your devices, own-device auto-accept | Not implemented | Identity/certificate/service feasibility, secure credentials and real peers; required parity work |
| Google-compatible nearby QR | Proto field alone | Actual Google handshake, encode/decode and Android camera/Quick Share flow |
| Multiple recipients/queueing | Bounded core concurrency/FIFO foundation implemented; native recipient UX/policy pending | Independent recipient state; bounded parallelism only where supported |
| Discovery and transport combinations | BLE/GATT and Wi-Fi LAN implemented | Per-platform transport inventory, network-isolation/direct-link scenarios and measured cold discovery |
| Desktop share integration | User requested context-menu sending; not implemented | Dolphin service-menu adapter first for the target KDE environment, shared recipient flow, then other desktop adapters; accessibility and package checks |
| Google cross-platform AirDrop/Apple paths | Not implemented | Separate transport/backend feasibility and physical peer matrix |
| Google's encrypted cloud QR to Apple browsers | Not implemented | Google service access/identity/limits feasibility; a custom server is not equivalent |
| Device-dependent recent features, including tap-to-share | Not inventoried | Platform/hardware applicability and current official-source inventory |

Official references checked on 2026-10-12:

- [Android Quick Share help](https://support.google.com/android/answer/9286773?hl=en): nearby sharing, clipboard, QR, multiple receivers, account visibility/acceptance and Apple interoperability routes.
- [Google Quick Share for Windows help](https://support.google.com/android/answer/13801258?hl=en): files/folders, device naming, account visibility, background notifications and file-manager sending.
- [Tap-to-share help](https://support.google.com/android/answer/17161144): a separate item in the current capability inventory; do not assume Linux hardware equivalence.

Separate **schema modeled**, **implementation present**, **automated contract passed**, **real Google peer verified**, **blocked**, and **unknown**. No one status implies another. Record Google app/OS/build, payload, transport, network and identity mode for physical evidence. Full compatibility is the target; it is not an achieved claim or a guaranteed timeline for undocumented/service-backed mechanisms.

Additional protocol capabilities already in the roadmap (previews, resume/hash integrity, streams, Wi-Fi/app metadata) remain planned; determine peer support and product applicability rather than claiming every protobuf field is a user-visible Google feature.

## Staged implementation plan

1. **P0 — direction and acceptance inventory.** Record this correction, confirm A/B interaction, enumerate the versioned Google feature/peer matrix and produce a concrete service/API/security design for approval. Documentation-only rollback. Status: option A and context-menu goal accepted; concrete service proposal recorded; contract review and complete Google capability inventory pending.
2. **P1 — background service boundary.** Reuse existing core behind a thin Rust service, add deterministic lifecycle/consent/snapshot/reconnect contracts and a headless client. Keep protocol framing/crypto unchanged. Rollback: keep the existing desktop build usable while the new path is validated.
3. **P2 — native desktop shell and UX.** Add GTK4 desktop client, selected tray/notification behavior, settings, device selection and transfer controls. Prove KDE/Wayland placement and fallback behavior before presenting a final visual design. Rollback: isolated shell/package changes; no persisted-data loss.
4. **P3 — migrate desktop capabilities and packaging.** Transfer clipboard, file dialogs, reveal/open, autostart, logs, updates and settings migration. Document per-feature old/new equivalence. Retire Tauri/Vue only after native feature and package checks pass. Rollback: independent commits and preserved settings backup/migration contract.
5. **P4 — close Google parity gaps in vertical slices.** Each feature receives protocol research, branch/negative/interaction inventory, deterministic tests and real Google peer acceptance. Account/QR/Apple/service dependencies are release blockers for claiming full parity, not optional tasks silently dropped from the goal. Keep transport and UI changes separable.
6. **P5 — release acceptance.** Complete core/service/native-client tests, ten consecutive stability runs, package/install checks and the physical compatibility matrix on the final implementation. Publish only capabilities with evidence; promote `master` only under the existing release policy.

These milestones are not time estimates. The user has answered the material UX and descriptor choices; the offline file/core implementation has started the migration. The daemon/API/UI are still pending. Continue within the approved direction, documenting concrete contracts before implementation and asking only for new significant decisions. Do not use the completed Tauri maintenance checks as native-shell verification.
