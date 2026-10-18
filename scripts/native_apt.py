#!/usr/bin/env python3
"""Prepare/sign/verify native APT snapshots locally; never enroll, install or publish."""
import argparse
import ctypes
from contextlib import contextmanager
from datetime import datetime, timedelta, timezone
from email.utils import format_datetime, parsedate_to_datetime
import gzip
import hashlib
import ipaddress
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit

import native_deb

SUITE = "rquickshare-native-ubuntu26.04"
PRODUCTION_URI = "https://download.morozhenko.com/dist/rQuickShare/"
LIFETIME = timedelta(days=7)
MAX_INDEX = 1024 * 1024
MAX_PACKAGE = 1024 * 1024 * 1024
require = native_deb.require


def run(*args, cwd=None):
    return subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True).stdout


def fingerprint(value):
    require(isinstance(value, str) and re.fullmatch(r"(?:[A-F0-9]{40}|[A-F0-9]{64})", value), "Expected full uppercase primary fingerprint")
    return value


def uri(value):
    require(isinstance(value, str) and len(value) <= 2048 and all(33 <= ord(c) < 127 for c in value), "Invalid repository URI")
    parsed = urlsplit(value)
    require(parsed.scheme == "https" and parsed.hostname and parsed.username is None and parsed.password is None
            and "?" not in value and "#" not in value and not parsed.netloc.endswith(":"), "Expected credential-free HTTPS repository URI")
    if ":" in parsed.hostname:
        ipaddress.IPv6Address(parsed.hostname)
    else:
        require(len(parsed.hostname) <= 253 and all(re.fullmatch(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?", label)
                for label in parsed.hostname.split(".")), "Invalid repository hostname")
    require(parsed.port is None or 1 <= parsed.port <= 65535, "Invalid repository port")
    require(not any(part in {".", ".."} for part in parsed.path.split("/")) and "%" not in value and "\\" not in value, "Invalid repository path")
    return value.rstrip("/")


def regular(path, maximum):
    require(not path.is_symlink() and path.is_file() and path.stat().st_size <= maximum, "Expected bounded regular input file")


def digest(path):
    regular(path, MAX_PACKAGE)
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read(path, maximum=MAX_INDEX):
    regular(path, maximum)
    return path.read_bytes()


@contextmanager
def stage(output):
    output = output.absolute()
    require(not output.exists() and not output.is_symlink(), "Output already exists; snapshots are immutable")
    require(output.parent.is_dir() and output.parent.resolve() == output.parent, "Output parent must be an existing canonical directory")
    with tempfile.TemporaryDirectory(prefix=".rqs-apt-", dir=output.parent) as temporary:
        root = Path(temporary) / "snapshot"
        root.mkdir(mode=0o755)
        yield root
        for path in [root, *root.rglob("*")]:
            require(not path.is_symlink() and (path.is_file() or path.is_dir()), "Unexpected staged special file")
            path.chmod(0o755 if path.is_dir() else 0o644)
        # Linux RENAME_NOREPLACE preserves immutable output even across a concurrent creator.
        libc = ctypes.CDLL(None, use_errno=True)
        rename = libc.renameat2
        rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
        rename.restype = ctypes.c_int
        if rename(-100, os.fsencode(root), -100, os.fsencode(output), 1) != 0:
            raise OSError(ctypes.get_errno(), "Cannot publish local immutable snapshot")


def paragraphs(data):
    require(len(data) <= MAX_INDEX, "Metadata exceeds limit")
    text = data.decode("utf-8")
    require("\x00" not in text and "\r" not in text, "Invalid metadata encoding")
    result = []
    for paragraph in text.strip("\n").split("\n\n"):
        fields = {}
        previous = None
        for line in paragraph.splitlines():
            if line.startswith(" "):
                require(previous is not None, "Unexpected metadata continuation")
                fields[previous] += "\n" + line
            else:
                name, separator, value = line.partition(":")
                require(separator and re.fullmatch(r"[A-Za-z0-9-]+", name) and name not in fields
                        and (value.startswith(" ") or value == "" and name == "SHA256"), "Invalid/duplicate metadata field")
                previous = name
                fields[name] = value.removeprefix(" ")
        require(0 < len(fields) <= 64, "Invalid metadata field count")
        result.append(fields)
    return result


def index(root, architecture):
    raw = run("dpkg-scanpackages", "--multiversion", "--hash", "sha256", "pool", "/dev/null", cwd=root).encode()
    entries = [entry for entry in paragraphs(raw) if entry.get("Architecture") == architecture]
    require(len(entries) == 1, "Expected one native release per architecture")
    entry = entries[0]
    return ("\n".join(f"{name}: {value}" for name, value in sorted(entry.items())) + "\n\n").encode()


def release(root, architectures, date):
    require(date.tzinfo is not None and date.utcoffset() == timedelta(0) and date.microsecond == 0, "Expected whole-second UTC date")
    fields = ["Origin: rQuickShare", "Label: rQuickShare", f"Suite: {SUITE}", f"Codename: {SUITE}",
              "Architectures: " + " ".join(architectures), "Components: main", "Date: " + format_datetime(date, usegmt=True),
              "Valid-Until: " + format_datetime(date + LIFETIME, usegmt=True), "Acquire-By-Hash: yes", "SHA256:"]
    for architecture in architectures:
        for name in ["Packages", "Packages.gz"]:
            relative = f"main/binary-{architecture}/{name}"
            path = root / "dists" / SUITE / relative
            fields.append(f" {digest(path)} {path.stat().st_size} {relative}")
    return ("\n".join(fields) + "\n").encode()


def clock():
    return datetime.now(timezone.utc).replace(microsecond=0)


def prepare(packages, output, now=None):
    now = clock() if now is None else now
    require(1 <= len(packages) <= 2, "Expected one or two native packages")
    admitted = {}
    version = None
    for package in packages:
        regular(package, MAX_PACKAGE)
        regular(package.with_suffix(".deb.sha256"), 256)
        fields = native_deb.verify(package, smoke=False)
        require(not fields["Version"].endswith(".debug"), "Debug packages cannot enter the native channel")
        require(version in {None, fields["Version"]}, "Architectures must share one release version")
        version = fields["Version"]
        arch = fields["Architecture"]
        require(arch not in admitted, "Duplicate release architecture")
        admitted[arch] = package
    with stage(output) as root:
        pool = root / "pool/main/r/rquickshare-native"
        pool.mkdir(parents=True)
        for arch, source in sorted(admitted.items()):
            target = pool / f"rquickshare-native_{version}_{arch}.deb"
            shutil.copyfile(source, target)
            target.with_suffix(".deb.sha256").write_text(native_deb.checksum(target))
        for arch in sorted(admitted):
            directory = root / "dists" / SUITE / f"main/binary-{arch}"
            directory.mkdir(parents=True)
            data = index(root, arch)
            require(len(data) <= MAX_INDEX, "Index exceeds limit")
            for name, content in [("Packages", data), ("Packages.gz", gzip.compress(data, mtime=0))]:
                path = directory / name
                path.write_bytes(content)
                by_hash = directory / "by-hash/SHA256" / digest(path)
                by_hash.parent.mkdir(parents=True, exist_ok=True)
                by_hash.write_bytes(content)
        (root / "dists" / SUITE / "Release").write_bytes(release(root, sorted(admitted), now))
        validate_tree(root, now)


def inventory(root):
    require(not root.is_symlink() and root.is_dir(), "Expected repository directory")
    entries = []
    for number, path in enumerate(root.rglob("*")):
        require(number < 64 and not path.is_symlink() and (path.is_dir() or path.is_file()), "Unexpected repository link/special/entry count")
        entries.append(path)
    return entries


def validate_tree(root, now=None, signed=False, check_time=True):
    inventory(root)
    root = root.resolve(strict=True)
    base = root / "dists" / SUITE
    raw = read(base / "Release", 16384)
    parsed = paragraphs(raw)
    require(len(parsed) == 1, "Expected one Release stanza")
    fields = parsed[0]
    architectures = fields.get("Architectures", "").split(" ")
    require(architectures in [["amd64"], ["arm64"], ["amd64", "arm64"]], "Invalid channel architectures")
    try:
        date = parsedate_to_datetime(fields["Date"])
    except (KeyError, ValueError, TypeError, OverflowError) as error:
        raise ValueError("Invalid Release date") from error
    if check_time:
        now = clock() if now is None else now
        require(date.tzinfo is not None and date <= now + timedelta(seconds=10) and now < date + LIFETIME, "Expired or future repository metadata")
    expected = {f"dists/{SUITE}/Release"}
    packages = {}
    version = None
    for arch in architectures:
        directory = base / f"main/binary-{arch}"
        data = read(directory / "Packages")
        entries = paragraphs(data)
        require(len(entries) == 1, "Expected one indexed native package")
        entry = entries[0]
        native_deb.validate_metadata(entry)
        require(entry["Architecture"] == arch and not entry["Version"].endswith(".debug"), "Wrong native release architecture/profile")
        require(version in {None, entry["Version"]}, "Mixed channel versions")
        version = entry["Version"]
        name = f"pool/main/r/rquickshare-native/rquickshare_{version}_{arch}.deb"
        require(entry.get("Filename") == name, "Unexpected package path")
        package = root / name
        regular(package.with_suffix(".deb.sha256"), 256)
        package_digest = digest(package)
        require(entry.get("SHA256") == package_digest and entry.get("Size") == str(package.stat().st_size), "Indexed package hash/size mismatch")
        native_deb.verify(package, smoke=False)
        require(data == index(root, arch), "Package index differs from actual package metadata")
        packages[arch] = package_digest
        expected |= {name, name + ".sha256"}
        compressed = read(directory / "Packages.gz")
        require(compressed == gzip.compress(data, mtime=0), "Unexpected or corrupted compressed index")
        for filename in ["Packages", "Packages.gz"]:
            path = directory / filename
            relative = path.relative_to(root).as_posix()
            by_hash = f"{directory.relative_to(root).as_posix()}/by-hash/SHA256/{digest(path)}"
            require(read(root / by_hash) == read(path), "By-hash index differs")
            expected |= {relative, by_hash}
    require(raw == release(root, architectures, date), "Release fields/hash/size differ from actual indexes")
    if signed:
        expected |= {f"dists/{SUITE}/InRelease", f"dists/{SUITE}/Release.gpg"}
    seen = set()
    allowed_directories = {str(parent) for name in expected for parent in Path(name).parents}
    for path in inventory(root):
        name = path.relative_to(root).as_posix()
        if path.is_dir():
            require(name in allowed_directories, "Unexpected repository directory")
        else:
            require(path.is_file() and name in expected, "Unexpected repository file")
            seen.add(name)
    require(seen == expected, "Incomplete repository inventory")
    return {"version": version, "packages": packages}


def public_key(keyring, expected):
    fingerprint(expected)
    regular(keyring, MAX_INDEX)
    data = read(keyring)
    require(data[:1] and data[0] & 0x80, "Expected exported binary public keyring (.gpg)")
    with tempfile.TemporaryDirectory(prefix="rqs-public-key-") as temporary:
        text = run("gpg", "--no-options", "--homedir", temporary, "--no-autostart", "--batch", "--with-colons", "--show-keys", str(keyring.absolute()))
    primaries = []
    primary = False
    for line in text.splitlines():
        fields = line.split(":")
        if fields[0] == "pub":
            primary = True
        elif fields[0] == "sub":
            primary = False
        elif fields[0] == "fpr" and primary:
            primaries.append(fields[9])
            primary = False
    require(primaries == [expected], "Public keyring does not contain exactly the expected primary key")


def signatures(root, keyring, expected):
    public_key(keyring, expected)
    base = root / "dists" / SUITE
    read(base / "Release", 16384)
    for path in [base / "InRelease", base / "Release.gpg"]:
        regular(path, 32768)
        data = read(path, 32768)
        begin = b"-----BEGIN PGP SIGNED MESSAGE-----\n" if path.name == "InRelease" else b"-----BEGIN PGP SIGNATURE-----\n"
        require(data.startswith(begin) and data.endswith(b"-----END PGP SIGNATURE-----\n")
                and data.count(b"-----BEGIN PGP SIGNATURE-----") == 1
                and data.count(b"-----END PGP SIGNATURE-----") == 1, "Unexpected signature framing/trailing data")
    with tempfile.TemporaryDirectory(prefix="rqs-gpgv-") as temporary:
        cleartext = Path(temporary) / "Release"
        common = ["gpgv", "--homedir", temporary, "--keyring", str(keyring.absolute()), "--status-fd", "1"]
        statuses = [run(*common, "--output", str(cleartext), str(base / "InRelease")),
                    run(*common, str(base / "Release.gpg"), str(base / "Release"))]
        for status in statuses:
            valid = [line.split() for line in status.splitlines() if line.startswith("[GNUPG:] VALIDSIG ")]
            require(len(valid) == 1 and len(valid[0]) >= 11 and valid[0][9] in {"8", "9", "10"}
                    and (valid[0][2] == expected or valid[0][-1] == expected), "Unexpected release signer/hash algorithm")
        require(read(cleartext, 16384) == read(base / "Release", 16384), "InRelease cleartext differs from Release")


def compare_previous(current, previous):
    require(set(previous["packages"]) <= set(current["packages"]), "Channel removed a published architecture")
    subprocess.run(["dpkg", "--compare-versions", current["version"], "ge", previous["version"]], check=True)
    if current["version"] == previous["version"]:
        require(all(current["packages"][arch] == value for arch, value in previous["packages"].items()), "Same-version package bytes changed; increment native revision")


def verify(root, keyring, expected, now=None, previous=None, check_time=True):
    require(not root.is_symlink() and root.is_dir(), "Expected repository directory")
    root = root.resolve(strict=True)
    inventory(root)
    signatures(root, keyring, expected)
    current = validate_tree(root, now, signed=True, check_time=check_time)
    if previous is not None:
        compare_previous(current, verify(previous, keyring, expected, now, check_time=False))
    return current


def sign(source, output, home, keyring, expected, now=None, previous=None):
    require(not source.is_symlink() and source.is_dir(), "Expected unsigned snapshot directory")
    public_key(keyring, expected)
    require(home.is_dir() and not home.is_symlink(), "Expected explicitly selected GPG home")
    require(not output.absolute().is_relative_to(source.resolve()), "Signing output must be outside its input snapshot")
    current = validate_tree(source, now)
    if previous is not None:
        compare_previous(current, verify(previous, keyring, expected, now, check_time=False))
    with stage(output) as root:
        shutil.copytree(source, root, dirs_exist_ok=True)
        base = root / "dists" / SUITE
        command = ["gpg", "--no-options", "--homedir", str(home.absolute()), "--batch", "--no-tty", "--local-user", expected, "--digest-algo", "SHA256"]
        run(*command, "--output", str(base / "InRelease"), "--clearsign", str(base / "Release"))
        run(*command, "--armor", "--output", str(base / "Release.gpg"), "--detach-sign", str(base / "Release"))
        verify(root, keyring, expected, now, previous)


def bootstrap(repository_uri, repository, keyring, expected, output, now=None):
    repository_uri = uri(repository_uri)
    architectures = sorted(verify(repository, keyring, expected, now)["packages"])
    with stage(output) as root:
        shutil.copyfile(keyring, root / "rquickshare-native-archive-keyring.gpg")
        sources = f"Types: deb\nURIs: {repository_uri}\nSuites: {SUITE}\nComponents: main\nArchitectures: {' '.join(architectures)}\nBy-Hash: force\nSigned-By: /etc/apt/keyrings/rquickshare-native-archive-keyring.gpg {expected}\n"
        (root / "rquickshare-native.sources").write_text(sources)
        (root / "FINGERPRINT").write_text(expected + "\n")
        for path in root.iterdir():
            path.chmod(0o644)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    create = commands.add_parser("prepare", help="Create an explicitly unsigned immutable staging snapshot")
    create.add_argument("--package", type=Path, action="append", required=True)
    create.add_argument("--output", type=Path, required=True)
    for name in ["sign", "verify", "bootstrap"]:
        sub = commands.add_parser(name)
        sub.add_argument("--keyring", type=Path, required=True)
        sub.add_argument("--fingerprint", required=True)
        sub.add_argument("--repository", type=Path, required=True)
        if name != "bootstrap":
            if name == "sign":
                sub.add_argument("--gnupghome", type=Path, required=True)
                sub.add_argument("--output", type=Path, required=True)
                previous = sub.add_mutually_exclusive_group(required=True)
                previous.add_argument("--first-release", action="store_true")
                previous.add_argument("--previous", type=Path)
            else:
                sub.add_argument("--previous", type=Path)
        else:
            sub.add_argument("--uri", default=PRODUCTION_URI, help="HTTPS archive base (default: selected production prefix)")
            sub.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            prepare(args.package, args.output)
        elif args.command == "sign":
            sign(args.repository, args.output, args.gnupghome, args.keyring, args.fingerprint, previous=args.previous)
        elif args.command == "verify":
            verify(args.repository, args.keyring, args.fingerprint, previous=args.previous)
        else:
            bootstrap(args.uri, args.repository, args.keyring, args.fingerprint, args.output)
    except (ValueError, OSError, UnicodeError, subprocess.CalledProcessError) as error:
        # External tools may include arbitrary key/path data; do not print their output.
        print("Native APT operation failed:", type(error).__name__, file=sys.stderr)
        return 1
    print("Native APT operation complete:", args.command, "(no enrollment, installation or publication)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
