#!/usr/bin/env python3
"""Provision/inspect manual local signing custody; never sign or publish a release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
import time

import native_apt as apt

ROOT = Path(__file__).resolve().parents[1]
UID = "rQuickShare Native Archive"
AGENT_CONFIG = ("enforce-passphrase-constraints\nmin-passphrase-len 16\n"
                "no-allow-loopback-pinentry\nno-allow-external-cache\n"
                "default-cache-ttl 60\nmax-cache-ttl 300\n")


def identity(text, complete=True, now=None):
    """Parse public GPG --with-colons/--with-keygrip output, never secret packets."""
    apt.require(len(text) <= 65536, "Key listing exceeds limit")
    keys = []
    current = None
    for line in text.splitlines():
        row = line.split(":")
        if row[0] in {"pub", "sub"}:
            apt.require(len(row) >= 12 and row[1] not in {"r", "e", "d", "i"}
                        and row[3] == "22" and row[6].isdigit()
                        and int(row[6]) > (time.time() if now is None else now), "Invalid/expired key")
            capabilities = {letter for letter in row[11] if letter.islower()}
            apt.require(capabilities == ({"c"} if row[0] == "pub" else {"s"}), "Unexpected key role")
            current = {"kind": row[0], "expires": int(row[6])}
            keys.append(current)
        elif row[0] in {"fpr", "grp"}:
            apt.require(current is not None and len(row) >= 10, "Unbound key identity")
            field = "fingerprint" if row[0] == "fpr" else "keygrip"
            apt.require(field not in current, "Duplicate key identity")
            if field == "fingerprint":
                apt.fingerprint(row[9])
            else:
                apt.require(re.fullmatch(r"[A-F0-9]{40}", row[9]), "Invalid keygrip")
            current[field] = row[9]
        else:
            apt.require(row[0] in {"tru", "uid"}, "Unexpected key listing record")
    expected = ["pub", "sub"] if complete else ["pub"]
    apt.require([key["kind"] for key in keys] == expected
                and all(set(key) == {"kind", "expires", "fingerprint", "keygrip"} for key in keys), "Incomplete/extra key identity")
    apt.require(len({key["fingerprint"] for key in keys}) == len(keys)
                and len({key["keygrip"] for key in keys}) == len(keys), "Repeated key identity")
    return {"primary": keys[0], **({"signing_subkey": keys[1]} if complete else {})}


def protected_keys(text, expected):
    """Require exact passphrase-protected local disk key set from agent KEYINFO."""
    apt.require(len(text) <= 65536, "Agent listing exceeds limit")
    lines = text.splitlines()
    apt.require(lines and lines[-1] == "OK", "Agent listing did not complete")
    seen = set()
    for line in lines[:-1]:
        fields = line.split()
        apt.require(len(fields) >= 8 and fields[:2] == ["S", "KEYINFO"]
                    and re.fullmatch(r"[A-F0-9]{40}", fields[2])
                    and fields[3] == "D" and fields[7] == "P"
                    and fields[2] not in seen, "Missing/unsafe private key protection")
        seen.add(fields[2])
    apt.require(seen == set(expected), "Unexpected usable secret keys")


def directory(path):
    apt.require(path.is_absolute() and path.resolve() == path and path.is_dir()
                and path.stat().st_uid == os.getuid()
                and stat.S_IMODE(path.stat().st_mode) == 0o700, "Expected caller-owned private canonical directory")


def fresh_state(path):
    apt.require(path.is_absolute() and path.resolve() == path and not path.exists()
                and not path.is_symlink() and not path.is_relative_to(ROOT), "Expected fresh private state outside repository")
    parent = path.parent
    apt.require(parent.is_dir() and parent.resolve() == parent and parent.stat().st_uid == os.getuid()
                and not stat.S_IMODE(parent.stat().st_mode) & 0o022, "Unsafe state parent")


def write_private(path, data):
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600), "wb") as stream:
        stream.write(data)


class Gpg:
    def __init__(self, home):
        self.home = home

    def run(self, *args, output=None):
        command = ["gpg", "--no-options", "--homedir", str(self.home), "--batch", "--no-tty", "--pinentry-mode", "ask", *args]
        result = subprocess.run(command, check=True, stdout=output or subprocess.PIPE, stderr=subprocess.PIPE)
        return result.stdout.decode() if output is None else None

    def listing(self):
        return self.run("--with-colons", "--with-fingerprint", "--with-subkey-fingerprint", "--with-keygrip", "--list-keys")

    def keyinfo(self):
        return subprocess.run(["gpg-connect-agent", "--homedir", str(self.home), "--no-autostart", "KEYINFO --list", "/bye"],
                              check=True, capture_output=True, text=True).stdout


def inspect(state):
    """Read public identity and disk layout without starting or stopping an agent."""
    directory(state)
    for name in ["authority", "signer"]:
        directory(state / name)
        directory(state / name / "private-keys-v1.d")
    metadata = state / "identity.json"
    apt.require(stat.S_IMODE(metadata.lstat().st_mode) == 0o600 and not metadata.is_symlink(), "Unsafe identity file")
    value = json.loads(apt.read(metadata, 16384))
    apt.require(set(value) == {"schema", "custody", "identity", "public_keyring_sha256"}
                and value["schema"] == 1 and value["custody"] == "same-linux-workstation", "Unexpected local custody metadata")
    exported = state / "archive-keyring.gpg"
    apt.require(apt.digest(exported) == value["public_keyring_sha256"], "Public keyring changed")
    keys = value["identity"]
    apt.require(set(keys) == {"primary", "signing_subkey"}, "Invalid custody identity")
    # show-keys is public-only, isolated and explicitly cannot autostart an agent.
    with tempfile.TemporaryDirectory(prefix="rqs-local-public-") as temporary:
        listing = apt.run("gpg", "--no-options", "--homedir", temporary, "--no-autostart", "--batch",
                          "--with-colons", "--with-fingerprint", "--with-subkey-fingerprint", "--with-keygrip", "--show-keys", str(exported))
    apt.require(identity(listing) == keys, "Public key identity differs from custody metadata")
    apt.public_key(exported, keys["primary"]["fingerprint"])
    for home, expected in [("authority", {key["keygrip"] for key in keys.values()}),
                           ("signer", {keys["signing_subkey"]["keygrip"]})]:
        files = list((state / home / "private-keys-v1.d").iterdir())
        apt.require({path.name for path in files} == {grip + ".key" for grip in expected}, "Unexpected private key layout")
        for path in files:
            info = path.lstat()
            apt.require(stat.S_ISREG(info.st_mode) and stat.S_IMODE(info.st_mode) == 0o600
                        and info.st_uid == os.getuid(), "Unsafe private key file")
    return value


def initialize(state, pinentry=None):
    """Only this fresh command owns these agents; preserve partial identity on failure."""
    fresh_state(state)
    if pinentry is not None:
        # Explicit dependency for an owned integration-test Pinentry, never a CLI password bypass.
        apt.require(pinentry.is_file() and pinentry.resolve() == pinentry
                    and re.fullmatch(r"/[A-Za-z0-9/_.+-]+", str(pinentry)), "Unsafe test Pinentry path")
    state.mkdir(mode=0o700)
    homes = []
    temporary_export = state / ".subkeys.gpg"
    try:
        for name in ["authority", "signer"]:
            home = state / name
            home.mkdir(mode=0o700)
            homes.append(home)
            config = AGENT_CONFIG + (f"pinentry-program {pinentry}\n" if pinentry is not None else "")
            write_private(home / "gpg-agent.conf", config.encode())
        authority, signer = (Gpg(home) for home in homes)
        authority.run("--quick-generate-key", UID, "ed25519", "cert", "5y")
        primary = identity(authority.listing(), complete=False)["primary"]
        protected_keys(authority.keyinfo(), {primary["keygrip"]})
        authority.run("--quick-add-key", primary["fingerprint"], "ed25519", "sign", "1y")
        keys = identity(authority.listing())
        protected_keys(authority.keyinfo(), {key["keygrip"] for key in keys.values()})
        with os.fdopen(os.open(state / "archive-keyring.gpg", os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600), "wb") as stream:
            authority.run("--export-options", "export-minimal", "--export", primary["fingerprint"], output=stream)
        with os.fdopen(os.open(temporary_export, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600), "wb") as stream:
            authority.run("--export-secret-subkeys", primary["fingerprint"], output=stream)
        signer.run("--import", str(temporary_export))
        apt.require(identity(signer.listing()) == keys, "Imported signer identity changed")
        protected_keys(signer.keyinfo(), {keys["signing_subkey"]["keygrip"]})
        value = {"schema": 1, "custody": "same-linux-workstation", "identity": keys,
                 "public_keyring_sha256": apt.digest(state / "archive-keyring.gpg")}
        write_private(state / "identity.json", (json.dumps(value, sort_keys=True, indent=2) + "\n").encode())
        inspect(state)
        return value
    except BaseException:
        # A partial generated identity is recoverable; no readiness claim survives failure.
        (state / "identity.json").unlink(missing_ok=True)
        raise
    finally:
        temporary_export.unlink(missing_ok=True)
        for home in reversed(homes):
            subprocess.run(["gpgconf", "--homedir", str(home), "--kill", "gpg-agent"], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["initialize", "inspect"])
    parser.add_argument("--state-dir", type=Path, default=Path.home() / ".local/share/rquickshare-signing")
    args = parser.parse_args()
    try:
        value = initialize(args.state_dir) if args.command == "initialize" else inspect(args.state_dir)
    except Exception as error:
        print("Local signing custody operation failed:", type(error).__name__, file=sys.stderr)
        return 1
    print("Public identity/layout verified; primary fingerprint:", value["identity"]["primary"]["fingerprint"])
    print("No release signature, publication, enrollment or installation performed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
