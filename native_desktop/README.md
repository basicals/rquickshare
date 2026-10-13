# Native development shell

The tray now exports a standard DBusMenu with Open, file/folder selection, incoming/
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

Normal launch requires a supported Layer Shell compositor, a same-user KDE status
notifier watcher and the manually started native daemon on the user session bus.
Follow [native-daemon-startup.md](../docs/native-daemon-startup.md); do not start it
beside an existing transport owner. No implicit daemon installation/activation.
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
never sends automatically. A missing GUI is started from this same executable,
without paths. The daemon must already be started explicitly. The panel's own
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
new source/settings mutations. Outgoing native text/clipboard/image and URL opening
remain separate capabilities. See [native-text.md](../docs/native-text.md) for contracts
and exact actual observations; no installed Wayland/Google acceptance is implied.
