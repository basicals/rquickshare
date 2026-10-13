# Native development shell

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
Without the prerequisites, startup fails explicitly. The panel stays hidden until
tray activation or a CLI import. Escape/close hides it; Quit closes local clients, not daemon work.

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

Run the nine scoped methods, including the single GTK test, on an isolated display:

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
Fixed stderr errors describe busy/recovery/startup failures. Desktop notification
routing of those errors remains required before shipping the Dolphin action.
The public ingress contract and measured SDK checks are in
[native-desktop-ingress.md](../docs/native-desktop-ingress.md).
