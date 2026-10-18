#!/usr/bin/env python3
"""Recover the authenticated prior APT snapshot from a dedicated private R2 archive."""
import argparse
import hashlib
from pathlib import Path
import re
import sys

import native_apt as apt
import native_r2 as r2

MAX_SNAPSHOT_OBJECTS = 64
PACKAGE = re.compile(r"pool/main/r/rquickshare-native/rquickshare-native_(?:0|[1-9][0-9]*)\\.(?:0|[1-9][0-9]*)\\.(?:0|[1-9][0-9]*)-[1-9][0-9]{0,9}_(?:amd64|arm64)\\.deb(?:\\.sha256)?")

def admitted(key):
    prefix = f"dists/{apt.SUITE}/"
    if PACKAGE.fullmatch(key):
        return True
    if not key.startswith(prefix):
        return False
    item = key[len(prefix):]
    if item in {"Release", "Release.gpg", "InRelease"}:
        return True
    return re.fullmatch(r"main/binary-(?:amd64|arm64)/(?:Packages(?:\\.gz)?|by-hash/SHA256/[a-f0-9]{64})", item) is not None

def read_object(client, key, maximum):
    try:
        response = client.get_object(Bucket=r2.BUCKET, Key=key)
    except Exception as error:
        raise r2.Unavailable("Archived object read unavailable") from None
    body = response["Body"]
    try:
        size = response["ContentLength"]
        apt.require(type(size) is int and 0 <= size <= maximum, "Archived object exceeds accepted bound")
        pieces, received = [], 0
        for chunk in body.iter_chunks(chunk_size=65536):
            received += len(chunk)
            apt.require(received <= size, "Archived object exceeds declared length")
            pieces.append(chunk)
        apt.require(received == size, "Truncated archived object")
        return b"".join(pieces)
    finally:
        body.close()

def restore(store, output, keyring, fingerprint):
    """True for a verified existing channel; False only for confirmed missing InRelease."""
    current = store.inspect(r2.ACTIVE, 32768, retain=True)
    if current is None:
        return False
    apt.require(current.data is not None and hashlib.sha256(current.data).hexdigest() == current.sha256,
                "Unverified active repository readback")
    prefix = f"snapshots/{current.sha256}/"
    try:
        listing = store.client.list_objects_v2(Bucket=r2.BUCKET, Prefix=prefix, MaxKeys=MAX_SNAPSHOT_OBJECTS + 1)
    except Exception:
        raise r2.Unavailable("Private snapshot inventory unavailable") from None
    apt.require(not listing.get("IsTruncated", False), "Archived snapshot exceeds bounded inventory")
    entries = listing.get("Contents", [])
    apt.require(1 <= len(entries) <= MAX_SNAPSHOT_OBJECTS, "Incomplete or oversized archived snapshot")
    names = [entry["Key"] for entry in entries]
    apt.require(len(names) == len(set(names)) and all(isinstance(x, str) and x.startswith(prefix)
                and admitted(x[len(prefix):]) for x in names), "Unsafe or unexpected snapshot key")
    with apt.stage(output) as folder:
        for name in names:
            relative = name[len(prefix):]
            maximum = apt.MAX_PACKAGE if relative.endswith(".deb") else apt.MAX_INDEX
            data = read_object(store.client, name, maximum)
            target = folder / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        apt.require((folder / r2.ACTIVE).read_bytes() == current.data,
                    "Snapshot does not match active repository generation")
        apt.verify(folder, keyring, fingerprint, check_time=False)
    return True

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--account-id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--keyring", type=Path, required=True)
    parser.add_argument("--fingerprint", required=True)
    args = parser.parse_args()
    store = None
    try:
        apt.require(not args.output.exists(), "Existing snapshot output is forbidden")
        apt.public_key(args.keyring, args.fingerprint)
        store = r2.connect(args.account_id)
        has_previous = restore(store, args.output, args.keyring, args.fingerprint)
        print("previous" if has_previous else "first")
        return 0
    except Exception as error:
        print("Prior release recovery failed:", type(error).__name__, file=sys.stderr)
        return 1
    finally:
        if store is not None:
            store.close()

if __name__ == "__main__":
    sys.exit(main())
