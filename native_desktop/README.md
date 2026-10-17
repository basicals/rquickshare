# Native development shell

Current implementation, selected distribution/signing bindings and remaining gates:
[current state](../docs/current-state.md). Dated checkpoints below retain historical observations.

The installed native28 uses one Receive, Send, Devices, Hide toolbar. Devices
opens the upper device list; incoming consent is immediately inline as type/code
and Decline/Accept, followed by byte progress and a transient Completed status.
Received files are automatically revealed safely; text opens a plain Copy/Close
review. Manual result actions live in the separate Transfers window. There is no
row disclosure or additional Full header. Main Send opens content types first;
choosing a global source then opens devices. Per-device Send instead offers an
inline File/Folder/Text/Clipboard strip and sends to that exact endpoint after input.

Prepared sources are retained in the daemon. Each device-row Send joins its FIFO
in click order; clearing preparation preserves admitted payloads. Clipboard chooses
local file/folder lists before image before text, without fallback after a chosen
format fails. File and text limits and explicit receiving consent still apply.
The card's lower controls are covered while a global source awaits target choices.

Primary tray clicks toggle the card and preserve valid same-session position.
Background movement and cursor feedback share one hit rule, excluding controls.
A free card hides after15 seconds without input while the pointer is outside;
requests/transfers, sources, dialogs/menus and recovery states pause dismissal.
Daemon work continues after hiding. See [native28 contract](../docs/native28-history-idle-movement-2026-10-17.md)
for exact measured checks and the user's general UI acceptance.

Receive reviews a waiting request or requests one-minute visibility. Pressing it
again during temporary visibility requests Hidden. The collapsed button pulses;
the open drawer uses a spinner/countdown. Visibility2 supplies the checked core
snapshot and SDK monotonic sample; the daemon alone owns expiry. No periodic IPC
or automatic acceptance is added by the display timer.

The tray offers card toggle, Receive, Send, Transfers, live visibility controls,
Settings and Quit. Transfers uses aligned live/metadata rows and footer controls;
completed metadata is retained up to30 days/1000 records without payload contents.
Old persisted metadata does not grant received-file access or device trust.

Settings retains branded General, Receiving, Advanced and About sections and
staged Cancel/OK edits. Legacy card_mode remains preserved in storage for contract
compatibility, while drawer expansion is local and initially closed. Exact
source/package/installed state is in [current state](../docs/current-state.md).

Settings OK saves validated native values, then requested login start. It closes
only after acknowledged application; failures/uncertainty/restart-required state
remain explicit. Cancel discards an unsubmitted draft, not admitted writes.
The daemon is the sole native-settings writer; the original Tauri settings remain
intact with exact private backup on import. Recovery actions are contextual in
Advanced. See [settings](../docs/native-settings.md) and the superseding mockup
contract. Physical KDE dragging/placement and real-peer checks remain separate.

Corrupt configuration or failed transport startup keeps a Stopped settings service
available for in-app repair. Save is distinct from guarded start/restart; prepared
files/live transfers block restart. Reconnect reads the new generation without
replaying source/send/save actions. Missing daemon or read-only `--settings PATH`
override disables persistence. A build does not install or enable login startup; normal native startup uses
the separate native store when present.

The daemon owns per-device admission, queue and independent results even after
the card closes. The older checkbox/multi-selection UI has been replaced by the
per-click contract in [managed dispatch](../docs/native-managed-dispatch.md) and
the [native25 interaction ledger](../docs/native25-interaction-polish-2026-10-17.md).
Current ordinary interoperability and failure/lifecycle acceptance are tracked in
[essential acceptance](../docs/native-essential-acceptance.md). The installed
source checkpoint is not a claim of full Google Quick Share feature parity.

Build from the repository root with Rust >=1.93, GTK4 >=4.12 development files,
GTK4 Layer Shell development/runtime, fontconfig/libseccomp development files,
Bubblewrap, Glycin loaders >=2.1, pkg-config and the existing core build
dependencies. This separate crate keeps GTK4 isolated from Tauri's GTK3 stack.

```sh
cargo +stable build --locked --manifest-path native_desktop/Cargo.toml
```

The private `rquickshare-preview-helper` binary is installed under `/usr/libexec`.
It runs Glycin with explicit Bwrap and hard per-process resource limits. The GUI
only receives bounded validated RGBA, with no unsandboxed fallback. Existing
`--preview --preview-page incoming --preview-mode full` exercises a synthetic
PNG through the real helper; Compact keeps its four quick actions. This local fixture
does not establish remote preview wire semantics or Google interoperability.
See [preview contract](../docs/native-preview-review.md). Tests require building
the helper first; real sandbox smoke must fail if the environment cannot enforce
isolation, never silently decode in-process. Native7 remains an older artifact.

Normal launch requires a supported Layer Shell compositor and same-user KDE status
notifier watcher. Normal launch delegates to the fixed systemd desktop user service
and explicitly shows its existing card; the worker activates the daemon when needed.
A development checkout without installed assets uses `rquickshare --service` and
a separately started daemon. Builds never install/enable services. The worker waits
for actual tray readiness and joins SIGINT/SIGTERM cleanup across reconnect.
Follow [native-package.md](../docs/native-package.md) and
[native-daemon-startup.md](../docs/native-daemon-startup.md); do not start it beside
an existing transport owner. Passive SDK connections and preview never activate it.
Missing graphical/tray prerequisites fail explicitly. A missing/incompatible daemon
enters a visible, non-preview unavailable state with a contextual Retry connection action. The panel
stays hidden until tray activation, a CLI import or a safe pending daemon notice. Escape/close hides it; Quit closes local clients, not daemon work.

To review the compiled widget source without connecting a daemon or tray:

```sh
GTK_A11Y=none GDK_BACKEND=x11 GSK_RENDERER=cairo \
dbus-run-session --config-file docs/prototypes/preview-bus.conf -- \
xvfb-run -a native_desktop/target/debug/rquickshare \
  --preview-output /absolute/path/panel.png --preview-page incoming
```

Pages: nearby, incoming, transfers. Preview shows explicitly marked examples;
controls are disabled and it cannot acquire/send/accept. Screenshot completion
waits for an allocated render node, without sleeps. It does not prove Wayland
placement. An interactive ordinary window is available only via explicit --preview.

Run the scoped native methods, including the single GTK test, on an isolated display:

```sh
GTK_A11Y=none GDK_BACKEND=x11 GSK_RENDERER=cairo \
dbus-run-session --config-file docs/prototypes/preview-bus.conf -- \
xvfb-run -a cargo +stable test --locked --manifest-path native_desktop/Cargo.toml
```

Tray tests create and close their own private brokers with no activation directories.
No check registers an item with the user's real tray. Build/runtime private
dependency extraction used during development is recorded in the SSOT; there
are no environment-specific search paths committed in this crate.

The headless sender acquires local paths/folders or local file URIs and imports
read-only descriptors into the existing panel:

```sh
native_desktop/target/debug/rquickshare send -- /local/file 'file:///local/a%20b'
```

Invocation prepares the complete selection and presents recipient selection; it
never sends automatically. A missing GUI is activated through its fixed desktop
user unit, without paths; that worker activates the daemon. There is no child-launch
fallback. The panel's own
chooser and CLI share one preparation owner; an existing selection is preserved.
CLI failures preserve fixed stderr diagnostics and route a finite error code to the
same GUI, without paths or arbitrary text. Error display works without a transfer
backend and never retries source acquisition, preparation or Send.
The public ingress contract and measured SDK checks are in
[native-desktop-ingress.md](../docs/native-desktop-ingress.md).

Historical pre-native16 card presentation started in Compact mode. Full kept Nearby, Incoming
and Transfers, with source controls on Nearby. The tray and right-click
menus expose secondary actions; Settings persists Appearance and supported service
values through the daemon's typed contract. No legacy persistent setting is overwritten. Full/Compact share
checked consent/cancel commands and preserve the prepared source owner.

Use `--preview-mode compact` or `--preview-mode full` with the explicit preview
options to review both surfaces. Automatic desktop-safety-aware notification
presentation while GTK runs is implemented in
[native-custom-card.md](../docs/native-custom-card.md).

Normal mode observes the actual KDE screen-lock and effective DND providers using
owned read-only session-bus connections. Unknown lock hides sensitive surfaces;
automatic cards also require known DND-off. Queued manual CLI/tray presentation
waits for verified unlock. No system notification service is replaced. Card
hide/dismissal does not answer consent or cancel a transfer. Only canonical undismissed daemon notices create popups, including pending outcomes
after GTK restart; ordinary retained history does not. One bounded policy presents
current events while preserving safety and explicit user actions.

Canonical daemon notices now survive GTK close/restart within the daemon epoch.
Exact presentation dismissal does not answer consent or dismiss a newer round.
Daemon wake uses a fixed package-owned D-Bus activation asset and a single owned
worker; same-generation failure/Quit never triggers an immediate respawn loop.
Uninstalled development without that asset retains notices for manual GUI startup.
Reconnect closes/joins the old GUI/client lifecycle before creating a fresh client,
retains display mode and never replays unsent preparation or uncertain sends.
Keyed rows retain expanded manifests through progress updates. The contract and
actual checks are in [native-notification-backend.md](../docs/native-notification-backend.md).
No physical DND/lock/focus/monitor or installed-product acceptance is claimed.

Received Text/URL shows advertised sender, verification code and byte count before
Accept. A retained completed incoming text offers Review text in its expanded list row.
The secondary window displays plain nonselectable text; only Copy replaces this
clipboard, after fresh unlock/epoch/resource checks and GTK provider acknowledgement.
Receipt/view/reconnect never copies or opens links. Lock/epoch change closes review;
closing review preserves an explicit prior copy. Daemon text is bounded and epoch-local,
not a permanent journal. Read recovery uses Recover operation; pending reads block
new source/settings mutations. Outgoing preparation and URL opening
remain separate capabilities. See [native-text.md](../docs/native-text.md) for contracts
and exact actual observations; no installed Wayland/Google acceptance is implied.

Text or link… in the sending surface opens a private editor. Write text or
explicitly Paste clipboard (1–128 KiB UTF-8, no NUL), review and Prepare text. Then
choose a Nearby recipient and press Send. Preparation alone never sends. Existing
prepared files/text must be discarded first. Clipboard capture is bounded, cancellable
and does not run when opening the panel. Lock/epoch/close clears the private editor.
Uncertain preparation uses the existing Recover/Discard controls without recapturing
clipboard. Full/Compact source summaries show generic kind/size. See
[native-outbound.md](../docs/native-outbound.md).

Send image… opens a separate review window. Capture clipboard image reads PNG only
on that explicit action, validates a static image up to32 MiB/64 million pixels,
and displays a preview up to512×512. Prepare image uses the same Files recipient
flow; Send remains separate. Anonymous sealed data is immutable and has no temporary
file to clean up. Unsupported/rejected recapture preserves the previous snapshot.
Lock/epoch/close clears the local image; owned CPU work retains the preparation
permit until completion. See [native-image.md](../docs/native-image.md).

Memory-backed file descriptors must carry all immutable seals and fit32 MiB each/
128 MiB aggregate in the daemon. Named tmpfs files follow ordinary mutable-file rules; unsealed anonymous
memory is rejected. Ordinary disk sources retain descriptor behavior. For automated disk-source tests
on a host with tmpfs /tmp, set TMPDIR to a directory on a disk filesystem first.

The native Debian builder is `python3 scripts/native_deb.py build` from the repository
root. It uses locked release binaries, derives actual ELF library dependencies,
stages fixed systemd/D-Bus/application/Dolphin assets and verifies the extracted
artifact offline. `--profile dev` labels a development package explicitly. No package
is installed or service enabled by build/verification. See
[packaging/README.md](packaging/README.md) for requirements and remaining acceptance.


Completed incoming files now offer Show in folder per manifest entry in its expanded list row. This uses the original completed transfer's collision-resolved result,
not the current receive setting. The daemon retains bounded private receipts and
opens a short-lived verified read-only descriptor on demand; the native client
passes that descriptor to portal OpenDirectory after the explicit click. Rendering,
receive completion and reconnect do not launch anything. Missing/moved/changed
files and unavailable portals show a finite error; no default application executes
the received content. Cancel folder request, hide/lock/epoch change and shutdown
close owned pending work. Already accepted external actions cannot be retracted.
See [native-reveal.md](../docs/native-reveal.md) for exact limits, ownership, actual
checks and physical KDE selection/activation acceptance still required.

Settings > General > Start at sign-in > OK enables the vendor desktop user
unit for future sessions: daemon and tray start with the card hidden. Default off.
Disabling future startup leaves current services/transfers running. The setting
reads actual systemd enablement and requires explicit Apply; Reload resolves
unconfirmed changes. Custom/masked/unknown units remain unavailable. See
[native-autostart.md](../docs/native-autostart.md) for ownership, constraints and
measured checks; a real installed login/logout remains acceptance.

Native updates use the selected signed APT channel design, with local immutable
prepare/sign/verify/bootstrap tooling and tracked Debian revision. Installation
and network update checks belong to the system package manager, not this GUI or
transfer daemon. CI emits unsigned staging only. Production HTTPS URI/signing
custody and installed update lifecycle remain pending; see
[native-apt.md](../docs/native-apt.md) and
[publication review](../docs/native-apt-publication-review.md).

R2 storage and outside-CI isolated signing are selected. A dedicated private bucket
and checked conditional publisher/public approval handoff are implemented in
[native-r2.md](../docs/native-r2.md). Existing download.morozhenko.com serves
Picturix; domain routing remains a separate choice before public binding. No
production key or publicly trusted channel is provisioned by these builds.


Incoming Linux files are now staged as unnamed inodes on the destination
filesystem. A final name is linked without overwrite only after exact completion
and data sync. On failure/cancellation, incomplete files disappear and completed
members remain; the card lists Saved N of M with completed-file-only Show in folder.
This requires O_TMPFILE and safe FD publication on the selected filesystem;
unsupported storage fails preparation before the peer receives Accept. There is
no unsafe final-name streaming fallback. Empty newly created folder hierarchy
may remain after failure. See [receive reliability](../docs/native-receive-reliability.md).
