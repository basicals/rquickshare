# Building and checking rQuickShare

The project has two Rust crates and one frontend:

- `core_lib`: discovery, protocol, connections and payload transfers.
- `app/main/src-tauri`: desktop commands, notifications, logging and app lifecycle.
- `app/main`: Vue frontend and Tauri v2 CLI.

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

From `core_lib`, use `cargo build` or `cargo build --release`. From `app/main`:

```bash
pnpm dev                         # Desktop development app
pnpm exec tauri build --debug --bundles deb
pnpm build                       # Release bundles for the current OS
```

Packaging remains a separate manual Actions gate. A package build does not certify installation, desktop behavior or Android interoperability. See [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) for other platforms.

## Automated checks

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

The preflight runs checks once. The manual/tag-triggered Mode B workflow repeats complete suites ten times and measures coverage. Physical Pixel/KDE checks and final post-hardware signoff remain separate; see [Mode B plan](docs/mode-b-test-plan.md) and [offline work log](docs/offline-maintenance.md).
