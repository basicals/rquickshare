#!/usr/bin/env python3
"""Encrypt, verify and restore bounded local signing recovery; never upload or release."""
import argparse
from contextlib import contextmanager
import ctypes
import hashlib
import io
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import tarfile
import tempfile

import native_apt as apt
import native_signer_local as local

MAX_ARCHIVE = 4 * 1024 * 1024
MAX_CIPHERTEXT = MAX_ARCHIVE + 65536
MAX_MEMBER = 1024 * 1024


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


@contextmanager
def private_stage(output):
    """Keep recovery output private through an atomic, no-replace directory commit."""
    local.fresh_state(output)
    with tempfile.TemporaryDirectory(prefix=".rqs-recovery-output-", dir=output.parent) as temporary:
        root = Path(temporary) / "backup"
        root.mkdir(mode=0o700)
        yield root
        rename = ctypes.CDLL(None, use_errno=True).renameat2
        rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
        rename.restype = ctypes.c_int
        if rename(-100, os.fsencode(root), -100, os.fsencode(output), 1) != 0:
            raise OSError(ctypes.get_errno(), "Cannot commit private recovery output")


def paths(metadata, expected):
    apt.fingerprint(expected)
    apt.require(set(metadata) == {"schema", "custody", "identity", "public_keyring_sha256"}
                and type(metadata["schema"]) is int and metadata["schema"] == 1
                and metadata["custody"] == "same-linux-workstation", "Invalid custody metadata")
    keys = metadata["identity"]
    apt.require(set(keys) == {"primary", "signing_subkey"}, "Invalid recovery identity")
    for name, kind in [("primary", "pub"), ("signing_subkey", "sub")]:
        key = keys[name]
        apt.require(set(key) == {"kind", "expires", "fingerprint", "keygrip"} and key["kind"] == kind
                    and re.fullmatch(r"[A-F0-9]{40}", key["keygrip"]), "Invalid recovery key role")
        apt.fingerprint(key["fingerprint"])
    apt.require(keys["primary"]["fingerprint"] == expected, "Recovery primary differs from trusted fingerprint")
    primary, sub = keys["primary"]["keygrip"], keys["signing_subkey"]["keygrip"]
    apt.require(primary != sub, "Recovery keygrips overlap")
    return {"identity.json", "archive-keyring.gpg", f"authority/openpgp-revocs.d/{expected}.rev",
            f"authority/private-keys-v1.d/{primary}.key", f"authority/private-keys-v1.d/{sub}.key",
            f"signer/private-keys-v1.d/{sub}.key"}


def require_idle(home):
    address = subprocess.check_output(["gpgconf", "--homedir", str(home), "--list-dirs", "agent-socket"], text=True).strip()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        try:
            client.connect(address)
        except (FileNotFoundError, ConnectionRefusedError):
            return
    raise ValueError("Source GPG profile has an active agent; ownership is external to backup")


def collect(state):
    metadata = local.inspect(state)
    for name in ["authority", "signer"]:
        require_idle(state / name)
    local.directory(state / "authority/openpgp-revocs.d")
    expected = metadata["identity"]["primary"]["fingerprint"]
    result = {name: apt.read(state / name, MAX_MEMBER) for name in paths(metadata, expected)}
    apt.require(json.loads(result["identity.json"]) == metadata and local.inspect(state) == metadata, "Source custody changed")
    return result, expected


def bundle(values, expected):
    apt.require(set(values) == paths(json.loads(values["identity.json"]), expected)
                and all(0 < len(value) <= MAX_MEMBER for value in values.values()), "Invalid recovery inventory/size")
    manifest = {"schema": 1, "primary_fingerprint": expected, "files": {
        name: {"size": len(value), "sha256": hashlib.sha256(value).hexdigest()} for name, value in values.items()}}
    files = values | {"manifest.json": canonical(manifest)}
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        for name, value in sorted(files.items()):
            info = tarfile.TarInfo(name)
            info.mode, info.size = 0o600, len(value)
            archive.addfile(info, io.BytesIO(value))
    result = buffer.getvalue()
    apt.require(len(result) <= MAX_ARCHIVE, "Recovery archive exceeds limit")
    return result


def unpack(data, expected):
    apt.fingerprint(expected)
    apt.require(0 < len(data) <= MAX_ARCHIVE, "Invalid recovery archive size")
    values = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:") as archive:
        for member in archive:
            apt.require(len(values) < 7 and member.name not in values and member.isfile()
                        and not member.pax_headers and member.mode == 0o600 and 0 < member.size <= MAX_MEMBER
                        and not member.name.startswith("/") and "\\" not in member.name
                        and all(part not in {"", ".", ".."} for part in member.name.split("/")), "Unsafe recovery member")
            stream = archive.extractfile(member)
            apt.require(stream is not None, "Missing recovery content")
            with stream:
                values[member.name] = stream.read(MAX_MEMBER + 1)
            apt.require(len(values[member.name]) == member.size, "Truncated recovery member")
    apt.require("manifest.json" in values and "identity.json" in values, "Missing recovery manifest")
    manifest = json.loads(values.pop("manifest.json"))
    apt.require(set(manifest) == {"schema", "primary_fingerprint", "files"}
                and type(manifest["schema"]) is int and manifest["schema"] == 1 and manifest["primary_fingerprint"] == expected
                and set(manifest["files"]) == set(values), "Recovery manifest identity/inventory changed")
    for name, value in values.items():
        apt.require(manifest["files"][name] == {"size": len(value), "sha256": hashlib.sha256(value).hexdigest()}, "Recovery content digest changed")
    # Canonical roundtrip also excludes hidden GNU/PAX headers, padding/trailing bytes and alternate manifests.
    apt.require(data == bundle(values, expected), "Noncanonical recovery container")
    return values


@contextmanager
def crypto_home(parent, pinentry=None):
    with tempfile.TemporaryDirectory(prefix=".rqs-backup-crypto-", dir=parent) as temporary:
        home = Path(temporary)
        config = local.AGENT_CONFIG
        if pinentry is not None:
            apt.require(pinentry.is_file() and pinentry.resolve() == pinentry
                        and re.fullmatch(r"/[A-Za-z0-9/_.+-]+", str(pinentry)), "Unsafe test Pinentry")
            config += f"pinentry-program {pinentry}\n"
        local.write_private(home / "gpg-agent.conf", config.encode())
        try:
            yield home
        finally:
            subprocess.run(["gpgconf", "--homedir", str(home), "--kill", "gpg-agent"], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def crypt(home, data, encrypt):
    apt.require(0 < len(data) <= (MAX_ARCHIVE if encrypt else MAX_CIPHERTEXT), "Cryptographic input exceeds limit")
    command = ["gpg", "--no-options", "--homedir", str(home), "--batch", "--no-tty", "--pinentry-mode", "ask",
               "--no-symkey-cache", "--max-output", str(MAX_CIPHERTEXT if encrypt else MAX_ARCHIVE), "--set-filename", ""]
    command += (["--cipher-algo", "AES256", "--s2k-digest-algo", "SHA256", "--s2k-count", "65011712",
                 "--compress-algo", "none", "--symmetric"] if encrypt else ["--decrypt"])
    result = subprocess.run(command, input=data, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout
    apt.require(0 < len(result) <= (MAX_CIPHERTEXT if encrypt else MAX_ARCHIVE), "Cryptographic output exceeds limit")
    return result


def restore_values(values, expected, output, pinentry=None):
    bundle(values, expected)  # Validate every name/size before constructing any destination.
    local.fresh_state(output)
    output.mkdir(mode=0o700)
    homes = []
    try:
        for name in ["authority", "signer"]:
            home = output / name
            home.mkdir(mode=0o700)
            homes.append(home)
            (home / "private-keys-v1.d").mkdir(mode=0o700)
            config = local.AGENT_CONFIG
            if pinentry is not None:
                apt.require(pinentry.resolve() == pinentry and re.fullmatch(r"/[A-Za-z0-9/_.+-]+", str(pinentry)), "Unsafe test Pinentry")
                config += f"pinentry-program {pinentry}\n"
            local.write_private(home / "gpg-agent.conf", config.encode())
        (output / "authority/openpgp-revocs.d").mkdir(mode=0o700)
        for name, value in values.items():
            local.write_private(output / name, value)
        apt.require(all(apt.read(output / name, MAX_MEMBER) == value for name, value in values.items()), "Restored bytes differ before GPG use")
        metadata = json.loads(values["identity.json"])
        apt.require(set(values) == paths(metadata, expected), "Restore inventory changed")
        for home in homes:
            local.Gpg(home).run("--import", str(output / "archive-keyring.gpg"))
            subprocess.run(["gpgconf", "--homedir", str(home), "--launch", "gpg-agent"], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        verified = local.inspect(output)
        for name, required in [("authority", {key["keygrip"] for key in verified["identity"].values()}),
                               ("signer", {verified["identity"]["signing_subkey"]["keygrip"]})]:
            local.protected_keys(local.Gpg(output / name).keyinfo(), required)
        with tempfile.TemporaryDirectory(prefix=".rqs-recovery-proof-", dir=output.parent) as temporary:
            proof = Path(temporary)
            message, signature = proof / "challenge", proof / "challenge.sig"
            local.write_private(message, b"rQuickShare restored-key verification; NOT A RELEASE OR PUBLICATION APPROVAL\n")
            subkey = verified["identity"]["signing_subkey"]["fingerprint"]
            local.Gpg(output / "signer").run("--local-user", subkey + "!", "--digest-algo", "SHA256",
                                           "--output", str(signature), "--detach-sign", str(message))
            status = apt.run("gpgv", "--homedir", str(proof), "--keyring", str(output / "archive-keyring.gpg"),
                             "--status-fd", "1", str(signature), str(message))
            valid = [line.split() for line in status.splitlines() if line.startswith("[GNUPG:] VALIDSIG ")]
            apt.require(len(valid) == 1 and valid[0][2] == subkey and valid[0][-1] == expected and valid[0][9] == "8", "Restored key proof differs")
            # GPG may rewrap its internal private-file format when unlocking. The
            # material must still sign as the same subkey and remain protected.
            local.protected_keys(local.Gpg(output / "signer").keyinfo(), {verified["identity"]["signing_subkey"]["keygrip"]})
            # This public-only revocation check cannot revoke the active custody.
            with crypto_home(proof) as revoke:
                local.Gpg(revoke).run("--import", str(output / "archive-keyring.gpg"))
                certificate = values[f"authority/openpgp-revocs.d/{expected}.rev"]
                apt.require(certificate.count(b":-----BEGIN PGP PUBLIC KEY BLOCK-----") == 1, "Invalid recovery revocation certificate")
                revocation = proof / "revocation.asc"
                local.write_private(revocation, certificate.replace(b":-----BEGIN PGP PUBLIC KEY BLOCK-----", b"-----BEGIN PGP PUBLIC KEY BLOCK-----", 1))
                local.Gpg(revoke).run("--import", str(revocation))
                rows = [line.split(":") for line in local.Gpg(revoke).listing().splitlines()]
                apt.require([row[1] for row in rows if row[0] == "pub"] == ["r"], "Recovery revocation is unusable")
        return {"schema": 1, "primary_fingerprint": expected, "signing_subkey_fingerprint": subkey,
                "protected_profiles_verified": True, "non_release_signature_verified": True, "revocation_verified": True}
    except BaseException:
        (output / "identity.json").unlink(missing_ok=True)
        raise
    finally:
        for home in reversed(homes):
            subprocess.run(["gpgconf", "--homedir", str(home), "--kill", "gpg-agent"], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def verify(ciphertext, expected, parent, pinentry=None, output=None):
    with crypto_home(parent, pinentry) as home:
        values = unpack(crypt(home, ciphertext, False), expected)
        if output is not None:
            return restore_values(values, expected, output, pinentry)
        with tempfile.TemporaryDirectory(prefix=".rqs-recovery-check-", dir=parent) as temporary:
            return restore_values(values, expected, Path(temporary) / "custody", pinentry)


def create(state, output, pinentry=None):
    local.fresh_state(output)
    values, expected = collect(state)
    # Secret-bearing temporary profiles stay next to the private source custody,
    # even when the caller's final ciphertext destination is a mounted cloud folder.
    with crypto_home(state, pinentry) as home:
        ciphertext = crypt(home, bundle(values, expected), True)
    receipt = verify(ciphertext, expected, state, pinentry)
    digest = hashlib.sha256(ciphertext).hexdigest()
    filename = f"rquickshare-signing-recovery-{expected[:12]}-{digest}.tar.gpg"
    receipt.update({"ciphertext_sha256": digest, "ciphertext_size": len(ciphertext), "filename": filename,
                    "cloud_copy_verified": False, "release_signed": False})
    # No plaintext or private profile is written to the final output directory.
    with private_stage(output) as root:
        local.write_private(root / filename, ciphertext)
        local.write_private(root / (filename + ".sha256"), (digest + "  " + filename + "\n").encode())
        local.write_private(root / "receipt.json", (json.dumps(receipt, indent=2) + "\n").encode())
        instructions = ("# rQuickShare encrypted signing recovery\n\n"
                        "Keep the archive password and original key password available separately.\n"
                        "The archive checksum detects damage; independently confirm the primary fingerprint.\n\n"
                        f"Trusted primary fingerprint: {expected}\n\n"
                        "From an audited rQuickShare source checkout, verify the downloaded archive:\n\n"
                        "```sh\npython3 scripts/native_signer_backup.py verify \\\n"
                        f"  --archive /absolute/path/{filename} \\\n  --fingerprint {expected}\n```\n\n"
                        "To restore, use the restore command with the same archive/fingerprint and\n"
                        "--output /absolute/fresh/private-state. It refuses existing state, verifies\n"
                        "the restored signing key and revocation in temporary profiles, and does not\n"
                        "install packages, publish a release or revoke active custody. Enter passwords\n"
                        "only through local Pinentry. Never upload restored private profiles.\n")
        local.write_private(root / "RESTORE.md", instructions.encode())
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["create", "verify", "restore"])
    parser.add_argument("--state-dir", type=Path, default=Path.home() / ".rquickshare-signing")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--fingerprint")
    args = parser.parse_args()
    try:
        if args.command == "create":
            apt.require(args.output is not None and args.archive is None and args.fingerprint is None, "Create requires only a fresh output")
            result = create(args.state_dir, args.output)
        else:
            apt.require(args.archive is not None and args.fingerprint is not None
                        and ((args.command == "restore") == (args.output is not None)), "Verify/restore requires archive and trusted fingerprint")
            with tempfile.TemporaryDirectory(prefix="rqs-recovery-operation-") as temporary:
                result = verify(apt.read(args.archive, MAX_CIPHERTEXT), args.fingerprint, Path(temporary), output=args.output)
    except Exception as error:
        print("Signing recovery operation failed:", type(error).__name__, file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
