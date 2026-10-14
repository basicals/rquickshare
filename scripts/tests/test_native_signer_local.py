import json
import os
from pathlib import Path
import stat
import socket
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
import native_apt as apt
import native_signer as signer
import native_signer_local as local
import test_native_apt as fixtures


class LocalSigningTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        fixtures.NativeAptTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        fixtures.NativeAptTests.tearDownClass.__func__(cls)

    @staticmethod
    def listing():
        rows = []
        for kind, role, fingerprint, grip in [("pub", "cSC", "A" * 40, "1" * 40), ("sub", "s", "B" * 40, "2" * 40)]:
            row = [""] * 17
            row[0], row[1], row[3], row[6], row[11] = kind, "u", "22", "1900000000", role
            rows.extend([":".join(row), "fpr:::::::::" + fingerprint + ":", "grp:::::::::" + grip + ":"])
        return "\n".join(rows) + "\n"

    @staticmethod
    def pinentry(root, cancel=False):
        path = root / ("pinentry-cancel" if cancel else "pinentry-test")
        path.write_text("#!/usr/bin/python3\nimport sys\nprint('OK TEST ONLY',flush=True)\n"
                        "for line in sys.stdin:\n"
                        " if line.startswith('GETPIN'):\n"
                        + ("  print('ERR 83886179 Test cancellation',flush=True)\n" if cancel else
                           "  print('D Test-Only-Passphrase-123456789!',flush=True)\n  print('OK',flush=True)\n")
                        + " elif line.startswith('BYE'):\n  print('OK',flush=True)\n  break\n"
                          " else: print('OK',flush=True)\n")
        path.chmod(0o700)
        return path

    def assert_agent_closed(self, home):
        # gpg-connect-agent --no-autostart exits0 even without an agent. Check the
        # actual IPC endpoint instead of inventing an exit-status contract.
        path = subprocess.check_output(["gpgconf", "--homedir", str(home), "--list-dirs", "agent-socket"], text=True).strip()
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            with self.assertRaises((FileNotFoundError, ConnectionRefusedError)):
                client.connect(path)

    def test_given_public_roles_when_parsed_then_exact_primary_and_subkey_identity_are_required(self):
        # Arrange / Act / Assert: independent public listing/protocol contracts.
        text = self.listing()
        expected = local.identity(text, now=1800000000)
        self.assertEqual(expected["primary"]["fingerprint"], "A" * 40)
        self.assertEqual(expected["signing_subkey"]["keygrip"], "2" * 40)
        self.assertEqual(set(local.identity("\n".join(text.splitlines()[:3]), complete=False, now=1800000000)), {"primary"})
        cases = ["", text[:40], text + text, text.replace("cSC", "sSC"), text.replace("cSC", "csSC"),
                 text.replace(":s:", ":e:"), text.replace(":22:", ":1:"), text.replace("1900000000", "1800000000"),
                 text.replace("1900000000", "0"), text.replace("1900000000", "bad"), text.replace("pub:u:", "pub:r:"),
                 text.replace("B" * 40, "A" * 40), text.replace("2" * 40, "1" * 40),
                 text.replace("A" * 40, "a" * 40), text.replace("1" * 40, "G" * 40),
                 text.replace("grp:::::::::" + "1" * 40 + ":\n", ""),
                 "fpr:::::::::" + "A" * 40 + ":\n" + text, text + "bad:record\n", "x" * 65537]
        for value in cases:
            with self.subTest(value=value[:60]), self.assertRaises(ValueError):
                local.identity(value, now=1800000000)

    def test_given_agent_keyinfo_when_keys_are_clear_missing_or_extra_then_protection_is_rejected(self):
        row = "S KEYINFO " + "1" * 40 + " D - - - P - - -\n"
        self.assertIsNone(local.protected_keys(row + "OK\n", {"1" * 40}))
        for value in ["", row, "OK\n", row + row + "OK\n", row.replace(" D ", " T ") + "OK\n",
                      row.replace(" P ", " C ") + "OK\n", row.replace(" P ", " - ") + "OK\n",
                      row.replace("1" * 40, "2" * 40) + "OK\n", row + "ERR 1 failure\n",
                      "S KEYINFO broken\nOK\n", row.replace("1" * 40, "x" * 40) + "OK\n", "x" * 65537]:
            with self.subTest(value=value[:60]), self.assertRaises(ValueError):
                local.protected_keys(value, {"1" * 40})

    def test_given_state_paths_when_existing_noncanonical_or_insecure_then_initialization_refuses_without_overwrite(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            local.fresh_state(root / "valid")
            (root / "occupied").mkdir()
            (root / "link").symlink_to(root / "absent")
            (root / "parent-link").symlink_to(root, target_is_directory=True)
            insecure = root / "insecure"
            insecure.mkdir(mode=0o777)
            insecure.chmod(0o777)
            for path in [root / "occupied", root / "link", root / "parent-link/new", Path("relative"),
                         root / "absent/new", insecure / "new", local.ROOT / "unused-signing-state"]:
                with self.subTest(path=path), self.assertRaises(ValueError):
                    local.fresh_state(path)
            private = root / "private"
            private.mkdir(mode=0o700)
            local.directory(private)
            private.chmod(0o755)
            with self.assertRaises(ValueError):
                local.directory(private)
            target = root / "file"
            local.write_private(target, b"original")
            self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o600)
            with self.assertRaises(FileExistsError):
                local.write_private(target, b"changed")
            self.assertEqual(target.read_bytes(), b"original")

    def test_given_real_protected_keys_when_local_custody_and_approved_release_are_created_then_only_subkey_signs(self):
        with tempfile.TemporaryDirectory(prefix="rqs-local-signing-test-") as temporary:
            root = Path(temporary)
            state = root / "custody"
            pinentry = self.pinentry(root)
            try:
                value = local.initialize(state, pinentry)
                self.assertEqual(value, local.inspect(state))
                self.assertFalse((state / ".subkeys.gpg").exists())
                for name in ["authority", "signer"]:
                    # Successful initialization closed only its exclusively owned agents.
                    self.assert_agent_closed(state / name)
                primary = value["identity"]["primary"]["fingerprint"]
                keyring = state / "archive-keyring.gpg"
                approved = signer.prepare(self.unsigned, keyring, primary, "a" * 40, root / "request", now=self.now)
                signer.sign(self.unsigned, keyring, primary, "a" * 40, root / "request/request.json", approved,
                            state / "signer", root / "signed", now=self.now)
                apt.verify(root / "signed", keyring, primary, now=self.now)
                status = apt.run("gpgv", "--homedir", str(root), "--keyring", str(keyring), "--status-fd", "1",
                                 str(root / "signed/dists" / apt.SUITE / "InRelease"))
                signature = next(line.split() for line in status.splitlines() if line.startswith("[GNUPG:] VALIDSIG "))
                self.assertEqual(signature[2], value["identity"]["signing_subkey"]["fingerprint"])
                self.assertEqual(signature[-1], primary)
                self.assertNotEqual(signature[2], primary)
                # Read-only inspection rejects layout/public drift and unsafe permissions.
                metadata = state / "identity.json"
                original = metadata.read_bytes()
                changed = json.loads(original)
                changed["schema"] = 2
                metadata.write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    local.inspect(state)
                metadata.write_bytes(original)
                (state / "signer/private-keys-v1.d/extra.key").write_text("extra")
                with self.assertRaises(ValueError):
                    local.inspect(state)
                (state / "signer/private-keys-v1.d/extra.key").unlink()
                metadata.chmod(0o644)
                with self.assertRaises(ValueError):
                    local.inspect(state)
                metadata.chmod(0o600)
                public = keyring.read_bytes()
                keyring.write_bytes(public + b"changed")
                with self.assertRaises(ValueError):
                    local.inspect(state)
                keyring.write_bytes(public)
                with self.assertRaises(ValueError):
                    local.initialize(state, pinentry)
            finally:
                # Test owns these fresh profiles and all later test signing processes.
                for name in ["authority", "signer"]:
                    if (state / name).exists():
                        subprocess.run(["gpgconf", "--homedir", str(state / name), "--kill", "gpg-agent"], check=True)

    def test_given_cancelled_pinentry_when_initializing_then_partial_private_state_is_preserved_without_readiness(self):
        with tempfile.TemporaryDirectory(prefix="rqs-local-cancel-test-") as temporary:
            root = Path(temporary)
            state = root / "custody"
            with self.assertRaises(subprocess.CalledProcessError):
                local.initialize(state, self.pinentry(root, cancel=True))
            self.assertTrue(state.is_dir())
            self.assertEqual(stat.S_IMODE(state.stat().st_mode), 0o700)
            self.assertFalse((state / "identity.json").exists())
            self.assertFalse((state / ".subkeys.gpg").exists())
            for name in ["authority", "signer"]:
                self.assert_agent_closed(state / name)


if __name__ == "__main__":
    unittest.main()
