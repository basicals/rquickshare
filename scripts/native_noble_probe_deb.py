#!/usr/bin/env python3
"""Create a STRICTLY EXPERIMENTAL Noble Debian artifact without installing it.

Builds on an Ubuntu 24.04 GitHub runner from already staged binaries. This is a
packaging/inventory probe, NOT an assertion that the package is installable.
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

    # Avoid system-wide Glycin loader registration collisions while the
    # application's loader discovery is still under investigation.
    # DO NOT suggest installation of this proof-of-packaging .deb.
    control_dir = STAGE / "DEBIAN"
    control_dir.mkdir()
    depends = (
        "libc6 (>= 2.38), libgtk-4-1 (>= 4.12), libglib2.0-0t64, "
        "libdbus-1-3, libwayland-client0, libseccomp2, "
        "bluez, bubblewrap, systemd, libpam-systemd, dbus-user-session, "
        "xdg-desktop-portal"
    )
    control = (
        "Package: rquickshare-native-noble-probe\n"
        "Version: 1.0.0-1\n"
        "Architecture: amd64\n"
        "Maintainer: rQuickShare experimental build <noreply@github.com>\n"
        "Section: net\n"
        "Priority: optional\n"
        "Conflicts: rquickshare, r-quick-share\n"
        "Replaces: rquickshare, r-quick-share\n"
        f"Depends: {depends}\n"
        "Description: Unvalidated Native GTK4 rQuickShare packaging probe\n"
        " NOT RELEASE READY. Do not install on a real system.\n"
        " Contains an experimental locally bundled Glycin decoder.\n"
    )
    (control_dir / "control").write_text(control)
    (control_dir / "control").chmod(0o644)
    for folder in [STAGE, *(x for x in STAGE.rglob("*") if x.is_dir())]:
        folder.chmod(0o755)
    OUT.mkdir(exist_ok=True)
    deb = OUT / "rquickshare-native-noble-PROBE_1.0.0-1_amd64.deb"
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
    print("Built packaging PROBE only:", deb)
    print("APT simulation exit:", apt.returncode)
    print("Not validated for installation; ELF dependency audit still required.")


if __name__ == "__main__":
    sys.path.insert(0, str(ROOT / "scripts"))
    main()
