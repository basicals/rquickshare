# Product direction: native desktop and Google Quick Share parity

## 2026-10-14: private R2 bucket and external signing handoff prepared

User selected Cloudflare R2 and isolated signing outside CI. Dedicated private
rquickshare-native-apt bucket created, Standard/WEUR; observed0 objects, no custom
domain, r2.dev disabled. Existing download.morozhenko.com is active on app-dist
and used by Picturix /dist/Picturix/latest.json; no existing product/storage/DNS or
cache setting was changed. [Domain review](native-r2-domain-review.md) recommends
an independent apt.morozhenko.com binding after collision preflight, alternative
shared-host path routing with a narrowly scoped Worker. Selection remains pending.

[native-r2.md](native-r2.md) records the conditional R2 publisher, canonical public
signing request with independently approved digest, outside-CI signer commands,
scoped public authority, optional pinned SDK and by-hash-only public APT projection.
Immutable pool/by-hash/audit objects precede one conditional no-store InRelease
commit; no deletion/unconditional overwrite/blind retry. The application/daemon
still have no signing key, HTTP update poller or privileged installer.

Measured publication/signer/S3/HTTP APT7/0 in6.916s, affected bootstrap1/0 in0.996s
and isolated file-APT1/0 in0.691s. Actual loopback APT accepted/downloaded by-hash
projection and rejected wrong key. Real test signing subkey verified under pinned
primary. R2 external writes use a conditional fake store and real SDK Stubber,
not production PUT evidence. Syntax/YAML/whitespace passed; optional SDK is in a
CI task venv with contents:read and no signing/publish grant. No remote CI run.

No production key, public object upload/enrollment, transfer/config/clipboard/peer,
branch/push or sibling change. Local stages R1–R4 complete; production hostname,
isolated signer host/hardware/release fingerprint and scoped publish credential
still need concrete binding. Native2 package/unsigned artifact retain prior
provenance/expiry; no Rust or package asset change required repetition/rebuild.
Installed and Google/KDE acceptance remain separate. Earlier production-choice
reviews below are historical where these selections supersede them.

## 2026-10-14: signed native APT staging and trust tooling implemented

User selected1. [native-apt.md](native-apt.md) records tracked native Debian
revision2, immutable standard APT indexes/by-hash/Release, explicit external GPG
signing, isolated pinned public verification, freshness and authenticated previous
release comparison. Signed bootstrap derives actual architectures; it does not
enroll a system. Build CI produces an explicitly unsigned staging artifact and
has no signing secret/publication authority. APT owns installation and network
checks; no native self-updater, HTTP polling or privileged transfer daemon exists.

Measured package Python5/0 in1.244s, repository7/0 in6.753s and affected final clock
boundary1/0 in1.436s passed. Actual isolated APT update/candidate/download matched
package bytes and refused a wrong key; ephemeral test authority was destroyed.
Actual compiled0.11.5+native2 amd64 release package/extracted offline smoke and
unsigned repository stage passed. Package SHA256
5821ad8c85556b603461876b7c6a5484c01048feb2dbf4e741c04058880ec3c7.
No Rust change or needless core/native suite repetition. Syntax/YAML/whitespace
passed; native CI source only, not remotely run. Exact state/provenance in SSOT.

Production HTTPS URI/operator and signing custody are not configured. Review in
[native-apt-publication-review.md](native-apt-publication-review.md): prefer an
isolated signer outside CI; compare protected CI signing as an alternative.
Hosting must retain authenticated immutable objects across metadata promotion.
No production key, trust enrollment, package install, publication/push or peer
operation occurred. These concrete bindings precede an operational public APT
channel. Installed upgrade/removal/KDE lifecycle and protocol/account/QR/Google
parity remain open. Earlier pending update-choice entries below are historical.

## 2026-10-14: opt-in daemon and tray login-start implemented

User selected1. [native-autostart.md](native-autostart.md) freezes the unified
systemd desktop process owner, default-off Start at sign-in, actual OS enablement
and explicit Settings Apply/Reload. Normal app/CLI/daemon-wake activation delegates
to the same unit; a foreground --service worker retains its own guard/signals
across reconnect and waits for actual tray readiness. Login starts daemon and tray
with hidden card. Disabling future startup and GUI Quit preserve daemon transfers.
No JSON duplication, detached child, shell/systemctl or forced custom-unit override.

Persistent unit-file changes are followed by manager configuration reload and
readback. Failed/uncertain outcomes stay unknown; readback can finish pending
configuration sync without repeating enable/disable. Unit definitions remain
vendor-only with no dropins, same-UID/unique-owner checks and bounded owned IO.

Measured full native36/0, then affected login6/0 and final synchronization7/0;
strict native/core lint, package Python5/0 and actual final release Debian/extracted
offline smoke passed. Inventory578 core/38 native is not a newly run full suite.
Final amd64 package SHA256 d2a8211bd2e5ff05771b6381fb3e3a0a04fa770008db66d8e163930bc0f4fc99.
Exact timings, earlier failures/repairs and remaining installed recovery acceptance
are recorded in the autostart SSOT. No user-manager/install/enable/config/clipboard/
portal/peer operation, push or sibling project change. Earlier pending-autostart
entries below are historical. Next distribution choice is prepared in [native-update-review.md](native-update-review.md):
recommend a signed native APT channel with package-manager installation ownership.
Alternative: authenticated native GitHub artifacts with explicit OS-installer handoff.
No updater/repository trust or publication is configured. Protocol parity/account/QR
and physical Google/KDE acceptance remain.

## 2026-10-14: systemd user activation and native Debian package implemented

User selected1. [native-package.md](native-package.md) records the process-based
systemd user unit, fixed fail-closed D-Bus delegation, owned explicit activation
and unchanged passive SDK handshake. GTK launch/Reconnect activates missing service;
Quit does not stop transfers. Installation does not start/enable sharing. Direct
locked release Debian build/verifier and separate manual Ubuntu26.04-container
artifact workflow are implemented; existing Tauri release jobs remain baseline.

Actual release package/extracted offline smoke, service262 plus additional real
cold-broker1, Python package5, strict core/native lint and affected GTK1 passed.
The private broker reads the shipped activation asset and requests the fixed unit;
its external manager is fake, so installed systemd/session acceptance stays open.
Inventory576 core/29 native is not a newly run full suite. Small root-cause repair
prevents build.rs rewriting/invalidation on every invocation; final release cache
checks0.18/0.15s preserved the generated index mtime. Exact failed fixtures, repairs,
warnings and timings are in the SSOT. Direct dev, no host install/enable/live user
manager/config/clipboard/peer, push/branch or NativePair change.

Next [native-autostart-review.md](native-autostart-review.md) recommends1: explicit
default-off Settings login-start launches daemon plus hidden-card tray client;
alternative2 starts only daemon with lazy GUI. Selection is needed for that product
presentation and larger GUI ownership/readiness refactor. Legacy default-off was
verified in source. Updater policy, installed package lifecycle, protocol/account/
QR and real Google/KDE matrices remain. Earlier pending-package entries below are
historical; full Google parity remains the goal, not an achieved claim.

## 2026-10-14: installed daemon ownership review prepared

[native-package-review.md](native-package-review.md) audits current source assets,
Tauri-only package workflows and NativePair's direct Debian build model. NativePair
has no installed daemon activation contract to copy. Recommend1: systemd user
process ownership with fixed D-Bus activation; alternative2 is direct session
D-Bus daemon startup. This significant installed lifecycle choice awaits selection.
The current daemon releases/reacquires its name during joined settings restart;
a conventional Type=dbus unit would conflict with that behavior. Preserve process
lifetime and existing readiness/epoch contracts rather than copy an unsafe unit.
No production package/service/autostart change or live manager operation in this review.

## 2026-10-14: transfer-bound native Show in folder implemented

User selected option1; [native-reveal.md](native-reveal.md) completes bounded actual
EOF receipts, original receive-directory authority, Files1 metadata/read-only FD
acquisition and explicit per-file portal OpenDirectory controls. No caller path,
current-setting reconstruction, default application execution or automatic launch.
Receipt quotas follow actual Arc ownership, history holds no completed-file FD,
and stale/moved/changed targets fail closed. A slow file open no longer blocks the
daemon event loop; one tracked filesystem worker is joined on close.

Actual full core570+8 integrations passed before the responsiveness repair;
afterward affected service258, receipt3, strict core lint and native rebuild passed.
Native observed27 methods pass in the initial interrupted run; repaired portal3,
strict lint/build passed afterward. Inventory571 core/29 native is not a fresh
whole-suite claim. Exact timings, failed fixture repairs and remaining installed
KDE selection/focus/activation gates are in the SSOT. Direct dev only; no live user
portal/config/clipboard, installation, peer, push or branch operation.

Remaining: native autostart/update/package lifecycle; protocol/name/temporary
visibility/trust/previews/resume/streams/multi-recipient/Wi-Fi/apps, Google account/QR
and actual Android/Windows/KDE matrices. Tauri remains the packaged migration
baseline. Full Google compatibility is the target, not an achieved claim. Earlier
review/pending entries below are historical.

## 2026-10-14: native clipboard images implemented; received-file reveal reviewed

User selected anonymous immutable source1/A. [native-image.md](native-image.md)
records completed source/accounting/PNG/worker/GTK stages: encoded PNG32MiB,
64million pixels, preview512×512, daemon anonymous Files128MiB aggregate retained
through actual readers. Capture and Prepare are explicit; recipient Send is separate.
The shared file controller/queue/recovery handles images without named private files
or caller deletion paths. Lock/epoch/close clears local image review and joins owned
CPU work. Mutable seal-capable tmpfs source descriptors are deliberately rejected;
ordinary disk descriptors retain existing behavior. Tray image ID17 preserves prior IDs.

Actual core whole suite558 plus8 binary integrations passed; additional anonymous
broker scenario and strengthened PNG3 passed. Native25 passed and final affected
GTK1/lint/build passed. Inventory became559 core library methods; exact timings,
limitations and failed/corrected fixtures are in SSOT, without a fresh559 whole-run,
ten-run stability or installed Google/KDE claim. Work stays directly on dev.

Next [native-reveal-review.md](native-reveal-review.md) recommends actual completed
transfer receipts and descriptor-based explicit Show in folder over opening only
the current receive folder. New API/security/UX boundary awaits selection. The
independent premature file EOF defect found during analysis is repaired under the
maintenance preference in [inbound-file-completion.md](inbound-file-completion.md).
Autostart/update/package lifecycle, protocol/name/temporary visibility/trust/
previews/resume/streams/multi-recipient/Wi-Fi/apps, Google account/QR and physical
Android/Windows/KDE matrices remain. Historical pending-image entries below are dated.

## 2026-10-13: clipboard image storage review prepared

Native outgoing text O1-O4 is complete. [native-image-review.md](native-image-review.md)
compares anonymous sealed memory-backed source(A,recommended) with private disk
spool(B). A real local kernel primitive probe passed without creating a named file;
all descriptors closed. Image capture/PNG validation/global memory-backed source
accounting are not implemented by that probe. Await explicit storage-boundary
selection before image production code under the user's significant-change rule.
No live clipboard/config/install or peer operation. All prior text checks remain
measured observations, not a newly rerun suite. Work stays on dev.

## 2026-10-13: native outgoing text and bounded clipboard capture implemented

[native-outbound.md](native-outbound.md) completes O1-O4 under selected A: unified
Files/Text source retained through actual request lifetime, one quota/selection/queue,
chunked typed text preparation, exact ordered recovery and safe metadata summaries.
Existing files/CLI and text preserve each other's source slot. Source is not Send.
GTK Send text in header/context/tray opens a private editor; only explicit paste
reads a bounded UTF-8 stream. Prepare leads to recipient choice; Send is separate.
Lock/epoch/close cancels capture and clears editor; reconnect never recaptures text.

Checks include actual native+legacy two-manager text transfer, domain byte/quota/
replay tables, private-broker interrupted Begin/Append/Commit recovery and preserved
file lifetime checks. Native22 passed before summary addition; affected panel7/menu2
then passed. Strict lint/build and8 binary integrations passed. Exact failures,
corrections/counts and source rollback are in SSOT; no whole-product/Google claim.
Next: clipboard images, received reveal/open, autostart/updates/package lifecycle,
then outstanding protocol/account/QR and physical Android/Windows/KDE gates.
Earlier reviews and dated pending-text entries below are historical.

## 2026-10-13: outgoing native source ownership review prepared

The user requested continuation beyond received text. Core already sends text, but
native Selection/Reservation/SendDispatch only own opened files. Source inspection
confirms that outgoing clipboard needs a typed ownership/refactor/API decision,
not a direct UI-to-core shortcut. [native-outbound-review.md](native-outbound-review.md)
recommends A: one prepared Files/Text source with shared queue, quota, receipts,
recovery and recipient flow. B keeps separate text preparation/dispatch lifecycles.
Native64KiB wire bounds require staged text bytes. No new production code precedes
selection; substantial file-source refactor must be explicit. Remaining priority
and prospective milestones are recorded in the review. Work stays on dev.

## 2026-10-13: received native text and explicit copy implemented

User selected 1/A. [native-text.md](native-text.md) records R1-R4: bounded native
Text/URL consent metadata, sender/PIN/size/type validation, private immutable
Finished resource, captured Text1 availability, authenticated per-client chunked
read/retry/release, serialized controller and secondary plain review with explicit
Copy. Receipt/reconnect never changes clipboard or opens URLs. Known unlock and
current epoch/resource are checked again on Copy; lock/epoch/unavailability closes
the private surface. GTK acknowledges provider ownership, never persistence.

Actual core suite passed 543 methods +8 binary integrations before the final
admission fixture; that affected fixture passed separately. Native20 passed/1 failed
identified unintended PRIMARY focus selection, fixed at its root; affected GTK
and readiness checks passed afterward. Strict core/native lint and native build
passed. Scope inventory/checkpoint details are in the SSOT; no ten-run/coverage or
physical-device claim. Work is committed on dev; no live config/clipboard/install,
new branch, push or Tauri retirement. Original upstream refs remain.

Next native gaps: outgoing explicit clipboard/text/image selection, reveal/open,
autostart/updates and installed package equivalence. Received-text support does not
establish full Google parity: account/QR/peer capability and Android/Windows/KDE
physical matrices remain open. Earlier dated entries are historical checkpoints.

## 2026-10-13: separate tray menu completed

The accepted right-click actions now have a standard DBusMenu at /Menu on the
owned StatusNotifierItem connection. [native-tray-menu.md](native-tray-menu.md)
records T1-T3: fixed privacy-free rows, actual same-UID/no-FD validation, bounded
ordered eight-batch action mailbox, truthful grouped-event failure IDs, mode/enabled
projection and immediate lock/unlock readiness. GTK's existing checked actions
remain the sole UI execution owner. Quit closes the panel, never the transfer daemon.

Primary activation opens the card; a direct ContextMenu request opens its existing
popover. KDE may import the exported menu into its own native renderer. Physical
Plasma placement/focus/mixed-scale/Wayland behavior remains unverified. All 20 native
methods passed, followed only by affected checks; final strict native lint/build
passed. Core/settings checks remain their preceding measured observations.

Settings and separate tray menu source are complete. Next native capabilities are
clipboard/text/reveal, autostart/updates and installed package equivalence. Physical
Google parity/account/QR/peer matrices remain open. No live installation or push.
Prior checkpoints below describe their dated state, including pending-menu entries.

## 2026-10-13: daemon-owned native settings implemented

User selected 1/A. [native-settings.md](native-settings.md) is the implementation
SSOT: owner-local per-transfer receive authority, private transactional schema-1
store, exact source/revision conflict checks, retained original legacy file/private
backup, typed authenticated Settings1, joined nonblocking writer, truthful saved/
applied/restart status, configuration-only recovery, idle session cutoff/restart,
and secondary GTK editor with persistent Full/Compact. No manual JSON repair is
required for readable bounded corrupt settings; inaccessible/oversized storage is
never discarded. Missing backend keeps appearance explicitly temporary.

Verification includes the full core command (532 unit/export + 8 integration),
18 native methods followed only by affected checks, strict core/native lint and
native build. The compiled inert settings image is reviewed. These are automated
source checks, not installed migration, compositor or Google peer acceptance.
No user settings access, live registration/install, push or Tauri retirement.

Remaining native migration: clipboard/text/reveal, separate tray dropdown,
autostart/updates and installed package lifecycle. Physical KDE lock/DND/focus/
placement/scale and Google interoperability/parity remain open. Do not claim full
native feature parity from this settings milestone. Work stays on dev; upstream
references are retained. Earlier checkpoints below describe historical state.

## 2026-10-13: native settings ownership review prepared

The next native gap is persistent settings and in-app visibility recovery. GTK
currently retains only Appearance mode in process; the daemon reads legacy settings
but offers no native write/reload contract. The accepted recovery UX still forbids
manual JSON repair. [native-settings-review.md](native-settings-review.md) maps
actual owners and proposes 1/A (recommended): the existing daemon is the sole writer
of a versioned native store with typed edits/acknowledgement. Alternative 2/B is an
independent per-user settings service. This is a new persistence/API decision;
notification option A did not approve it. Await selection before settings code.

The review also identifies process-global CUSTOM_DOWNLOAD as incompatible with
owner-local settings and pinned receive authority; its substantial refactor is
included in the proposal, not silently implemented. The narrow independent privacy
fix is complete: both download setters now log fixed categories instead of paths.
Strict core library Clippy (4.61s), core/Tauri format and diff checks passed once.
No fresh suite/runtime/device claim, user settings access, migration or install.
Continue on dev. Earlier checkpoints below are historical.

## 2026-10-13: daemon-owned custom-card delivery implemented

The user accepted A/option 1, the daemon notice read model. All source milestones
N1-N5 in [native-notification-backend.md](native-notification-backend.md) are complete:
epoch-local bounded notices, immutable Notifications1 reads, exact dismissal,
canonical GTK presentation, keyed rows, fixed package GUI activation, finite CLI
error codes, unavailable backend and explicit joined reconnect. No implicit Send,
consent, source replay or system-notification takeover. Dismissal cannot affect a
newer round. Same-generation failure/Quit does not create a respawn loop.

Actual verification: 219 core service methods passed, then two affected wake methods
passed after the native-future lint fix/additional blocked-bus integration; 15 native
methods passed, followed by three affected policy/widget methods. Final strict
core/native Clippy, both builds, format/diff and XML/activation-asset checks passed.
Ten new methods by primary purpose: N2=1, N3=3, N4=1, N5=3, N7=2. Combined inventory
535 is not a fresh whole-suite/coverage claim. Six compiled inert previews were
regenerated; physical KDE/Google acceptance is not implied.

No install/push/live registration, legacy settings write or Tauri retirement.
Remaining: installed package startup/upgrade/removal and KDE lock/DND/focus/scale;
durable native settings, separate tray dropdown, clipboard/reveal/autostart/updates
and actual Google parity. Registry survives GTK restart only within its daemon epoch.
Earlier dated checkpoints and pending choices below are historical. Continue on dev.

## 2026-10-13: custom Full/Compact card and desktop safety implemented

The user selected fully custom rQuickShare cards with Full/Compact modes and
secondary settings/right-click actions. C1/C2 in
[native-custom-card.md](native-custom-card.md) are implemented in commits
72ca702 and f09671e. Compact is the initial mode; Full retains recipient choice,
incoming review and transfers. Header/card right-click menus share checked actions;
Appearance settings retain the mode for the current GTK process only. Tray
ContextMenu currently opens the panel; a separate tray dropdown is not implemented.

While GTK runs, bounded presentation policy observes current daemon requests and
new completion/error transitions. Read-only KDE lock/DND monitors pin same-UID
owners and reject stale replies. Unknown lock hides sensitive surfaces; automatic
cards also require known DND-off. Dismissal never answers consent or cancels work.

Observed: all 13 native methods passed once, followed only by checks affected by
later fixes; final strict native Clippy/build/format/diff passed. Four new methods
primarily cover N5 (2), N3 (1), N7 (1), with secondary edge/privacy tables. Current
core/native inventory is 525, not a fresh whole-suite run or coverage measurement.
Six compiled inert previews show both modes; they do not prove KDE compositor,
lock/focus/scale behavior or interoperability. No live installation, push, daemon/
transport/core API change, legacy settings write or Tauri retirement in this slice.

C3 remains pending: closed-GUI delivery, cross-GTK notice recovery/dismissal and
visible backend-unavailable Dolphin errors. The next significant decision is
[native-notification-backend-review.md](native-notification-backend-review.md):
recommend A, a bounded daemon notice read model coherent with captured transfer
state; B uses a supervised renderer and private acknowledged IPC. No C3 production
code precedes this new API/data-model/process decision. Keyed GTK row updates are
also required before final focus/expansion acceptance. Durable settings,
clipboard/reveal/autostart/updates/packaging and actual Google parity remain open.
Earlier dated checkpoints below are historical, including their pending choices.

## 2026-10-13: GUI-owned CLI import and Dolphin source assets

The user delegated ingress ownership selection to architectural quality. Option 1
is implemented: CLI acquires read-only descriptors; authenticated bounded Desktop1
imports into the panel's existing SDK connection. Interrupted replies replay the
same import; committed preparation survives CLI disconnect. Invocation never sends.
The chooser and CLI share one preparation gate, preserving an existing selection.
CLI cold start is driven by broker ownership/child exit. No implicit daemon startup.

[native-desktop-ingress.md](native-desktop-ingress.md) records invariants, rollback
and actual verification: 43 affected core client methods passed; nine native methods
passed, followed by one affected startup-fixture rerun; strict lint/build/format
passed. Combined test inventory is 521, not a whole-suite execution. Source-only
Dolphin metadata has static/KConfig validation and an explicit package lifecycle.
No live install/registration, push, settings changes or Tauri retirement.

Native notifications and visible nonterminal CLI errors are the next release gate;
[native-notification-review.md](native-notification-review.md) records the pending
incoming-action UX choice. Settings, clipboard/reveal/autostart/updates/packaging,
physical KDE placement and actual Google peer parity remain unfinished. Historical
checkpoints below describe their dated state, not current authorization.


## 2026-10-12: native GTK shell implementation checkpoint

[native-gtk-shell.md](native-gtk-shell.md) now records a separate native executable
with real PanelController binding, local capability acquisition, explicit
recipient/send/consent controls and KDE tray registration/placement logic. Hide
keeps the client; Quit joins local tasks and leaves accepted daemon transfers.
Seven new scoped methods and fourteen affected SDK checks passed, plus strict
lint; three isolated compiled previews were reviewed. No real KDE placement proof
or full-product acceptance. No installation or Tauri retirement.

The next public-contract choice is
[context-menu ingress ownership](native-desktop-ingress-review.md); accepted
connection-owned selections cannot simply be passed as a token from CLI to GTK.
Notifications, settings migration/editor, clipboard/reveal/autostart/updates,
Dolphin/package integration and physical Google parity remain required. Work stays
in dev. Historical checkpoints below do not supersede this current state.

## 2026-10-12: complete native Manager2 view implemented

The user selected option 2 and confirmed there are no old native clients. Manager2
replaces unpublished Manager1 in daemon, client and XML; no compatibility adapter.
[native-complete-view.md](native-complete-view.md) is authoritative for ordered
sender/files/PIN/progress, immutable manifests, complete bounded captures and
safe native acceptance. Prior pending-choice/Manager1 summaries below are historical.

Measured full run: 488 unit/export + 8 integration passed; final round-reset
change passed 32 affected session tests. Current inventory 497, not a fresh whole-
497 run. Strict core/desktop Clippy, format and diff checks passed. No installation,
push or user-session changes. Continue native panel/controller integration; GTK
clock placement, notifications, settings/Dolphin/package and physical Google parity
remain unfinished. Terminal closed.

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

[Native GTK preview and transfer detail review](native-desktop-details-review.md)
now provide real widget captures of the compact panel. The GUI is still a review
prototype. Live consent requires the next bounded sender/file/PIN/progress contract;
no details are fabricated and no acceptance controls are enabled in the preview.

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
