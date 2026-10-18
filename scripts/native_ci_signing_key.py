#!/usr/bin/env python3
"""Fail closed unless the CI signing key is one stub primary plus one signing subkey."""
import argparse
from pathlib import Path
import re
import sys

EXPECTED_SUBKEY = "1F5150D98D94C43109FBA2503E7DE744C1014357"


def validate_listing(data, primary, subkey=EXPECTED_SUBKEY):
    if not re.fullmatch(r"[A-F0-9]{40}", primary) or not re.fullmatch(r"[A-F0-9]{40}", subkey):
        raise ValueError("Invalid pinned signing identity")
    records = [line.split(":") for line in data.splitlines() if line]
    if not records or any(line[0] not in {"sec", "fpr", "ssb", "grp", "uid"} for line in records):
        raise ValueError("Unexpected secret-key inventory record")
    # GPG includes a keygrip and user ID alongside the actual key/fingerprint
    # records. These do not grant signing authority; retain strict order of
    # the only two permitted key records and forbid extra subkeys.
    keys = [line for line in records if line[0] in {"sec", "ssb", "fpr"}]
    if [line[0] for line in keys] != ["sec", "fpr", "ssb", "fpr"]:
        raise ValueError("Unexpected private key inventory")
    stage = 0
    for record in records:
        if record[0] == "sec":
            if stage != 0:
                raise ValueError("Unexpected secret primary")
            stage = 1
        elif record[0] == "fpr":
            if stage not in {1, 3}:
                raise ValueError("Unexpected fingerprint")
            stage += 1
        elif record[0] == "ssb":
            if stage != 2:
                raise ValueError("Unexpected signing subkey")
            stage = 3
        elif record[0] == "uid" and stage != 2:
            raise ValueError("Unexpected user ID placement")
        elif record[0] == "grp" and stage not in {2, 4}:
            raise ValueError("Unexpected keygrip placement")
    if stage != 4 or len(keys[0]) <= 14 or keys[0][14] != "#":
        raise ValueError("Certification primary secret must not enter CI")
    if len(keys[2]) <= 14 or keys[2][14] != "+" or "s" not in keys[2][11]:
        raise ValueError("Exactly one usable signing subkey is required")
    if keys[1][9] != primary or keys[3][9] != subkey:
        raise ValueError("Protected key does not match pinned public authority")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--listing", type=Path, required=True)
    parser.add_argument("--fingerprint", required=True)
    args = parser.parse_args()
    try:
        validate_listing(args.listing.read_text(encoding="utf-8"), args.fingerprint)
        return 0
    except (ValueError, OSError) as error:
        print("CI signing identity rejected:", type(error).__name__, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
