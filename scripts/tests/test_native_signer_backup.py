import io
import json
from pathlib import Path
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
import native_signer_backup as backup
import native_signer_local as local
import test_native_signer_local as local_tests


class SigningBackupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="rqs-backup-test-")
        cls.root = Path(cls.temporary.name)
        cls.pinentry = local_tests.LocalSigningTests.pinentry(cls.root)
        cls.state = cls.root / "source"
        try:
            cls.identity = local.initialize(cls.state, cls.pinentry)
            cls.values, cls.primary = backup.collect(cls.state)
        except BaseException:
            cls.tearDownClass()
            raise

    @classmethod
    def tearDownClass(cls):
        for name in ["authority", "signer"]:
            if (cls.state / name).exists():
                subprocess.run(["gpgconf", "--homedir", str(cls.state / name), "--kill", "gpg-agent"], check=True)
        cls.temporary.cleanup()

    @staticmethod
    def archive(data, mutate):
        result = io.BytesIO()
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:") as source, tarfile.open(fileobj=result, mode="w", format=tarfile.PAX_FORMAT) as target:
            for i, original in enumerate(source):
                value = source.extractfile(original).read()
                member, content = mutate(i, original, value)
                if member is not None:
                    target.addfile(member, io.BytesIO(content) if member.isfile() else None)
        return result.getvalue()

    def test_given_fixed_recovery_inventory_when_serialized_then_roundtrip_is_exact_and_untrusted_members_fail(self):
        # Arrange / Act / Assert: real protected fixture inventory, not mocked custody logic.
        data = backup.bundle(self.values, self.primary)
        self.assertEqual(backup.unpack(data, self.primary), self.values)
        self.assertEqual(len(self.values), 6)
        self.assertNotIn(b"pinentry-program", data)
        for label, edit in [
            ("absolute", lambda i, m, v: (setattr(m, "name", "/outside") or m, v) if i == 0 else (m, v)),
            ("traversal", lambda i, m, v: (setattr(m, "name", "../outside") or m, v) if i == 0 else (m, v)),
            ("backslash", lambda i, m, v: (setattr(m, "name", "unsafe\\outside") or m, v) if i == 0 else (m, v)),
            ("mode", lambda i, m, v: (setattr(m, "mode", 0o644) or m, v) if i == 0 else (m, v)),
            ("link", lambda i, m, v: (setattr(m, "type", tarfile.SYMTYPE) or m, v) if i == 0 else (m, v)),
            ("directory", lambda i, m, v: (setattr(m, "type", tarfile.DIRTYPE) or m, v) if i == 0 else (m, v)),
            ("hardlink", lambda i, m, v: (setattr(m, "type", tarfile.LNKTYPE) or m, v) if i == 0 else (m, v)),
            ("fifo", lambda i, m, v: (setattr(m, "type", tarfile.FIFOTYPE) or m, v) if i == 0 else (m, v)),
            ("oversized-member", lambda i, m, v: (setattr(m, "size", backup.MAX_MEMBER + 1) or m, bytes(backup.MAX_MEMBER + 1)) if i == 0 else (m, v)),
            ("empty-member", lambda i, m, v: (setattr(m, "size", 0) or m, b"") if i == 0 else (m, v)),
            ("missing", lambda i, m, v: (None, v) if i == 0 else (m, v)),
            ("changed", lambda i, m, v: (m, b"x" * len(v)) if i == 0 else (m, v)),
            ("pax", lambda i, m, v: (setattr(m, "pax_headers", {"comment": "untrusted"}) or m, v) if i == 0 else (m, v)),
            ("duplicate", lambda i, m, v: (setattr(m, "name", "identity.json") or m, v) if i == 0 else (m, v)),
        ]:
            with self.subTest(kind=label), self.assertRaises((ValueError, tarfile.TarError)):
                backup.unpack(self.archive(data, edit), self.primary)
        for value in [b"", b"not a tar", data[:511], data + b"trailing", b"x" * (backup.MAX_ARCHIVE + 1)]:
            with self.subTest(size=len(value)), self.assertRaises((ValueError, tarfile.TarError)):
                backup.unpack(value, self.primary)
        with self.assertRaises(ValueError):
            backup.unpack(data, "A" * 40)
        for values in [self.values | {"extra": b"x"}, self.values | {"identity.json": b"{}"},
                       self.values | {"archive-keyring.gpg": b""}, self.values | {"archive-keyring.gpg": b"x" * (backup.MAX_MEMBER + 1)}]:
            with self.assertRaises(ValueError):
                backup.bundle(values, self.primary)
        for schema in [True, 1.0, "1", 0, 2, None]:
            metadata = json.loads(self.values["identity.json"])
            metadata["schema"] = schema
            with self.subTest(schema=schema), self.assertRaises(ValueError):
                backup.bundle(self.values | {"identity.json": json.dumps(metadata).encode()}, self.primary)

    def test_given_unsafe_restore_or_concurrent_output_when_attempted_then_no_foreign_file_is_written_or_replaced(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for values in [self.values | {"../outside": b"secret"}, self.values | {"identity.json": b"{}"}]:
                with self.assertRaises(ValueError):
                    backup.restore_values(values, self.primary, root / "restore", self.pinentry)
                self.assertFalse((root / "restore").exists())
                self.assertFalse((root.parent / "outside").exists())
            target = root / "output"
            with self.assertRaises(OSError):
                with backup.private_stage(target) as staged:
                    local.write_private(staged / "ours", b"ours")
                    target.mkdir(mode=0o700)
                    local.write_private(target / "theirs", b"theirs")
            self.assertEqual([p.name for p in target.iterdir()], ["theirs"])
            self.assertEqual((target / "theirs").read_bytes(), b"theirs")
            target2 = root / "safe"
            with backup.private_stage(target2) as staged:
                local.write_private(staged / "ciphertext", b"encrypted")
            self.assertEqual(stat.S_IMODE(target2.stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE((target2 / "ciphertext").stat().st_mode), 0o600)
            self.assertFalse(any(p.name.startswith(".rqs-recovery-output-") for p in root.iterdir()))

    def test_given_an_external_source_agent_when_collected_then_backup_refuses_without_stopping_it(self):
        home = self.state / "authority"
        subprocess.run(["gpgconf", "--homedir", str(home), "--launch", "gpg-agent"], check=True)
        try:
            with self.assertRaises(ValueError):
                backup.collect(self.state)
            result = subprocess.run(["gpg-connect-agent", "--homedir", str(home), "--no-autostart", "KEYINFO --list", "/bye"], check=True, capture_output=True)
            self.assertIn(b"S KEYINFO", result.stdout)
        finally:
            # This test exclusively owns this source agent, not the backup operation.
            subprocess.run(["gpgconf", "--homedir", str(home), "--kill", "gpg-agent"], check=True)

    def test_given_actual_encrypted_backup_when_verified_and_restored_then_protected_keys_signature_and_revocation_work(self):
        before = dict(self.values)
        output = self.root / "backup"
        receipt = backup.create(self.state, output, self.pinentry)
        ciphertext = (output / receipt["filename"]).read_bytes()
        self.assertNotIn(b"private-keys-v1.d", ciphertext)
        self.assertFalse(receipt["cloud_copy_verified"])
        self.assertTrue(receipt["revocation_verified"])
        self.assertTrue(receipt["non_release_signature_verified"])
        self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o700)
        for path in output.iterdir():
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
        restored = self.root / "restored"
        result = backup.verify(ciphertext, self.primary, self.root, self.pinentry, restored)
        self.assertTrue(result["protected_profiles_verified"])
        self.assertEqual(local.inspect(restored), self.identity)
        self.assertEqual(backup.collect(self.state)[0], before)
        restored_values = backup.collect(restored)[0]
        # Unlocking may convert GPG's own protected file representation. Exact
        # pre-use bytes, post-use protection and exact signing identity are enforced.
        rewrapped = f"signer/private-keys-v1.d/{self.identity['identity']['signing_subkey']['keygrip']}.key"
        self.assertEqual({name: data for name, data in restored_values.items() if name != rewrapped},
                         {name: data for name, data in before.items() if name != rewrapped})
        for name in ["authority", "signer"]:
            local_tests.LocalSigningTests.assert_agent_closed(self, restored / name)
        changed = ciphertext[:-1] + bytes([ciphertext[-1] ^ 1])
        with self.assertRaises(subprocess.CalledProcessError):
            backup.verify(changed, self.primary, self.root, self.pinentry, self.root / "tampered-restore")
        self.assertFalse((self.root / "tampered-restore").exists())
        with self.assertRaises(ValueError):
            backup.verify(ciphertext, "A" * 40, self.root, self.pinentry, self.root / "wrong-primary")
        self.assertFalse((self.root / "wrong-primary").exists())
        wrong = self.root / "wrong-pinentry"
        wrong.write_text(self.pinentry.read_text().replace("Test-Only-Passphrase-123456789!", "Different-Test-Passphrase-98765!"))
        wrong.chmod(0o700)
        with self.assertRaises(subprocess.CalledProcessError):
            backup.verify(ciphertext, self.primary, self.root, wrong, self.root / "wrong-password")
        self.assertFalse((self.root / "wrong-password").exists())
        self.assertFalse(any(p.name.startswith((".rqs-backup-crypto-", ".rqs-recovery-check-", ".rqs-recovery-proof-")) for p in self.root.rglob("*")))

    def test_given_cancelled_password_or_oversized_input_when_encrypting_then_no_ready_backup_or_source_change_occurs(self):
        before = dict(self.values)
        cancel = local_tests.LocalSigningTests.pinentry(self.root, cancel=True)
        with self.assertRaises(subprocess.CalledProcessError):
            backup.create(self.state, self.root / "cancelled", cancel)
        self.assertFalse((self.root / "cancelled").exists())
        self.assertEqual(backup.collect(self.state)[0], before)
        for encrypt, data in [(True, b""), (False, b""), (True, bytes(backup.MAX_ARCHIVE + 1)), (False, bytes(backup.MAX_CIPHERTEXT + 1))]:
            with self.subTest(encrypt=encrypt, size=len(data)), self.assertRaises(ValueError):
                backup.crypt(self.root / "no-home-created", data, encrypt)


if __name__ == "__main__":
    unittest.main()
