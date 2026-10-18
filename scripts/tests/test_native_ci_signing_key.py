from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
import native_ci_signing_key as ci

PRIMARY = "6D8854762CF0C86881138A56695A0888D09FA616"
SUBKEY = ci.EXPECTED_SUBKEY
STUB = "sec:-:255:22:AAAA:0:0::-:::cSC:::#::ed25519:::0:"
SUB = "ssb:-:255:22:BBBB:0:0:::::s:::+::ed25519::"
LISTING = "\n".join([STUB, f"fpr:::::::::{PRIMARY}:", SUB, f"fpr:::::::::{SUBKEY}:"])


class CISecretSubkeyTests(unittest.TestCase):
    def test_given_subkey_only_export_when_validated_then_exact_pinned_identity_is_accepted(self):
        ci.validate_listing(LISTING, PRIMARY)

    def test_given_usable_primary_or_wrong_signing_key_when_validated_then_fail_closed(self):
        for corrupted in [
            LISTING.replace(":::#::", ":::+::"),
            LISTING.replace(SUBKEY, PRIMARY),
            LISTING.replace(PRIMARY, SUBKEY, 1),
            LISTING.replace(":::::s:::+::", ":::::e:::+::"),
            LISTING.replace(":::::s:::+::", ":::::s:::#::"),
            LISTING + "\n" + SUB,
            LISTING.replace("ssb:", "sec:"),
            LISTING.replace("fpr:::::::::", "sig:::::::::", 1),
            "",
        ]:
            with self.subTest(value=corrupted[-90:]), self.assertRaises(ValueError):
                ci.validate_listing(corrupted, PRIMARY)


if __name__ == "__main__":
    unittest.main()
