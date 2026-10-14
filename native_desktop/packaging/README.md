# Native Linux package

The source builds a native Debian package with GTK4/Layer Shell GUI, independent
systemd user daemon, D-Bus activation, application entry and Dolphin Send action.
A build stages/extracts/verifies files only; it never installs or enables services.

From the repository root, with Rust >=1.92, Python >=3.11 and Debian development
libraries/tools installed:

```sh
python3 scripts/native_deb.py build
python3 scripts/native_deb.py verify native_desktop/target/packages/rquickshare_0.11.5+native1_amd64.deb
```

Default artifacts contain locked release binaries. `build --profile dev` explicitly
labels a development artifact `.debug`; it does not certify release optimization.
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
`/usr/lib/systemd/user/rquickshare-daemon.service` and fixed application/D-Bus/Dolphin
assets. It replaces the same-name legacy package on explicit installation; never
install two different packages owning `/usr/bin/rquickshare`. The legacy source
and release workflows remain migration baseline until native acceptance.

Explicit application launch/Reconnect requests the fixed daemon D-Bus name when
missing, requiring a systemd-activation-capable user bus. That request delegates
to the user unit. The activation Exec sentinel fails closed on unsupported brokers;
there is no shell/direct-daemon fallback. Type=exec supervises process lifetime,
preserving the daemon's joined settings-generation bus-name changes. SIGTERM joins
shutdown without an arbitrary forced-kill deadline. GUI Quit leaves daemon work
alone. The package has no enablement/autostart/maintainer start scripts and does not
start sharing merely because it is installed. Login-start controls and updates
remain separate work. No Google compatibility or installed KDE acceptance implied.

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
