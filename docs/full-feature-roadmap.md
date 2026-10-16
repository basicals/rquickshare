# Full Quick Share roadmap

Current implementation, selected distribution/signing bindings and remaining gates:
[current state](current-state.md). Dated checkpoints below retain historical observations.

_Last updated: 2026-10-16_

The product goal is a native Linux client with feature parity and full interoperability with Google Quick Share. The user reconfirmed on 2026-10-12 that the intended desktop is in the notification/system-tray area near the clock and must migrate from Tauri to NativePair's Rust daemon / D-Bus / GTK4 direction. See [product-direction.md](product-direction.md) for the authoritative goals, initial compatibility matrix and migration history. Current choices and implementation gates are indexed in current-state.md.

Implemented capability markers below describe existing core/legacy source and
scoped checks; they do not establish native GTK feature parity or current-package
physical Google compatibility. Native file/card/notification ownership is now
implemented; daemon-owned persistent settings and in-app recovery are implemented
in [native-settings.md](native-settings.md), with physical migration acceptance pending.
Native received Text/URL review and explicit Copy are implemented in
[native-text.md](native-text.md). Outgoing native text/clipboard capture and shared recipient preparation are implemented
in [native-outbound.md](native-outbound.md). Native clipboard PNG images are implemented
in [native-image.md](native-image.md), with bounded validation, immutable anonymous
sources and explicit review/Prepare/Send. Explicit transfer-bound received-file
Show in folder is implemented in [native-reveal.md](native-reveal.md), with verified
read-only descriptor acquisition and owned portal requests. Physical KDE selection
remains acceptance. Native systemd user activation and direct verified Debian release
packaging are implemented in [native-package.md](native-package.md), with installed
lifecycle acceptance open. Selected opt-in login-start (daemon plus hidden-card
tray) is implemented in [native-autostart.md](native-autostart.md). Signed native APT
staging/trust/bootstrap tooling is implemented in [native-apt.md](native-apt.md);
R2 publication and outside-CI signing on the existing Linux workstation are selected;
private-bucket publication/approval tooling is in [native-r2.md](native-r2.md).
Production prefix routing and real signing identity are ready; exact release
approval, scoped publisher access, signed public APT and installed updates remain
open. Account/QR capabilities still require their own work. Latest completed slice:
[isolated native preview decoder foundation](native-preview-review.md), not remote
preview ingestion. [Completion audit](native-completion-audit.md) records remaining
everyday UI work and physical acceptance separately.

## Product principles

- Preserve interoperability with Android Quick Share before adding convenience features.
- Keep transport/protocol state machine changes covered by regression tests and Mode B.
- Prefer local/offline behavior. Google sign-in must never be required for ordinary nearby file/text sharing.
- Do not fake Google Account semantics: local trusted devices are a separate feature from Google's Contacts/Your devices identity.
- Keep OS-bound behavior (BlueZ, mDNS, native desktop lifecycle) behind explicit seams. Tauri is the current implementation and a migration baseline, not the target product shell.
- Account-backed identity, compatible QR and platform-specific Google capabilities remain in the parity target; unknown feasibility must be recorded rather than silently reclassified as optional.
- New functionality is developed directly on `dev`. `master` remains release-only; long-lived feature branches are not used.

## Capability matrix

### Foundation — implemented / hardened

- Android <-> Linux file transfer.
- Multiple file payloads.
- Inbound text/URL/Wi-Fi credential payload parsing.
- BLE discovery + modern Pixel GATT bootstrap.
- BLE -> Wi-Fi LAN bandwidth upgrade.
- Fixed listening port for firewall-friendly operation.
- Cancellation, progress state, collision-safe payload IDs.
- Wayland clipboard fallback.
- Background receive/tray lifecycle.
- Security hardening, bounded queues, size/path validation and CSP.
- Mode B/coverage/security/package CI.

### Phase 1 — complete the everyday Quick Share experience

1. **Modern sharing protocol model** — implemented on `dev`; wire compatibility tests exist
   - Preserve the existing Rust module namespace for compatibility.
   - Add modern wire fields by their canonical field numbers:
     - file parent folder, attachment hash and sensitive-content flag;
     - text sensitive flag;
     - Wi-Fi SAE;
     - app metadata;
     - stream metadata;
     - sharing use cases;
     - preview payload ids and transfer id;
     - response attachment details / resume metadata;
     - QR handshake field;
     - binding frames.
   - Add compatibility tests for old + new frames.

2. **Outbound text / clipboard** — implemented on `dev`; state-machine coverage and current-package hardware smoke remain
   - Send plain text, URL, address and phone-number metadata as BYTE payloads.
   - Share current clipboard text from UI.
   - Share clipboard images as regular image files without leaking temporary files.
   - Correct outbound Finished state and UX.
   - Tests for metadata classification, framing, payload bytes and cancellation.

3. **Folders** — implemented on `dev`, hardware smoke pending
   - Select files or folders; drag-and-drop directories use the same expansion path.
   - Recursively enumerate folders without following symbolic links. Empty folders are not representable by the Quick Share wire format and are skipped.
   - Preserve the selected directory name as the first `FileMetadata.parent_folder` segment. Example: selecting `Trip/` sends `Trip/photo.jpg` as `name = "photo.jpg"`, `parent_folder = "Trip"`; nested files use forward-slash relative parents such as `Trip/photos`.
   - Individual selected files keep an empty `parent_folder`.
   - Reject an outbound selection if recursive expansion yields zero regular files rather than sending an empty introduction.
   - On receive, accept forward- or backslash-separated peer parent folders but normalize to safe relative components. Reject absolute paths, `.`, `..`, empty/oversized/control-character components and excessive depth.
   - Create directories only after user consent. Never mutate the filesystem while merely parsing an Introduction frame.
   - Resolve an occupied top-level folder to one collision-safe root and apply that same remapping to every descendant, preserving the hierarchy instead of flattening or independently renaming children.
   - Never follow a received symlink out of the download root; canonical/root-boundary checks remain mandatory before file creation.
   - Tests cover nested folders, duplicate/root collisions, mixed files + folders, symlink inputs, empty folders, Windows-style separators, malformed parent paths and transactional failure.

4. **Custom device name — native implementation complete; real peer acceptance open**
   - Persist a validated display name.
   - Update mDNS endpoint info and outbound connection metadata without restart.
   - Keep the DNS hostname independent from the human-readable device name.
   - Native schema2/Settings2, live mDNS/GATT acknowledgements, immutable admitted
     session names and GTK default/reset are implemented in [native-device-name.md](native-device-name.md).
   - Name-only saves preserve active/expired temporary visibility and accepted
     receive-directory capabilities after a partial discovery failure. Native4
     package verified offline; installed KDE/Google checks remain separate.

5. **Trusted devices / local auto-accept**
   - [PairedKey authentication foundation](native-trust-review.md) implemented:
     bounded role/token-bound signatures, certificate time/hash checks and Fail
     gating; native7 verified offline. Production certificate provider is empty.
     Certificate acquisition/provenance/rotation, persistent trust and auto-accept
     remain unimplemented; offline crypto fixtures are not Google Account support.
   - Explicit local trust store keyed by stable peer identity material where available, not display name alone.
   - Per-device auto-accept toggle and revoke action.
   - Never auto-trust arbitrary "same name" peers.
   - Separate clearly from Google's account-backed Your devices / Contacts modes.

6. **Visibility and desktop UX**
   - Everyone / temporary visibility / hidden semantics.
   - Core-owned one-minute deadline, generation-safe expiry, read-only transport/
     native/legacy consumers and owned mDNS SDK cleanup implemented; native6
     verified offline. See [visibility contract](native-visibility-review.md).
     Physical withdrawal/suspend and native quick-control UX remain acceptance/work.
   - Tray visibility controls: source complete with current-policy status and
     replay-safe daemon commands; see [live visibility](native-live-visibility.md).
     Installed KDE/radio acceptance remains separate.
   - Background notifications with Accept / Reject / Cancel where supported.
   - Dark/system theme.
   - Localization architecture.
   - Better completed-transfer history.

### Phase 2 — advanced protocol features

7. **Preview payloads**
   - Prerequisite [bounded byte ingress](native-byte-ingress.md) implemented;
     this does not yet receive/display or advertise preview images.
   - User selected [isolated Glycin decoder](native-preview-review.md). Private
     Bwrap helper/resource limits/validated pixels/lifecycle and native8 offline
     Full/Compact fixture are verified. Remote encoding/mapping, daemon IPC/card
     binding and outgoing preview negotiation remain open.
   - Generate bounded image/video/file previews.
   - Advertise preview payload ids.
   - Receive/display previews before consent when peer supplies them.

8. **Resume and dedup**
   - Stable attachment hashes.
   - Receiver existing-file size / payload details.
   - Safe partial-file resume with integrity checks.
   - Never infer equality from filename+size alone.

9. **Stream payloads**
   - Implement STREAM framing and lifecycle.
   - Expose a clean stream abstraction independent of files.
   - Initial Linux use cases: pipe/stdin or application-generated streams.
   - Keep live media use cases optional.

10. **Remote Copy**
    - Current outbound text already advertises `REMOTE_COPY`; this does not prove complete capability negotiation or Android clipboard interoperability.
    - Implement `SharingUseCase.REMOTE_COPY` for clipboard-oriented transfers where Android peers support it.
    - Treat this as protocol capability negotiation, not merely a UI shortcut.

11. **Multi-recipient**
    - Queue multiple selected devices by default.
    - Add bounded concurrent transfers after resource/BlueZ testing.
    - Independent cancellation/progress per recipient.

12. **Application / Wi-Fi payload completeness**
    - Android app metadata including multi-APK/split packages where peers support it.
    - Wi-Fi credential outbound support including SAE.
    - Sensitive-content flags and safer UI.

### Phase 3 — required interoperability and parity work

13. **Quick Share QR compatibility**
    - Required for the parity goal; separate nearby Google QR from Google's service-backed cross-platform QR.
    - Modern proto contains QR handshake data, but Google's public Android QR flow also uses a Quick Share web page.
    - Reverse engineer the interoperable handshake before claiming compatibility.
    - A local-only QR fallback may be added separately and must be labeled as such.

14. **Google Account identity**
    - Required for Contacts / Your devices / account-backed auto-accept parity; a local trust feature is supplemental.
    - Research public certificates, account visibility and contact verification.
    - True Contacts / Your devices interoperability likely depends on Google identity/backend services.
    - Do not substitute a local trust list and call it Google account support.

15. **AirDrop interoperability**
    - Android Quick Share on selected devices can interoperate with AirDrop.
    - Linux support is a separate protocol/transport project (Apple discovery/AWDL/AirDrop), not a normal Quick Share frame extension.
    - Research as a separate transport/dependency workstream within the platform-specific parity matrix. No full-parity claim while applicable Apple paths remain unverified.

### Phase 4 — polish and distribution

- First-class Debian/Ubuntu packaging.
- AppImage after Wayland regression coverage.
- Optional Flatpak once Bluetooth/mDNS permissions are proven reliable.
- ARM64 builds.
- Context-menu / file-manager integration.
- DBus/share-target integration where desktop environments permit it.
- Autostart/background mode, update channel, diagnostics export.
- Accessibility, keyboard navigation and polished responsive UI.

## Clipboard ephemeral-file security boundary

Architecture decision (2026-10-06):

- `OutboundPayload::EphemeralFiles` remains a transport DTO for Tauri/TypeScript compatibility, but its strings are never deletion-capable by themselves.
- Core owns a dedicated `ManagedEphemeralFile` capability type. It can only be constructed after validating that the path is a direct child of the system temporary directory and matches the application-owned `rquickshare-clipboard-*.png` naming contract.
- `OutboundRequest::new` is fallible. Ephemeral payload paths are converted to `ManagedEphemeralFile` before a request exists; any invalid path rejects the request.
- Cleanup is performed only through validated `ManagedEphemeralFile` instances. Normal file payloads never participate in cleanup.
- The Tauri cleanup command uses the same core capability type/validation rather than maintaining a second path-security rule.
- Security invariant: no arbitrary user-selected or attacker-supplied path may acquire deletion capability merely by appearing in an outbound payload.
- Rollback is isolated to the managed-file type, constructor validation, and clipboard cleanup command; the serialized outbound payload shape remains compatible.

## Test policy

A prospective local Ubuntu + physical Pixel/ADB hardware automation harness is specified in [hardware-test-automation.md](hardware-test-automation.md). The proposed harness reduces manual interventions but does not waive any Mode B, Bluetooth interoperability or physical-device smoke gate.

- Every fixed protocol bug gets a regression test.
- Normal `dev` commits run the fast preflight once.
- The final Mode B gate runs the complete Rust and frontend automated suite 10 consecutive times.
- Package build is a separate manual 1x gate.
- Hardware/OS-bound features retain an explicit smoke matrix: Ubuntu/KDE Wayland + target Pixel first, then additional Android/Linux combinations.
- Coverage is used as a diagnostic map, not an arbitrary vanity threshold.

## Current implementation order

The native daemon/D-Bus/GTK shell, manual-sharing surfaces, desktop ownership and
decoder foundation and daemon-owned live visibility/quick controls are implemented.
See [completion audit](native-completion-audit.md) and [visibility contract](native-live-visibility.md).
Continue installed/physical acceptance separately and preserve the open account,
preview wire and resume gates. The list below retains broader feature dependencies;
it is not a claim that completed foundation slices need implementing again.

1. Modern protocol model.
2. Clipboard/text outbound.
3. Folder hierarchy.
4. Custom device name.
5. Trusted devices.
6. Visibility/tray/theme/i18n polish.
7. Preview + resume/dedup.
8. Streams / Remote Copy / multi-recipient.
9. QR/account/AirDrop research.

## Modern BWU endpoint identity compatibility

Hardware smoke on 2026-10-07 exposed intermittent image-transfer startup latency/failure while text payloads remained reliable. A protocol audit identified the following compatibility gap, now **implemented with regression tests on `dev`**:

- Modern `BandwidthUpgradeNegotiationFrame.ClientIntroduction` includes optional field 3, `last_endpoint_id`, used when the peer changes endpoint identity during dynamic BWU role switching.
- Previously, rQuickShare modeled only `endpoint_id` and routed a BWU socket only by that id; a peer changing endpoint identity could be rejected despite belonging to a pending upgrade.
- `validate_client_introduction`, `ClientIntroductionIdentity::matches_expected`, `BwuRouter::route_with_alias` and the TCP listener now preserve and use `last_endpoint_id`.
- Compatibility rule: prefer an exact current-id match, otherwise accept/reroute only when the advertised `last_endpoint_id` exactly matches a pending/expected endpoint. Never consume an unrelated pending route.
- Legacy peers that omit `last_endpoint_id` keep the existing exact-match behavior.
- Existing inbound/router/listener tests cover exact-match precedence, valid old-id aliases, missing/malformed identity and preservation of unrelated pending routes. Keep broader secure-session/migration hardware gates separate.
- Keep BWU phase timeouts unchanged until hardware logs distinguish TCP-route timeout from old-channel drain timeout; do not hide handshake defects by shortening timers.

