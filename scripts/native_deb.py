#!/usr/bin/env python3
"""Build/verify an uninstalled native Debian artifact. Never calls a user manager."""
import argparse
import configparser
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "native_desktop/packaging"
FILES = {
    "usr/bin/rquickshare": (None, 0o755),
    "usr/libexec/rquickshare-daemon": (None, 0o755),
    "usr/libexec/rquickshare-preview-helper": (None, 0o755),
    "usr/share/doc/rquickshare-native/LICENSE": (ROOT / "LICENSE", 0o644),
    "usr/share/doc/rquickshare-native/README.md": (ASSETS / "README.md", 0o644),
}
for relative in [
    "lib/systemd/user/rquickshare-daemon.service",
    "lib/systemd/user/rquickshare-desktop.service",
    "share/applications/io.github.morozhenko_apps.RQuickShare.desktop",
    "share/dbus-1/services/io.github.morozhenko_apps.RQuickShare.service",
    "share/dbus-1/services/io.github.morozhenko_apps.RQuickShare.Desktop.service",
    "share/kio/servicemenus/io.github.morozhenko_apps.RQuickShare.Send.desktop",
]:
    FILES["usr/" + relative] = (ASSETS / relative, 0o644)

for size, source in [(32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png")]:
    FILES[f"usr/share/icons/hicolor/{size}x{size}/apps/rquickshare.png"] = (
        ROOT / "native_desktop/assets/icons" / source, 0o644,
    )


def run(*args, cwd=None, env=None):
    return subprocess.check_output(args, cwd=cwd, env=env, text=True).strip()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def native_revision(value):
    require(type(value) is int and 1 <= value <= 2147483647, "Invalid native revision")
    return value


def native_version(version, revision, debug=False):
    require(re.fullmatch(r"\d+\.\d+\.\d+", version) is not None, "Invalid source version")
    return version + "-" + str(native_revision(revision)) + (".debug" if debug else "")


def validate_metadata(fields):
    require(fields.get("Package") == "rquickshare-native", "Unexpected package identity")
    require(fields.get("Conflicts") == "rquickshare" and fields.get("Replaces") == "rquickshare", "Missing legacy package conflict/replacement")
    version = re.fullmatch(r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)-([1-9][0-9]{0,9})(?:\.debug)?", fields.get("Version", ""))
    require(version is not None, "Invalid native version")
    native_revision(int(version[1]))
    require(fields.get("Architecture") in {"amd64", "arm64"}, "Unsupported architecture")
    depends = fields.get("Depends", "")
    names = {re.match(r"[a-z0-9+.-]+", item.strip())[0] for item in depends.split(",") if re.match(r"[a-z0-9+.-]+", item.strip())}
    require({"systemd", "libpam-systemd", "dbus-user-session", "bluez", "xdg-desktop-portal", "libgtk-4-1", "libgtk4-layer-shell0", "bubblewrap", "glycin-loaders"} <= names, "Missing runtime dependency")
    require("libgtk-4-1 (>= 4.12)" in depends, "Missing GTK API floor")
    require("glycin-loaders (>= 2.1)" in depends, "Missing Glycin loader floor")


def validate_elf(header, architecture):
    require(len(header) >= 20 and header[:6] == b"\x7fELF\x02\x01", "Expected little-endian 64-bit ELF")
    require(int.from_bytes(header[16:18], "little") in {2, 3}, "Expected executable ELF")
    require(int.from_bytes(header[18:20], "little") == {"amd64": 62, "arm64": 183}[architecture], "ELF architecture mismatch")


def validate_assets():
    application = configparser.ConfigParser(interpolation=None)
    application.read(ASSETS / "share/applications/io.github.morozhenko_apps.RQuickShare.desktop")
    require(application["Desktop Entry"].get("Icon") == "rquickshare", "Unexpected application icon")
    unit = configparser.ConfigParser(interpolation=None)
    unit.read(ASSETS / "lib/systemd/user/rquickshare-daemon.service")
    require(set(unit.sections()) == {"Unit", "Service"}, "Unexpected unit enablement")
    require(dict(unit["Service"]) == {
        "type": "exec", "execstart": "/usr/libexec/rquickshare-daemon",
        "restart": "no", "timeoutstopsec": "infinity", "sendsigkill": "no",
    }, "Unsafe process lifetime contract")
    require(unit["Unit"].get("PartOf") == "graphical-session.target", "Missing session ownership")
    activation = configparser.ConfigParser(interpolation=None)
    activation.read(ASSETS / "share/dbus-1/services/io.github.morozhenko_apps.RQuickShare.service")
    require(dict(activation["D-BUS Service"]) == {
        "name": "io.github.morozhenko_apps.RQuickShare",
        "systemdservice": "rquickshare-daemon.service", "exec": "/usr/bin/false",
    }, "Unsafe daemon activation contract")
    desktop = configparser.ConfigParser(interpolation=None)
    desktop.read(ASSETS / "lib/systemd/user/rquickshare-desktop.service")
    require(set(desktop.sections()) == {"Unit", "Service", "Install"}, "Unexpected desktop unit sections")
    require(dict(desktop["Service"]) == {
        "type": "exec", "execstart": "/usr/bin/rquickshare --service",
        "restart": "no", "timeoutstopsec": "infinity", "sendsigkill": "no",
    }, "Unsafe desktop lifetime contract")
    require(desktop["Unit"].get("PartOf") == "graphical-session.target"
            and desktop["Unit"].get("Wants") == "rquickshare-daemon.service"
            and desktop["Unit"].get("After") == "graphical-session-pre.target dbus.socket rquickshare-daemon.service",
            "Unsafe desktop session ordering")
    require(dict(desktop["Install"]) == {"wantedby": "graphical-session.target"}, "Unsafe opt-in enablement")
    gui = configparser.ConfigParser(interpolation=None)
    gui.read(ASSETS / "share/dbus-1/services/io.github.morozhenko_apps.RQuickShare.Desktop.service")
    require(dict(gui["D-BUS Service"]) == {
        "name": "io.github.morozhenko_apps.RQuickShare.Desktop",
        "systemdservice": "rquickshare-desktop.service", "exec": "/usr/bin/false",
    }, "Unsafe desktop activation contract")


def checksum(package):
    with package.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return digest + "  " + package.name + "\n"


def verify(package, smoke=True):
    package = package.resolve(strict=True)
    require(package.with_suffix(package.suffix + ".sha256").read_text() == checksum(package), "Checksum mismatch")
    fields = {name: run("dpkg-deb", "--field", str(package), name) for name in ["Package", "Version", "Architecture", "Depends", "Conflicts", "Replaces"]}
    validate_metadata(fields)
    validate_assets()
    allowed_dirs = {"", "."} | {str(parent) for name in FILES for parent in Path(name).parents}
    for control in [False, True]:
        process = subprocess.Popen(["dpkg-deb", "--ctrl-tarfile" if control else "--fsys-tarfile", str(package)], stdout=subprocess.PIPE)
        try:
            seen = set()
            with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
                for entry in archive:
                    require(entry.uid == 0 and entry.gid == 0, "Non-root archive ownership")
                    name = entry.name.removeprefix("./").rstrip("/")
                    require(not name.startswith("/") and ".." not in Path(name).parts, "Unsafe archive path")
                    if entry.isdir():
                        require(entry.mode == 0o755 and (name in {"", "."} if control else name in allowed_dirs), "Unexpected directory/mode")
                        continue
                    require(entry.isfile() and name not in seen, "Unexpected link/special/duplicate entry")
                    seen.add(name)
                    if control:
                        require(name == "control" and entry.mode == 0o644, "Unexpected maintainer script/control asset")
                    else:
                        require(name in FILES and entry.mode == FILES[name][1], f"Unexpected package file/mode: {name} ({entry.mode:o})")
                        source = FILES[name][0]
                        if source is not None:
                            require(archive.extractfile(entry).read() == source.read_bytes(), "Packaged asset mismatch")
                        else:
                            validate_elf(archive.extractfile(entry).read(64), fields["Architecture"])
            require(seen == ({"control"} if control else set(FILES)), "Incomplete package inventory")
        finally:
            process.stdout.close()
            process.wait()
        require(process.returncode == 0, "Cannot read Debian archive")
    if smoke:
        require(fields["Architecture"] == run("dpkg", "--print-architecture"), "Cannot smoke foreign architecture")
        with tempfile.TemporaryDirectory(prefix="rqs-deb-verify-") as temporary:
            directory = Path(temporary)
            run("dpkg-deb", "--extract", str(package), str(directory))
            clean = os.environ.copy()
            clean["HOME"] = str(directory / "home")
            clean["XDG_CONFIG_HOME"] = str(directory / "config")
            clean["XDG_DATA_HOME"] = str(directory / "data")
            clean.pop("DBUS_SESSION_BUS_ADDRESS", None)
            clean.pop("DISPLAY", None)
            clean.pop("WAYLAND_DISPLAY", None)
            gui = run(str(directory / "usr/bin/rquickshare"), "--help", env=clean)
            require("native tray client" in gui, "Not the native GUI binary")
            daemon = directory / "usr/libexec/rquickshare-daemon"
            helper = directory / "usr/libexec/rquickshare-preview-helper"
            require("Requires Bubblewrap sandbox" in run(str(helper), "--help", env=clean), "Not the private preview helper")
            require("--check" in run(str(daemon), "--help", env=clean), "Not the native daemon")
            settings = directory / "fixture.json"
            settings.write_text('{"visibility":0}')
            require("no service started" in run(str(daemon), "--settings", str(settings), "--check", env=clean), "Offline settings check failed")
    print("Native Debian package verified:", package)
    return fields


def build(args):
    validate_assets()
    versions = [tomllib.loads((ROOT / crate / "Cargo.toml").read_text())["package"]["version"] for crate in ["core_lib", "native_desktop"]]
    require(re.fullmatch(r"\d+\.\d+\.\d+", versions[1]) is not None, "Invalid desktop SemVer")
    architecture = run("dpkg", "--print-architecture")
    require(architecture in {"amd64", "arm64"}, "Unsupported native architecture")
    require(not os.environ.get("CARGO_BUILD_TARGET") and not os.environ.get("CARGO_TARGET_DIR"), "Use native default cargo target directories")
    revision = tomllib.loads((ASSETS / "release.toml").read_text())["revision"]
    version = native_version(versions[1], revision, args.profile == "dev")
    for crate, binary in [("core_lib", "rquickshare-daemon"), ("native_desktop", "rquickshare"), ("native_desktop", "rquickshare-preview-helper")]:
        subprocess.run(["cargo", "+stable", "build", "--locked", "--manifest-path", str(ROOT / crate / "Cargo.toml"), "--bin", binary, "--profile", args.profile], check=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    package = output / f"rquickshare-native_{version}_{architecture}.deb"
    with tempfile.TemporaryDirectory(prefix="rqs-deb-build-") as temporary:
        work = Path(temporary)
        stage = work / "debian/rquickshare-native"
        for destination, (source, mode) in FILES.items():
            if source is None:
                crate = "core_lib" if destination.endswith("/rquickshare-daemon") else "native_desktop"
                source = ROOT / crate / "target" / ("debug" if args.profile == "dev" else "release") / Path(destination).name
            target = stage / destination
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            target.chmod(mode)
        (work / "debian").mkdir(exist_ok=True)
        (work / "debian/control").write_text("Source: rquickshare-native\n\nPackage: rquickshare-native\nArchitecture: any\nDescription: Native nearby sharing\n")
        control = stage / "DEBIAN"
        control.mkdir()
        command = ["dpkg-shlibdeps", "-O", "-e" + str(stage / "usr/bin/rquickshare"), "-e" + str(stage / "usr/libexec/rquickshare-daemon"), "-e" + str(stage / "usr/libexec/rquickshare-preview-helper")]
        for library in args.library_dir:
            command.append("-l" + str(library.resolve(strict=True)))
        if args.shlibs_file:
            command.append("-L" + str(args.shlibs_file.resolve(strict=True)))
        dependencies = run(*command, cwd=work)
        require(dependencies.startswith("shlibs:Depends="), "Missing generated ELF dependencies")
        depends = dependencies.removeprefix("shlibs:Depends=") + ", libgtk-4-1 (>= 4.12), systemd, libpam-systemd, dbus-user-session, bluez, xdg-desktop-portal, bubblewrap, glycin-loaders (>= 2.1)"
        validate_metadata({"Package": "rquickshare-native", "Version": version, "Architecture": architecture, "Depends": depends, "Conflicts": "rquickshare", "Replaces": "rquickshare"})
        (control / "control").write_text(f"Package: rquickshare-native\nVersion: {version}\nArchitecture: {architecture}\nConflicts: rquickshare\nReplaces: rquickshare\nMaintainer: rQuickShare contributors <noreply@github.com>\nSection: net\nPriority: optional\nDepends: {depends}\nRecommends: xdg-desktop-portal-kde\nDescription: Native nearby file and text sharing\n GTK4 tray client and independently owned transfer daemon.\n")
        (control / "control").chmod(0o644)
        for directory in [stage, *[path for path in stage.rglob("*") if path.is_dir()]]:
            directory.chmod(0o755)
        environment = os.environ.copy()
        environment.setdefault("SOURCE_DATE_EPOCH", str(int((ROOT / "core_lib/Cargo.toml").stat().st_mtime)))
        subprocess.run(["dpkg-deb", "--root-owner-group", "--build", str(stage), str(package)], check=True, env=environment)
    package.with_suffix(".deb.sha256").write_text(checksum(package))
    verify(package)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    create = commands.add_parser("build")
    create.add_argument("--profile", choices=["release", "dev"], default="release")
    create.add_argument("--output", type=Path, default=ROOT / "native_desktop/target/packages")
    create.add_argument("--shlibs-file", type=Path, help="Actual Debian shlibs metadata for privately extracted libraries")
    create.add_argument("--library-dir", type=Path, action="append", default=[])
    check = commands.add_parser("verify")
    check.add_argument("package", type=Path)
    args = parser.parse_args()
    try:
        build(args) if args.command == "build" else verify(args.package)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print("Native package failed:", error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
