# Native Linux package

The source builds a native Debian package with GTK4/Layer Shell GUI, independent
systemd user daemon, D-Bus activation, application entry and Dolphin Send action.
A build stages/extracts/verifies files only; it never installs or enables services.

From the repository root, with Rust >=1.92, Python >=3.11 and Debian development
libraries/tools installed:

```sh
python3 scripts/native_deb.py build
python3 scripts/native_deb.py verify native_desktop/target/packages/rquickshare_0.11.5+native2_amd64.deb
```

Default artifacts contain locked release binaries. `build --profile dev` explicitly
labels a development artifact `.debug`; it does not certify release optimization.
The tracked packaging/release.toml revision increments for every changed native
release at the same Rust version. APT needs a strictly newer package version;
never silently replace the bytes of an already published version.
The builder derives runtime shared-library dependencies from both actual ELF files
using dpkg-shlibdeps and fails if library metadata is missing. GTK4>=4.12 and Layer
Shell are required. ABI compatibility is determined by the build environment: a
26.04 build is not an Ubuntu24.04 compatibility claim. Native amd64/arm64 only.
`--output DIRECTORY` changes the artifact location without installing anything.

For an explicitly private development sysroot, `--library-dir DIRECTORY` and
`--shlibs-file FILE` accept actual extracted Debian library metadata; no unresolved
library is ignored. Such an artifact still requires those libraries when installed.
The package and neighboring SHA256 sidecar are verified together. Verification
checks exact allowed paths, owner/mode, assets, dependencies and native identity,
then runs extracted GUI/daemon --help and a private fixture --settings PATH --check.
It does not connect to the user bus, show UI or read user configuration.

The `rquickshare` package owns `/usr/bin/rquickshare`, `/usr/libexec/rquickshare-daemon`,
both daemon and desktop user units and fixed application/D-Bus/Dolphin
assets. It replaces the same-name legacy package on explicit installation; never
install two different packages owning `/usr/bin/rquickshare`. The legacy source
and release workflows remain migration baseline until native acceptance.

Normal application/Dolphin/daemon-wake launch delegates to the fixed desktop user
unit through D-Bus. The foreground worker is `rquickshare --service`; it starts
hidden and waits for actual tray readiness. An explicit normal launch sends Show
to the existing owner. Separate retained process guards prevent competing daemon
or GUI workers, including reconnect. No shell/direct-child fallback is provided.
Both Type=exec services join shutdown and belong to the graphical session. GUI
Quit leaves daemon work alone; an external blocked IO may delay joined shutdown.

Settings > Start at sign-in > Apply login start explicitly enables only the
package desktop unit for future graphical sessions. It pulls in the daemon and
starts the tray with the card hidden. The default is disabled. Disabling future
startup never stops current services or transfers. This setting reads actual
systemd state, checks the vendor unit and uses no-force persistent enablement;
custom/masked/unknown state remains unavailable. Unit-file changes are followed by
manager configuration reload and actual readback; no service start/stop is issued.
Reload resolves unconfirmed edits without repeating unit-file writes.
The package has no maintainer enable/start scripts, presets or separate XDG login
launcher; installation alone starts/enables nothing. A supported systemd user bus,
graphical session environment and KDE tray watcher are required. Updates and
installed lifecycle acceptance remain separate work. No Google compatibility or
installed KDE acceptance is implied.

Native updates use the selected dedicated signed APT distribution. The local
scripts/native_apt.py prepares immutable standard indexes/by-hash/Release, signs
using an explicit external GPG home and independently selected public fingerprint,
verifies the complete trust/hash chain, and emits reviewable scoped deb822 bootstrap
files. The application does not download/install its own replacements. Repository
metadata expires after seven days and must be refreshed/resigned before expiry.
The native CI artifact is explicitly unsigned; no production key, repository URI,
enrollment or publication is configured by a build. See docs/native-apt.md in the
source tree for signing/promotion commands, tests and remaining production binding.

Dolphin uses `rquickshare send -- %U`, 1..256 local files/folders as separate arguments.
Acquisition is all-or-nothing; preparation preserves existing source and requires
explicit recipient selection and Send. GUI activation is fixed and receives no
untrusted paths; transfer/source handoff uses the bounded typed descriptor API.

Installed acceptance remains: systemd delegation/session environment/logout,
settings restart without process death, old-owner collision, upgrades/removal with
running transfers, actual Dolphin multi-selection/folders/Unicode/percent/error/
busy cases, menu discovery, GTK Layer Shell/portal lock/DND/focus/scale and real
Google peers. Staged package verification does not establish these observations.
Contracts and measured checks are maintained in repository docs/native-package.md,
docs/native-desktop-ingress.md and docs/native-notification-backend.md.
