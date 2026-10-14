#!/usr/bin/env python3
"""Public signing-request handoff; run approval/sign only on the isolated signer."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

import native_apt as apt


def request(snapshot, keyring, fingerprint, source_commit, previous=None, now=None):
    apt.require(re.fullmatch(r"(?:[a-f0-9]{40}|[a-f0-9]{64})", source_commit), "Expected full source commit")
    apt.public_key(keyring, fingerprint)
    current = apt.validate_tree(snapshot, now)
    previous_digest = None
    if previous is not None:
        apt.compare_previous(current, apt.verify(previous, keyring, fingerprint, now, check_time=False))
        previous_digest = apt.digest(previous / "dists" / apt.SUITE / "InRelease")
    files = [{"path": path.relative_to(snapshot).as_posix(), "size": path.stat().st_size, "sha256": apt.digest(path)}
             for path in sorted(apt.inventory(snapshot)) if path.is_file()]
    value = {"schema": 1, "suite": apt.SUITE, "source_commit": source_commit, "primary_fingerprint": fingerprint,
             "previous_inrelease_sha256": previous_digest, "files": files}
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def prepare(snapshot, keyring, fingerprint, source_commit, output, previous=None, now=None):
    data = request(snapshot, keyring, fingerprint, source_commit, previous, now)
    with apt.stage(output) as root:
        (root / "request.json").write_bytes(data)
        (root / "request.sha256").write_text(hashlib.sha256(data).hexdigest() + "  request.json\n")
    return hashlib.sha256(data).hexdigest()


def approved_request(snapshot, keyring, fingerprint, source_commit, manifest, approved, previous=None, now=None):
    apt.require(re.fullmatch(r"[a-f0-9]{64}", approved), "Expected independently approved request SHA256")
    data = apt.read(manifest, 65536)
    apt.require(hashlib.sha256(data).hexdigest() == approved, "Request differs from approved digest")
    apt.require(data == request(snapshot, keyring, fingerprint, source_commit, previous, now), "Candidate/source/authority/baseline differs from approved request")


def sign(snapshot, keyring, fingerprint, source_commit, manifest, approved, home, output, previous=None, now=None):
    # The caller supplies approval, not an automatic grant from untrusted CI metadata.
    approved_request(snapshot, keyring, fingerprint, source_commit, manifest, approved, previous, now)
    with apt.stage(output) as outer:
        signed = outer / "signed"
        apt.sign(snapshot, signed, home, keyring, fingerprint, now, previous)
        # Bind the copied unsigned bytes as well, before publishing the local result.
        apt.require(all(apt.digest(signed / path.relative_to(snapshot)) == apt.digest(path)
                        and (signed / path.relative_to(snapshot)).stat().st_size == path.stat().st_size
                        for path in apt.inventory(snapshot) if path.is_file()), "Signed output differs from approved input")
        approved_request(snapshot, keyring, fingerprint, source_commit, manifest, approved, previous, now)
        # Publish the standard snapshot directly, without approval files in its validated tree.
        for path in signed.iterdir():
            path.rename(outer / path.name)
        signed.rmdir()
    return approved


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["request", "sign"])
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--keyring", type=Path, required=True)
    parser.add_argument("--fingerprint", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output", type=Path, required=True)
    baseline = parser.add_mutually_exclusive_group(required=True)
    baseline.add_argument("--first-release", action="store_true")
    baseline.add_argument("--previous", type=Path)
    parser.add_argument("--request", type=Path)
    parser.add_argument("--approve-sha256")
    parser.add_argument("--gnupghome", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "request":
            apt.require(args.request is None and args.approve_sha256 is None and args.gnupghome is None, "Request preparation takes no signing credentials or approval")
            value = prepare(args.repository, args.keyring, args.fingerprint, args.source_commit, args.output, args.previous)
        else:
            apt.require(args.request is not None and args.approve_sha256 is not None and args.gnupghome is not None, "Sign requires an independently reviewed request and explicit signer home")
            value = sign(args.repository, args.keyring, args.fingerprint, args.source_commit, args.request,
                         args.approve_sha256, args.gnupghome, args.output, args.previous)
    except Exception as error:
        print("Native signing handoff failed:", type(error).__name__, file=sys.stderr)
        return 1
    print("Native signing request SHA256:", value)
    return 0


if __name__ == "__main__":
    sys.exit(main())
