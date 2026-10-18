from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
import native_ci_signing_key as ci
import native_apt as apt
import test_native_apt as fixtures

PRIMARY = "6D8854762CF0C86881138A56695A0888D09FA616"
SUBKEY = ci.EXPECTED_SUBKEY
STUB = "sec:-:255:22:AAAA:0:0::-:::cSC:::#::ed25519:::0:"
SUB = "ssb:-:255:22:BBBB:0:0:::::s:::+::ed25519::"
LISTING = "\n".join([STUB, f"fpr:::::::::{PRIMARY}:", SUB, f"fpr:::::::::{SUBKEY}:"])


class CISecretSubkeyTests(unittest.TestCase):
    def test_given_subkey_only_export_when_validated_then_exact_pinned_identity_is_accepted(self):
        ci.validate_listing(LISTING, PRIMARY)
        # GPG --with-colons emits optional keygrips and user IDs.
        listing = "\\n".join([
            STUB, f"fpr:::::::::{PRIMARY}:", "grp:::::::::PRIMARY_KEYGRIP:",
            "uid:::::::::CI TEST <fixture@invalid.example>:", SUB,
            f"fpr:::::::::{SUBKEY}:", "grp:::::::::SUBKEY_KEYGRIP:",
        ])
        ci.validate_listing(listing, PRIMARY)

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
            LISTING + "\\n" + "grp:::::::::unexpected-keygrip:",
            LISTING.replace("ssb:", "uid:", 1),
            "",
        ]:
            with self.subTest(value=corrupted[-90:]), self.assertRaises(ValueError):
                ci.validate_listing(corrupted, PRIMARY)


class ProtectedSubkeySignatureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        fixtures.NativeAptTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        fixtures.NativeAptTests.tearDownClass.__func__(cls)

    def test_given_exported_secret_subkey_and_private_passphrase_when_signing_then_real_gpgv_accepts_and_insecure_passphrase_is_rejected(self):
        with tempfile.TemporaryDirectory(prefix="rqs-protected-ci-fixture-") as temporary:
            root = Path(temporary)
            authority, worker = root / "authority", root / "worker"
            authority.mkdir(mode=0o700)
            worker.mkdir(mode=0o700)
            password = "test-only-secure-passphrase-12345"
            common = ["gpg", "--no-options", "--homedir", str(authority), "--batch",
                      "--pinentry-mode", "loopback", "--passphrase", password]
            try:
                subprocess.run([*common, "--quick-generate-key", "CI TEST ONLY <fixture@invalid.example>",
                                "ed25519", "cert", "1d"], check=True, capture_output=True)
                listing = subprocess.check_output(["gpg", "--no-options", "--homedir", str(authority),
                                                    "--batch", "--with-colons", "--list-keys"], text=True)
                primary = next(line.split(":")[9] for line in listing.splitlines() if line.startswith("fpr:"))
                subprocess.run([*common, "--quick-add-key", primary, "ed25519", "sign", "1d"],
                               check=True, capture_output=True)
                keyring = root / "public.gpg"
                with keyring.open("wb") as output:
                    subprocess.run(["gpg", "--no-options", "--homedir", str(authority),
                                    "--batch", "--export", primary], check=True, stdout=output)
                private = root / "subkeys.gpg"
                with private.open("wb") as output:
                    subprocess.run([*common, "--export-secret-subkeys", primary], check=True,
                                   stdout=output, stderr=subprocess.PIPE)
                subprocess.run(["gpg", "--no-options", "--homedir", str(worker), "--batch",
                                "--import", str(private)], check=True, capture_output=True)
                value = subprocess.check_output(["gpg", "--no-options", "--homedir", str(worker),
                                                 "--batch", "--with-colons", "--list-secret-keys"],
                                                text=True)
                subkey = [line.split(":")[9] for line in value.splitlines() if line.startswith("fpr:")][1]
                ci.validate_listing(value, primary, subkey)
                passfile = root / "passphrase"
                passfile.write_text(password)
                passfile.chmod(0o600)
                signed = root / "signed"
                apt.sign(self.unsigned, signed, worker, keyring, primary, self.now, passphrase_file=passfile)
                self.assertEqual(apt.verify(signed, keyring, primary, self.now)["version"], "1.0.0-1")
                passfile.chmod(0o644)
                with self.assertRaises(ValueError):
                    apt.sign(self.unsigned, root / "unsafe", worker, keyring, primary, self.now,
                             passphrase_file=passfile)
                self.assertFalse((root / "unsafe").exists())
            finally:
                for home in (authority, worker):
                    subprocess.run(["gpgconf", "--homedir", str(home), "--kill", "gpg-agent"],
                                   check=False, capture_output=True)


if __name__ == "__main__":
    unittest.main()
