# Building and checking rQuickShare

Native settings now have a daemon-owned transactional store and secondary GTK
editor; see [native-settings.md](docs/native-settings.md) for paths, read-only
developer overrides, migration/recovery and actual verification. Do not install or
start a second transport owner to review the uninstalled native source.

The project has three Rust crates and one frontend:

- `core_lib`: discovery, protocol, connections and payload transfers.
- `app/main/src-tauri`: desktop commands, notifications, logging and app lifecycle.
- `app/main`: Vue frontend and Tauri v2 CLI.
- `native_desktop`: additive GTK4/Layer Shell tray client; see its [build/check guide](native_desktop/README.md).

## Linux prerequisites

Use stable Rust for tests/Clippy, nightly Rust for repository rustfmt settings, Node.js 24 and pnpm 9.7.0. The desktop toolchain file selects nightly; explicitly use `cargo +stable` for its tests/Clippy, as CI does.

On Debian/Ubuntu, install the CI development dependencies:

```bash
sudo apt-get install protobuf-compiler libdbus-1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev
```

From `app/main`, install locked frontend dependencies:

```bash
pnpm install --frozen-lockfile
```

## Development and packages

The native Linux package has an independent direct Debian build path:

```bash
python3 scripts/native_deb.py build
```

This builds locked release GUI/daemon binaries, derives ELF runtime dependencies,
stages the systemd user/D-Bus/application/Dolphin assets and verifies the archive,
checksum and extracted offline help/settings fixture. It never installs, enables
or starts services. Python >=3.11, Debian tooling and native GTK4/Layer Shell
development dependencies are required; see [native packaging](native_desktop/packaging/README.md).
Normal native application launch delegates to its desktop user service; development
without installed assets uses the foreground `rquickshare --service` worker.
Settings has default-off opt-in login start for daemon plus hidden-card tray;
see [native autostart](docs/native-autostart.md). Disabling it affects future logins
without stopping live transfers. No build installs/enables these assets.
The manual native-package-smoke workflow targets an Ubuntu26.04 container explicitly.
Existing Tauri release jobs remain the migration baseline; native installed startup,
upgrade/removal and KDE/Google acceptance are still separate gates.

From `core_lib`, use `cargo build` or `cargo build --release`. From `app/main`:

```bash
pnpm dev                         # Desktop development app
pnpm exec tauri build --debug --bundles deb
pnpm build                       # Release bundles for the current OS
```

Packaging remains a separate manual Actions gate. A package build does not certify installation, desktop behavior or Android interoperability. See [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) for other platforms.

## Automated checks

### Native Linux daemon (development)

The native executable is additive; the current packaged desktop remains Tauri.
Build/check settings without connecting to D-Bus or starting transports:

```bash
cargo +stable build --manifest-path core_lib/Cargo.toml --bin rquickshare-daemon --locked
core_lib/target/debug/rquickshare-daemon --check
```

The original legacy Linux AppData settings are read at
`$XDG_DATA_HOME/dev.mandre.rquickshare/.settings.json` (usual fallback
`~/.local/share/dev.mandre.rquickshare/.settings.json`) when no private native store
exists. Normal startup prefers the daemon-owned native store described in
[native-settings.md](docs/native-settings.md). `--settings PATH` is a
development override; `--help` lists arguments. Normal manual launch uses the
user session's local Unix D-Bus address and must not run alongside the old desktop
or developer runner. SIGINT/SIGTERM request joined shutdown. A private user-owned
runtime directory is required; do not run with sudo or register a system-bus service.

Invalid visibility starts hidden and exposes GetStartupWarnings; the existing UI
now offers explicit in-app recovery when used independently. Native GTK/card and
notification clients and persistent native settings/repair are implemented in
source; see [native settings ownership](docs/native-settings.md). Whole-file
corruption/read errors do not reset settings.
No activation/autostart/package files are installed by a build. Continuous production
use is still gated on installed settings/migration acceptance, package lifecycle and
physical KDE/Google peer acceptance. Implemented notification activation assets
remain source-only; a build never installs them.
Contracts and actual verification: [native startup](docs/native-daemon-startup.md).

The default `cargo run` remains the legacy `core_bin` developer runner; select the
native executable explicitly. Linux owner guards cover updated executables, not
arbitrarily old external builds. Core tests use test-owned dbus-daemon brokers and
process fixtures; install dbus-daemon for these tests, as CI declares.

From `core_lib`:

```bash
cargo +stable test --all-features
cargo +stable clippy --all-features --all-targets -- -D warnings
```

From `app/main/src-tauri`:

```bash
cargo +nightly fmt --all --check
cargo +stable test --no-default-features --bin rquickshare
cargo +stable clippy --no-default-features --all-targets -- -D warnings
```

From `app/main`:

```bash
pnpm lint
pnpm ts-check
pnpm exec vitest run
```

Desktop unit tests run without launching the GUI but require Linux development libraries. Core/frontend tests do not substitute for desktop assertions.

From the repository root, run the offline evidence analyzer tests (Python 3.9+; no third-party packages):

```bash
python3 -m unittest discover -s scripts/tests -v
```

The preflight runs checks once. The manual/tag-triggered Mode B workflow repeats complete suites ten times and measures coverage. Physical Pixel/KDE checks and final post-hardware signoff remain separate; see [Mode B plan](docs/mode-b-test-plan.md) and [offline work log](docs/offline-maintenance.md).
