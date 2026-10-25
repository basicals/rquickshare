#!/usr/bin/env python3
"""Package the validated build artifacts as an experimental Noble BLE-Lite beta.

This script only builds the Debian file; CI separately installs, tests and
removes it in a disposable Ubuntu 24.04 container. It does not modify hosts.
"""
from pathlib import Path
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
STAGE = Path(os.environ["RUNNER_TEMP"]) / "rquickshare-native-noble"
OUT = ROOT / "noble-diagnostics"
ASSETS = ROOT / "native_desktop/packaging"


def copy(src, dest, mode=0o644):
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dest)
    dest.chmod(mode)


def main():
    from native_deb import FILES
    for name, (src, mode) in FILES.items():
        dest = STAGE / name
        if src is None:
            if not dest.is_file():
                raise RuntimeError(f"Missing staged executable: {name}")
            continue
        copy(src, dest, mode)

    # Keep Glycin's bundled decoder and configuration as verified by CI.
    # This remains an experimental build, not a signed distribution release.
    control_dir = STAGE / "DEBIAN"
    control_dir.mkdir()
    depends = (
        "libc6 (>= 2.38), libgtk-4-1 (>= 4.12), libglib2.0-0t64, "
        "libdbus-1-3, libwayland-client0, libseccomp2, "
        "bluez, bubblewrap, systemd, libpam-systemd, dbus-user-session, "
        "xdg-desktop-portal"
    )
    control = (
        "Package: rquickshare-native-noble-beta\n"
        "Version: 1.0.0-2\n"
        "Architecture: amd64\n"
        "Maintainer: rQuickShare experimental build <noreply@github.com>\n"
        "Section: net\n"
        "Priority: optional\n"
        "Conflicts: rquickshare, r-quick-share\n"
        "Replaces: rquickshare, r-quick-share\n"
        f"Depends: {depends}\n"
        "Description: Low-radio-activity GTK4 rQuickShare beta for Noble\n"
        " Experimental build requiring manual Bluetooth and KDE integration validation.\n"
        " Contains an experimental locally bundled Glycin decoder.\n"
    )
    (control_dir / "control").write_text(control)
    (control_dir / "control").chmod(0o644)
    for folder in [STAGE, *(x for x in STAGE.rglob("*") if x.is_dir())]:
        folder.chmod(0o755)
    OUT.mkdir(exist_ok=True)
    deb = OUT / "rquickshare-native-noble-BLE-LITE_1.0.0-2_amd64.deb"
    subprocess.run(
        ["dpkg-deb", "--root-owner-group", "--build", str(STAGE), str(deb)],
        check=True,
    )
    with deb.open("rb") as stream:
        sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
    (OUT / (deb.name + ".sha256")).write_text(f"{sha256}  {deb.name}\n")

    list_result = subprocess.check_output(["dpkg-deb", "-c", str(deb)], text=True)
    (OUT / "deb-inventory.txt").write_text(list_result)
    subprocess.run(["dpkg-deb", "--info", str(deb)], check=True)
    # APT simulation is intentionally diagnostic, never an installation.
    apt = subprocess.run(
        ["sudo", "apt-get", "-s", "install", str(deb)],
        text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        check=False,
    )
    (OUT / "apt-install-simulation.txt").write_text(
        "WARNING: simulation is not a guarantee of safe installation.\n"
        + apt.stdout + f"\nexit_code={apt.returncode}\n"
    )
    print("Built experimental BLE-Lite beta:", deb)
    print("APT simulation exit:", apt.returncode)
    print("CI must separately validate installation, removal and ELF dependencies.")


if __name__ == "__main__":
    sys.path.insert(0, str(ROOT / "scripts"))
    main()
