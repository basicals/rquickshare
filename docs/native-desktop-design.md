# Native desktop migration and context-menu sender

_2026-10-12. Mode A. Status: descriptor input approved; offline file/core boundary implemented and validated._

Current integration checkpoint: the single-owner coordinator/core runtime and ordered native session events are implemented and verified in [native-core-runtime.md](native-core-runtime.md) and [native-core-events.md](native-core-events.md). Earlier pending-adapter descriptions below are historical prerequisites; authenticated D-Bus, activated daemon, native UI and physical Google parity remain open.

## Accepted scope and source of truth

[product-direction.md](product-direction.md) records the product goals. The user selected the compact tray panel plus native notifications and requested file-manager context-menu sending. The existing oversized drop-zone window must be replaced. NativePair's Rust daemon, D-Bus and GTK4/gtk-rs direction is accepted; this document proposes the remaining process/API/security contract.

The applicable agent guide is `/home/morozhenko/.codex/AGENTS.md`, also supplied by the user in the chat. Sections 3 and 3.1 require approval for significant public API/security decisions. Naming, crate layout and other reversible implementation details are autonomous. Repository policy remains direct atomic commits on `dev`, with no feature branch or standing release PR.

## Analysis of the current implementation

| Module | Current responsibility | Migration consequence |
|---|---|---|
| `core_lib/src/lib.rs` | `RQS` starts transports, owns cancellation/task tracking, discovery and visibility channels | One service instance must own `RQS`; retain transport/protocol behavior |
| `core_lib/src/manager.rs` | Bounded outbound queue, TCP sessions and per-request transfer routing | Adapt through a service command layer rather than exposing sockets to clients |
| `core_lib/src/channel.rs` | Broadcast transfer updates and front-to-core accept/reject/cancel messages | Service maintains authoritative state; a broadcast alone cannot recover missed updates |
| `core_lib/src/hdl/outbound.rs` | Opens paths, expands folders, skips symlinks, owns managed ephemeral cleanup | Path versus descriptor input is a real security/API choice; preserve folder and ownership contracts |
| `app/main/src-tauri/src/main.rs` | Creates `RQS`, tray, event bridges and notifications; second instance opens main window | Extract service ownership before retiring Tauri; never run old and new transport owners together |
| `app/main/src-tauri/src/store.rs` | Tauri-backed settings, including visibility and download path | Separate migration stage with backup; no silent defaults/reset |
| `app/main/src/composables/ContentStatus.vue` | Large drop area, file/folder picker and device-selection state | Replace presentation, retain actual sending capabilities |

The old frontend is not a native-shell template. GTK4 is not available through pkg-config on this host; NativePair has no implemented GTK UI to reuse. No package installation, live Bluetooth session or desktop activation has been performed for this design.

## Interaction contract selected by the user

- Tray activation presents the compact nearby-device/transfer panel. The primary surface has no dominant empty drop zone.
- A context action passes the selected files/folders to the same native sender and presents recipient selection. Selection alone never starts a transfer.
- Explicit recipient selection/send starts the operation. Cancelling the chooser releases its pending selection without deleting original files.
- Closing the panel hides the client surface; it does not cancel active transfers. Transfer cancellation is an explicit action.
- Incoming consent remains available through the panel as well as native notifications, including when notifications are suppressed or expired. Do not silently auto-accept.
- Settings are secondary. Do not reintroduce the large window as the normal send flow.

Exact visual layout and compositor placement still need a reviewable prototype. GTK4 generic toplevel positioning does not guarantee anchoring to an external tray on Wayland. First prove the KDE/Wayland integration; if it requires a different shell technology or materially different fallback, bring that decision to the user before implementing it.

## Proposed process and trust boundary

Common to both options below:

- A separate unprivileged Rust daemon owns one `RQS` per user bus, discovery, transfer state and notifications. GTK4 and the CLI are thin clients, using zbus as in NativePair.
- No root helper, system-bus application endpoint, local TCP control API or Google-account credential API is introduced in this slice.
- Verify callers using bus-authenticated credentials for the actual message sender; require the daemon UID and fail closed on unresolved credentials. A claimed username/PID/application ID is not authorization.
- Same UID is a desktop trust boundary, not isolation from malicious software running as that user. Do not advertise it as a sandbox permission model.
- Bus-name ownership is acquired before transport startup; failed ownership must not bind transports. The Tauri rollback build and new service are mutually exclusive launch modes until both use this boundary.
- Commands are bounded and return structured errors. Slow/offline clients cannot block transport work. Notification actions target a transfer ID and consent revision; stale/terminal actions cannot apply to a newer request.
- The service keeps a revisioned snapshot for reconnecting clients; subscribe before snapshot retrieval, then reconcile revisions. Missed core events cannot be represented as a successful synchronized view: mark affected state uncertain and recover from an authoritative source, or reject the operation. Add a minimal core snapshot seam if required; do not invent missing transfer outcomes.
- Closing clients does not stop the daemon. Explicit stop is separate from Hide and requires transfer-aware handling before the native UI exposes it. Service shutdown must invoke and await core shutdown.
- No persisted active-transfer recovery, Google identity, automatic trust, visibility-default change or destructive settings migration is included in the first service slice.

### Accepted outbound file-selection contract

| Option | Description | Pros | Cons / risks |
|---|---|---|---|
| A — same-user local paths first | Thin clients submit selected local file/folder paths; the same-UID daemon validates and opens them using existing core preparation | Preserves current core API and folder behavior; isolates desktop migration from protocol/file-reader refactoring | Same-user callers can request any daemon-readable path; names can change between selection and open; not sufficient as a sandbox boundary |
| B — descriptor/capability input first (recommended after security review) | Clients open selected regular files and submit handles plus bounded relative-name manifests; folders require a defined enumeration/handle contract | Pins opened objects across rename/replacement and supports a narrower selected-object model | Requires core input/ownership refactor now, directory semantics, descriptor-pressure handling and independent compatibility tests; FDs alone neither authenticate user intent nor freeze file contents |

Decision: the user approved B for the durable service contract. The earlier recommendation of A prioritized migration isolation and reuse, not the strongest file-access boundary. Keep paths at the desktop ingress adapter and pass acquired resources across the service boundary. Do not first publish a path-opening daemon API that will need replacing.

### Security review and accepted B contract

Passing paths is not inherently a remotely exploitable vulnerability: the proposed daemon is unprivileged, same-user and has no network control endpoint. However, its file-opening API would become a confused-deputy risk if a less privileged/sandboxed caller could use it to read files that caller cannot access. Same UID does not prove user intent or identical sandbox permissions. Broad local control methods remain a separate authorization concern even with FD input.

Source inspection found a concrete check/open race in current outbound preparation: `collect_outbound_file_candidates` checks metadata/symlinks, then `prepare_outbound_files` later uses `File::open(path)`. Folder traversal similarly resolves directory names repeatedly. An intervening rename/symlink replacement can change the opened object; checking that the result is a regular file does not establish that it is the originally selected object. This is a source-level finding, not an executed exploit or evidence of a remote attack. Existing non-racing symlink tests do not cover this race. Do not describe the existing file preparation as race-safe.

Accepted invariants for option B:

1. The CLI/file chooser resolves paths only in an ingress adapter. Acquire read-only file handles safely, then validate the acquired object. Never use a check-then-ordinary-open sequence as a security guarantee. Folder acquisition uses a root directory handle and confined descriptor-relative traversal (for example `openat2` resolution restrictions), rejecting symlinks/escapes and special files. Root selection itself still needs a safe acquisition policy; no design can retrospectively prove which object a file manager displayed before invocation.
2. The service accepts owned read handles and bounded metadata for regular files; file names are display/protocol metadata, not host paths to reopen. Use explicit snapshot/selection types rather than `Vec<String>` in the core adapter. Wire framing and crypto remain unchanged. Original source files never gain delete/cleanup ownership.
3. Folder support must not hold an unbounded FD per descendant. Use bounded enumeration/batches or a confined root-directory capability, with an explicit producer lifetime and cancellation contract before implementation. Directory capabilities grant subtree access and require their own validation; they are not equivalent to a single-file handle. No descriptor-exhaustion fallback to reopening absolute paths.
4. Copy received FD ownership correctly, enforce resource budgets, validate type/access mode, and make reads independent from a caller-controlled shared file offset (for example positioned reads). Handle close/replay/disconnect/cancellation explicitly. Validate manifests against traversal, collisions and limits. A pathname supplied alongside a handle is not authority.
5. A stable opened object is not immutable content. External writes/truncation, growing files, unmount/I/O failures and mid-read changes remain negative scenarios. Metadata checks alone cannot reliably detect every concurrent modification. Stable byte snapshots need a separate snapshot/copy policy with storage quotas, cleanup and source-consistency semantics; neither ordinary handles nor a naive copy provide an atomic snapshot of a concurrently written source. Do not claim such guarantees in this slice.
6. Caller credentials, per-selection ownership and explicit send/consent remain necessary. FD possession proves access to the supplied object, not a human click. Future sandbox isolation requires bus/portal access policy and containment of the file-acquisition component; descriptors alone do not sandbox a broadly privileged daemon. Portal integration is an adapter, not a mandatory cloud dependency.

Required test risks now include deterministic replacement between check/open, replaced directory ancestors, symlink/magic-link escapes, special files, inaccessible sources, manifest traversal/collisions, descriptor/count limits, shared-offset interference, stale ownership, cancellation cleanup, partial enumeration and source mutation/I/O errors. These are requirements for the scoped test inventory before coding, not implemented coverage claims.

### Proposed public contract for option A

The path-based contract below is retained as the reviewed alternative, not the recommended API. Option B must replace `CreateSelection(paths)` with a versioned handle/manifest acquisition contract; folder batching/ownership must be specified before that API is frozen. No introspection fixture has been published or implemented yet.

Names below are proposed for the initial Linux service; freeze introspection XML and version fixtures in the implementation commit. Do not expose internal `ChannelMessage` or its arbitrary direction/action fields as the public API.

| Surface | Proposed contract | Invariants |
|---|---|---|
| CLI | `rquickshare send -- <path-or-file-uri>...` | Opens recipient selection; no shell interpretation, no implicit recipient, no transmission from invocation alone |
| Manager | `io.github.morozhenko_apps.RQuickShare`, `/io/github/morozhenko_apps/RQuickShare`, interface `io.github.morozhenko_apps.RQuickShare.Manager1` | Same-user bus only; incompatible APIs receive a new major interface |
| Read state | `GetSnapshot() -> (epoch: s, revision: t, snapshot_json: s)` | Strict versioned JSON schema, bounded size, no secrets; new epoch on daemon restart; clients cannot reuse old IDs |
| Selection | `CreateSelection(paths: as) -> selection_id: s`; `DiscardSelection(selection_id: s)` | No send; caller-bound pending selection; discard idempotent; originals never become cleanup-owned |
| Send | `SendSelection(selection_id: s, endpoint_id: s, request_id: s) -> transfer_id: s` | Valid discovered endpoint, explicit send, repeated same request ID/arguments returns same result; conflicting replay rejected |
| Consent | `Respond(transfer_id: s, consent_revision: t, accept: b)` | Current inbound request only; duplicate same answer is harmless; conflicting/stale response rejected |
| Cancel | `Cancel(transfer_id: s)` | Explicit operation; repeated terminal cancellation cannot resurrect the transfer |
| State change | `StateChanged(epoch: s, revision: t)` | Invalidation hint; clients obtain current snapshot; coalesce progress to avoid unbounded notification traffic |

Bus scalar codes: `s` string, `as` string array, `t` unsigned 64-bit integer, `b` boolean. JSON is a proposed explicitly versioned snapshot encoding, not a permission to pass arbitrary core structures through IPC. Schema fixtures and input bounds must be documented before implementation. A native UI activation endpoint is separate from the headless service and will carry only a selection reference, not payload contents.

Pending selections are owned by their calling bus connection; the desktop client creates them after receiving CLI activation arguments. Caller disconnect releases unsent selections. Once sent, a transfer belongs to the service and survives caller disconnect. Invalid selections must not silently send only the valid subset: return an actionable validation error before a transfer is queued. Existing core behavior outside the new selection boundary remains unchanged.

Initial service read/control scope is transfers and selections only. Visibility, persistent settings, account credentials and download-directory migration are reviewed in their later slices; existing values are preserved during initial migration.

## Context-menu integration proposal

The first target adapter is Dolphin because the current desktop acceptance target is KDE. This is not a claim of coverage for all file managers. Other file-manager adapters can reuse the same CLI without changing the daemon API.

Ship a package-owned service-menu `.desktop` entry for local regular files and folders, using `X-KDE-Protocols=file` and `Exec=rquickshare send -- %U`. Each URL is its own argument; never use `sh -c`, string concatenation or evaluation. The parser handles local paths/file URIs once, preserves Unicode/spaces/percent signs, and rejects unsupported remote schemes with a visible error. Do not secretly download KIO/remote content. Disabling non-local actions is an initial adapter applicability boundary, not a claim of full Google parity.

The entry is installed by the application package with an uninstall counterpart. Do not alter the user's live Dolphin configuration or add an entry pointing at a nonexistent binary. Dolphin registration must wait until the CLI and recipient chooser work end to end.

## Implementation milestones, validation and rollback

| Stage | Changes / documentation | Verification | Risk / rollback |
|---|---|---|---|
| D0 — this review | Record accepted UX/context-menu scope, current coupling, options and concrete service proposal | Read sources; inspect docs diff | Documentation only; revert this commit |
| D1 — offline service foundation after approval | Rust service/client crates, fixed API schema/introspection for the selected file boundary, bounded command/state coordinator and injected core adapter; record branch/negative/interaction inventory before tests | Actual coordinator tests for replay, reconnect, stale consent, terminal cancellation, queue limits, disconnect and shutdown; private bus integration | No GUI/BlueZ in unit tests; retain old app and no live registration |
| D2 — existing-core adapter | One transport owner, supervised startup/shutdown, current settings read without migration; for B, replace path reopening with owned readers and bounded folder acquisition; document old/new mapping | Existing suites plus adapter lifecycle/selection/security tests and ten consecutive runs on final slice | File-input refactor is explicit and separately reversible; protocol framing/crypto unchanged; mutually exclusive old/new launch mode |
| D3 — native tray panel | GTK4 client, native notifications and proven KDE/Wayland activation/placement; review prototype before material layout/fallback decisions | Offline state projections, reconnect/notification actions, keyboard tests; physical KDE checks later | No claimed tray anchoring before evidence; isolated desktop code |
| D4 — context action | CLI activation, shared recipient flow, Dolphin service-menu package assets | CLI URI/argument boundaries, cancellation, multi-selection/folders, missing files, package install/uninstall; physical file-manager smoke later | No shell command interpolation; originals never deleted; remove adapter package assets to roll back |
| D5 — desktop completion | Clipboard, settings backup/migration, autostart, reveal/open, updates and package equivalence | Old/new capability checklist, full suites, final hardware gates | Retire Tauri only after equivalence and migration acceptance |

The native migration does not complete Google parity. The full feature/peer inventory and protocol/account/QR/Apple work remain separate required milestones in [product-direction.md](product-direction.md). Existing maintenance stability results do not validate unimplemented native code.

## Progress / handoff

- D0 analysis and proposal: complete.
- User interaction decision: option A accepted; oversized drop zone rejected; context-menu sender requested.
- File/security decision: B approved. The A API table is historical only and must not be implemented. No additional approval is needed for the already accepted descriptor-based file-input refactor.
- The user also authorized immediate fixes to brittle internal workarounds across projects, asking before substantial refactors. The preference is persisted in the global agent guide. It does not authorize unrelated cleanup.
- The first implementation slice and complete scoped branch/test inventory are recorded in [native-file-input.md](native-file-input.md). This implements the reusable file resource boundary and core entrypoint before publishing D-Bus methods. No desktop registration, settings migration or process activation is part of that slice.
- That slice is complete in implementation commit `c8d537e`: 62 new methods, all 258 automated checks passing ten times, and 18 assertion-killed copied-source mutants. This is the file-boundary portion of D1/D2; the subsequent coordinator is recorded below; the live daemon, D-Bus endpoint and native UI remain pending.
- Later service wire schemas are not frozen by the historical A table. Record their concrete contracts before service implementation; ask only for new significant decisions outside the accepted design.

The approved core supervision refactor is implemented in `ee2139a`; the subsequent [managed dispatch slice](native-managed-dispatch.md) adds owned outgoing cancellation, resource-release acknowledgement, queue-closure health input and startup rollback. The latest full core run passed 288 tests once; an affected 44-method manager check followed the final admission-fairness correction. Owned native inbound consent/receipts for both TCP and BLE are now implemented in [native-session-controls.md](native-session-controls.md); that later verification section supersedes the earlier inventory count here. Live service/state mapping, D-Bus and GTK are not certified by these foundations.

## Primary references checked on 2026-10-12

The offline coordinator is implemented and verified as specified in [native-service-coordinator.md](native-service-coordinator.md): 53 new methods and one complete core run (237 tests), with format/Clippy passing. It covers internal ownership/state only; authenticated D-Bus, the live core adapter and GTK remain pending. Development checks run once under the user's latest instruction; historical ten-run results remain observations, and repeated final stability gates remain separate.

- [KDE Dolphin service menus](https://develop.kde.org/docs/apps/dolphin/service-menus/): registration, local protocol filtering and context actions.
- [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest-single/): separate URI arguments and executable field-code rules.
- [D-Bus specification](https://dbus.freedesktop.org/doc/dbus-specification.html): caller credentials, user-bus activation and Unix descriptor transport.
- [GTK4 migration guide](https://docs.gtk.org/gtk4/migrating-3to4.html): positioning constraints; compositor integration needs separate evidence.
- [Linux open manual](https://www.man7.org/linux/man-pages/man2/open.2.html): acquired object lifetime, access modes and shared open-file descriptions.
- [Linux openat2 manual](https://www.man7.org/linux/man-pages/man2/openat2.2.html): descriptor-relative resolution and confinement restrictions.
- [XDG Document portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Documents.html): handle-based resource acquisition and application permission mediation.


## 2026-10-12 selection adapter checkpoint

The descriptor boundary now has implemented bounded transactional D-Bus ingress: see [native-selection-transactions.md](native-selection-transactions.md) for the authoritative Manager1 five-method fixture, lifecycle, authentication and measured results. Historical path-taking CreateSelection proposals in this document are not an implementation contract. Snapshot/send/control wire APIs, executable activation, retention review and the GTK chooser remain pending; the legacy window is still the current UI.

The subsequent [native-dbus-controls.md](native-dbus-controls.md) checkpoint adds
DiscardSelection, SendSelection, Respond and Cancel without changing those ownership
or trust rules. Nine methods now exist; snapshot/discovery/invalidation wire APIs,
executable activation, retention and the GTK chooser remain pending.

The current [native-dbus-state.md](native-dbus-state.md) checkpoint implements the
user-selected immutable bounded read pages and directed StateChanged hints. There
are now fourteen methods plus one signal. Executable activation, terminal/replay
retention and the native chooser still remain; historical JSON/path API proposals
above are not the current wire contract.

The next [native startup](native-daemon-startup.md) checkpoint adds the Linux
executable, shared native/legacy/developer launch guard, prior settings read and
user-selected Hidden recovery. The old GUI presents explicit visibility repair;
the native headless process exposes GetStartupWarnings. There are fifteen methods
plus StateChanged. Installed activation, retention, native GTK/settings, Dolphin
and physical interoperability remain pending; the legacy GUI cannot attach to
the new daemon while its owner guard is held.
