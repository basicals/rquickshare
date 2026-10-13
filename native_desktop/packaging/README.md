# Native desktop package assets

Source assets for the future native package; nothing here is installed by a build.
Package the native `rquickshare` executable and these files in the same package:

| Source | Package destination | Mode |
|---|---|---|
| `../target/release/rquickshare` | `/usr/bin/rquickshare` | 0755 |
| `share/kio/servicemenus/io.github.morozhenko_apps.RQuickShare.Send.desktop` | `/usr/share/kio/servicemenus/io.github.morozhenko_apps.RQuickShare.Send.desktop` | 0644 |

The package manager owns installation/removal. Uninstall removes its executable
and service menu together. No first-run copy into user configuration, standalone
menu installer, shell wrapper, or persistent user-config cleanup is needed.
A package must resolve the existing Tauri executable's conflicting path before
release; do not install this asset beside the legacy `rquickshare` binary, which
has no native `send` contract. Native daemon activation/settings/package migration
and dependency metadata remain a separate release stage.

The action is local-only, accepts 1..256 files/folders, and passes each URL as a
separate argument after `--`. CLI acquisition is all-or-nothing. The panel preserves
an existing preparation and requires explicit recipient selection and Send.
The themed icon is a standard desktop send action, not a new application logo.

Release gates: complete native notification/error presentation for nonterminal
launch; installed daemon/GUI startup and migration acceptance; actual Dolphin
multi-selection/folders/Unicode/percent/error/busy behavior; menu discovery,
package upgrade/uninstall and KDE monitor/scale/focus checks. These assets and
private-broker tests do not establish installed-product or Google peer acceptance.

Contract and measured checks: [native-desktop-ingress.md](../../docs/native-desktop-ingress.md).
KDE reference: https://develop.kde.org/docs/apps/dolphin/service-menus/
