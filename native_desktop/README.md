# Native development shell

The tray now exports a standard DBusMenu with Open, file/folder/text/image selection, incoming/
transfer navigation, Compact/Full, Settings, Reconnect, Hide and Quit. It contains
fixed labels only. Actions share current GTK readiness and lock safety; no consent,
implicit Send, saved-setting write or daemon-stop action is exposed. Primary click
opens the card; direct ContextMenu opens its existing popover. See
[native-tray-menu.md](../docs/native-tray-menu.md) for protocol/lifecycle and actual
checks. Physical Plasma menu rendering and placement/focus remain acceptance gates.

Settings now supports explicit Save/import, source conflict recovery, retry,
visibility, optional port, locally acquired receive-directory FD, and persistent
Full/Compact. The daemon is sole writer of a private native file; the original
Tauri file remains intact with an exact private backup on import. See
[native-settings.md](../docs/native-settings.md) for the authoritative contract.
Mode menu changes preview temporarily until explicitly saved in Settings.

Corrupt configuration or failed transport startup keeps a Stopped settings service
available for in-app repair. Save is distinct from guarded start/restart; prepared
files/live transfers block restart. Reconnect reads the new generation without
replaying source/send/save actions. Missing daemon or read-only `--settings PATH`
override disables persistence and labels appearance temporary. No install/autostart
is implied; normal native startup uses the separate native store when present.

An uninstalled GTK4 tray client for the native daemon. This does not retire the
Tauri application or establish Google Quick Share parity. The current plan,
invariants, exact measured checks and pending acceptance are in
[native-gtk-shell.md](../docs/native-gtk-shell.md).

Build from the repository root with Rust >=1.92, GTK4 >=4.12 development files,
GTK4 Layer Shell development/runtime, pkg-config and the existing core build
dependencies. This separate crate keeps GTK4 isolated from Tauri's GTK3 stack.

```sh
cargo +stable build --locked --manifest-path native_desktop/Cargo.toml
```

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
enters a visible, non-preview unavailable state with explicit Reconnect. The panel
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

Custom card presentation now starts in Compact mode. Full keeps Nearby, Incoming
and Transfers, with source controls on Nearby. The header menu and right-click
menu expose secondary actions; Settings persists Appearance and supported service
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
Accept. In Full > Transfers, a retained completed incoming text offers Review text.
The secondary window displays plain nonselectable text; only Copy replaces this
clipboard, after fresh unlock/epoch/resource checks and GTK provider acknowledgement.
Receipt/view/reconnect never copies or opens links. Lock/epoch change closes review;
closing review preserves an explicit prior copy. Daemon text is bounded and epoch-local,
not a permanent journal. Read recovery uses Recover operation; pending reads block
new source/settings mutations. Outgoing preparation and URL opening
remain separate capabilities. See [native-text.md](../docs/native-text.md) for contracts
and exact actual observations; no installed Wayland/Google acceptance is implied.

Send text… in the header/context/tray menu opens a private editor. Write text or
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
128 MiB aggregate in the daemon. Mutable tmpfs sources are rejected; ordinary disk
sources retain their existing descriptor behavior. For automated disk-source tests
on a host with tmpfs /tmp, set TMPDIR to a directory on a disk filesystem first.

The native Debian builder is `python3 scripts/native_deb.py build` from the repository
root. It uses locked release binaries, derives actual ELF library dependencies,
stages fixed systemd/D-Bus/application/Dolphin assets and verifies the extracted
artifact offline. `--profile dev` labels a development package explicitly. No package
is installed or service enabled by build/verification. See
[packaging/README.md](packaging/README.md) for requirements and remaining acceptance.


Completed incoming files now offer Show in folder per manifest entry under Full >
Transfers. This uses the original completed transfer's collision-resolved result,
not the current receive setting. The daemon retains bounded private receipts and
opens a short-lived verified read-only descriptor on demand; the native client
passes that descriptor to portal OpenDirectory after the explicit click. Rendering,
receive completion and reconnect do not launch anything. Missing/moved/changed
files and unavailable portals show a finite error; no default application executes
the received content. Cancel folder request, hide/lock/epoch change and shutdown
close owned pending work. Already accepted external actions cannot be retracted.
See [native-reveal.md](../docs/native-reveal.md) for exact limits, ownership, actual
checks and physical KDE selection/activation acceptance still required.

Settings > Start at sign-in > Apply login start enables the vendor desktop user
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
