# rQuickShare GTK4 BLE-Lite for Ubuntu 24.04 / KDE neon

Status: **experimental, field-tested on one HP Spectre x360**. Working reference: Debian package `rquickshare-native-noble-beta` **1.0.0-2**, compiled from `gtk4-noble`, successful workflow run [40192804021](https://github.com/basicals/rquickshare/actions/runs/40192804021).

## Reported field results

- Native GTK4 client and daemon start successfully in KDE neon Wayland.
- Sending and receiving files work.
- With the earlier beta (1.0.0-1), receiving a file could make a Logitech MX Anywhere 3 mouse lag; a subsequent transfer left it lagging until mouse power was cycled.
- With BLE-Lite (1.0.0-2), the user reported sending and receiving without lag. This is a single-device observation, not general proof.
- Host adapter: Intel 8265/8275 Wi-Fi, Intel USB Bluetooth ID `8087:0a2b`.
- Earlier beta: BlueZ showed 2 active advertising instances while running; stopping client and daemon reduced it to 0. That observation doesn't by itself identify which BLE operation caused lag.

## Minimal behavioral change

In `core_lib/src/lib.rs`, the legacy FE2C `BleAdvertiser` task is **disabled by default**. The modern `ReceiverAdvertiser`, receiver GATT service, BLE discovery and transfer paths remain intact. This reduces redundant advertising without removing receive/send functionality.

For controlled regression troubleshooting only, opt the legacy advertiser back in by setting `RQS_ENABLE_LEGACY_BLE_ADVERTISING=1` in the **daemon's** environment before starting it. Do not enable it by default on hardware affected by mouse lag.

## Installation / rollback

The BLE-Lite build is an **experimental beta, not an officially signed release**. Fetch only from the known repository GitHub Actions run, inspect the .deb metadata and SHA256, and avoid disabling browser protection globally.

- Package name: `rquickshare-native-noble-beta`
- Version: `1.0.0-2`
- Architecture: `amd64`
- Artifact file: `rquickshare-native-noble-BLE-LITE_1.0.0-2_amd64.deb`
- The Debian control declares `Conflicts`/`Replaces` with `rquickshare` and `r-quick-share`.
- Stop user services before upgrading, use `sudo apt install ./<deb-file>`, and restart the two user services after installing.
- Rollback to the previous experimental beta requires the earlier 1.0.0-1 .deb and an explicit downgrade; alternatively remove BLE-Lite with `sudo apt remove rquickshare-native-noble-beta` and install another known-good package.
- Preserve personal configuration; don't run `autoremove` as part of debugging.

## Automated verification

GitHub Actions on Ubuntu 24.04 compiles the native GTK4 app and Glycin loader, builds a .deb, tests its installation and removal in a clean Ubuntu 24.04 container, checks dependency linking, validates desktop/systemd metadata and renders a GTK4 PNG preview in virtual Wayland. These tests do **not** cover live transfer under a real Intel adapter, mouse coexistence, or D-Bus app service activation.

## Follow-up before a stable release

Repeat Bluetooth coexistence testing on additional adapters/devices, inspect BLE scanning/advertising lifecycle during transfers, investigate D-Bus activation and verify upgrades from distribution packages. Do not merge this experiment to `master` based on one successful field report.
