# Native desktop migration and context-menu sender

_2026-10-12. Mode A. Status: concrete proposal for API/security review; no native implementation yet._

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

### Decision needed: outbound file-selection contract

| Option | Description | Pros | Cons / risks |
|---|---|---|---|
| A — same-user local paths first (recommended) | Thin clients submit selected local file/folder paths; the same-UID daemon validates and opens them using existing core preparation | Preserves current core API and folder behavior; isolates desktop migration from protocol/file-reader refactoring | Same-user callers can request any daemon-readable path; names can change between selection and open; not sufficient as a sandbox boundary |
| B — descriptor/capability input first | Clients open selected regular files and submit handles plus bounded relative-name manifests; folders require a defined enumeration/handle contract | Pins opened objects across rename/replacement and supports a narrower selected-object model | Requires core input/ownership refactor now, directory semantics, descriptor-pressure handling and independent compatibility tests; FDs alone neither authenticate user intent nor freeze file contents |

Recommendation: A for the unsandboxed native desktop migration, with explicit same-user trust and no elevation. It preserves the validated transfer path and keeps future descriptor support versionable. B is appropriate if sandbox-mediated file access is required before the first native build. Both choices require existing symlink/path-traversal protections and structured errors; A does not promise snapshot semantics. The recommendation is pending approval.

### Proposed public contract for option A

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
| D1 — offline service foundation after approval | Rust service/client crates, fixed API schema/introspection, bounded command/state coordinator and injected core adapter; record branch/negative/interaction inventory before tests | Actual coordinator tests for replay, reconnect, stale consent, terminal cancellation, queue limits, disconnect and shutdown; private bus integration | No GUI/BlueZ in unit tests; retain old app and no live registration |
| D2 — existing-core adapter | One transport owner, supervised startup/shutdown, current settings read without migration, secure path selection; document old/new mapping | Existing suites plus adapter lifecycle/selection tests and ten consecutive runs on final slice | Protocol framing/crypto unchanged; mutually exclusive old/new launch mode |
| D3 — native tray panel | GTK4 client, native notifications and proven KDE/Wayland activation/placement; review prototype before material layout/fallback decisions | Offline state projections, reconnect/notification actions, keyboard tests; physical KDE checks later | No claimed tray anchoring before evidence; isolated desktop code |
| D4 — context action | CLI activation, shared recipient flow, Dolphin service-menu package assets | CLI URI/argument boundaries, cancellation, multi-selection/folders, missing files, package install/uninstall; physical file-manager smoke later | No shell command interpolation; originals never deleted; remove adapter package assets to roll back |
| D5 — desktop completion | Clipboard, settings backup/migration, autostart, reveal/open, updates and package equivalence | Old/new capability checklist, full suites, final hardware gates | Retire Tauri only after equivalence and migration acceptance |

The native migration does not complete Google parity. The full feature/peer inventory and protocol/account/QR/Apple work remain separate required milestones in [product-direction.md](product-direction.md). Existing maintenance stability results do not validate unimplemented native code.

## Progress / handoff

- D0 analysis and proposal: complete.
- User interaction decision: option A accepted; oversized drop zone rejected; context-menu sender requested.
- Remaining STOP FACTOR: approve option A or B for the file-selection/security boundary and associated service proposal before D1. No native code, desktop registration, settings change or process activation has been performed.
- After approval: record it here first, freeze D1 schema/limits and test inventory, then implement the bounded offline foundation. Document later findings and completed stages in this file.

## Primary references checked on 2026-10-12

- [KDE Dolphin service menus](https://develop.kde.org/docs/apps/dolphin/service-menus/): registration, local protocol filtering and context actions.
- [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest-single/): separate URI arguments and executable field-code rules.
- [D-Bus specification](https://dbus.freedesktop.org/doc/dbus-specification.html): caller credentials, user-bus activation and Unix descriptor transport.
- [GTK4 migration guide](https://docs.gtk.org/gtk4/migrating-3to4.html): positioning constraints; compositor integration needs separate evidence.
