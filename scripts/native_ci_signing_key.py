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
    if [line[0] for line in records] != ["sec", "fpr", "ssb", "fpr"]:
        raise ValueError("Unexpected private key inventory")
    if len(records[0]) <= 14 or records[0][14] != "#":
        raise ValueError("Certification primary secret must not enter CI")
    if len(records[2]) <= 14 or records[2][14] != "+" or "s" not in records[2][11]:
        raise ValueError("Exactly one usable signing subkey is required")
    if records[1][9] != primary or records[3][9] != subkey:
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
