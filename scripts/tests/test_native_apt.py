from datetime import timedelta
import hashlib
import os
from pathlib import Path
import pwd
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
import native_apt as apt
import test_native_deb as deb_test


class NativeAptTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="rqs-apt-tests-")
        cls.root = Path(cls.temporary.name)
        cls.home = cls.root / "gnupg"
        cls.home.mkdir(mode=0o700)
        try:
            cls.keys = []
            for number in [1, 2]:
                identity = f"rQuickShare isolated TEST ONLY {number} <test{number}@invalid.example>"
                apt.run("gpg", "--no-options", "--homedir", str(cls.home), "--batch", "--pinentry-mode", "loopback", "--passphrase", "",
                        "--quick-generate-key", identity, "ed25519", "sign", "0")
                listed = apt.run("gpg", "--no-options", "--homedir", str(cls.home), "--batch", "--with-colons", "--list-keys", identity)
                fingerprint = next(line.split(":")[9] for line in listed.splitlines() if line.startswith("fpr:"))
                keyring = cls.root / f"test-only-{number}.gpg"
                with keyring.open("wb") as target:
                    subprocess.run(["gpg", "--no-options", "--homedir", str(cls.home), "--batch", "--export-options", "export-minimal", "--export", fingerprint], stdout=target, check=True)
                cls.keys.append((keyring, fingerprint))
            cls.keyring, cls.fingerprint = cls.keys[0]
            cls.now = apt.clock()
            fixture = cls.root / "fixture"
            fixture.mkdir()
            cls.package = deb_test.NativeDebTests().create(fixture)
            cls.unsigned = cls.root / "unsigned"
            apt.prepare([cls.package], cls.unsigned, cls.now)
            cls.signed = cls.root / "signed"
            apt.sign(cls.unsigned, cls.signed, cls.home, cls.keyring, cls.fingerprint, cls.now)
        except BaseException:
            cls.tearDownClass()
            raise

    @classmethod
    def tearDownClass(cls):
        # This agent was created exclusively with the isolated test GNUPGHOME above.
        subprocess.run(["gpgconf", "--homedir", str(cls.home), "--kill", "gpg-agent"], check=True)
        cls.temporary.cleanup()

    def copy(self, root, source=None):
        target = root / "repository"
        shutil.copytree(source or self.signed, target)
        return target

    def test_given_uri_fingerprint_or_metadata_when_malformed_then_injection_and_ambient_trust_are_rejected(self):
        # Arrange / Act / Assert: table-driven pure boundary contracts.
        self.assertEqual(apt.uri("https://updates.invalid.example/native/"), "https://updates.invalid.example/native")
        self.assertEqual(apt.fingerprint(self.fingerprint), self.fingerprint)
        for value in ["", "http://updates.example", "file:///tmp/apt", "https://user:password@updates.example", "https://u@updates.example",
                      "https://updates.example/?a=b", "https://updates.example/#key", "https://updates.example/?", "https://updates.example/#", "https://@updates.example", "https://updates.example/../apt", "https://updates.example/%0Aapt",
                      "https://updates.example\\evil", "https://updates.example\nTrusted: yes", "https://updates.example:0", "https://updates.example:65536", "https://updates.example:", "https://\"evil", "https://bad..example", "https://" + "a" * 2048]:
            with self.subTest(uri=value), self.assertRaises(ValueError):
                apt.uri(value)
        for value in ["", "A" * 39, "A" * 41, "A" * 63, "A" * 65, "a" * 40, "G" * 40, self.fingerprint + "\n", None]:
            with self.subTest(fingerprint=value), self.assertRaises(ValueError):
                apt.fingerprint(value)
        for data in [b"", b"Bad", b"Package: one\nPackage: two\n", b" continuation\n", b"Package: one\x00\n", b"Package: one\r\n", b"x" * (apt.MAX_INDEX + 1)]:
            with self.subTest(metadata=data[:40]), self.assertRaises(ValueError):
                apt.paragraphs(data)

    def test_given_actual_signed_snapshot_when_verified_then_time_identity_and_scoped_bootstrap_are_exact(self):
        # Arrange / Act / Assert: actual gpgv verifies both signatures and cleartext.
        result = apt.verify(self.signed, self.keyring, self.fingerprint, self.now)
        self.assertEqual(result["version"], "0.11.5+native1")
        self.assertEqual(set(result["packages"]), {"amd64"})
        for now, valid in [(self.now - timedelta(seconds=10), True), (self.now - timedelta(seconds=11), False),
                           (self.now + apt.LIFETIME - timedelta(seconds=1), True), (self.now + apt.LIFETIME, False)]:
            with self.subTest(now=now):
                if valid:
                    apt.validate_tree(self.signed, now, signed=True)
                else:
                    with self.assertRaises(ValueError):
                        apt.validate_tree(self.signed, now, signed=True)
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "bootstrap"
            apt.bootstrap("https://updates.invalid.example/native", self.signed, self.keyring, self.fingerprint, output, self.now)
            self.assertEqual((output / "FINGERPRINT").read_text(), self.fingerprint + "\n")
            self.assertEqual((output / "rquickshare-native-archive-keyring.gpg").read_bytes(), self.keyring.read_bytes())
            text = (output / "rquickshare-native.sources").read_text()
            self.assertIn("By-Hash: force\n", text)
            self.assertIn(f"Signed-By: /etc/apt/keyrings/rquickshare-native-archive-keyring.gpg {self.fingerprint}\n", text)
            self.assertNotIn("Trusted:", text)
            self.assertNotIn("Allow-Insecure", text)
            self.assertEqual({path.name for path in output.iterdir()}, {"FINGERPRINT", "rquickshare-native.sources", "rquickshare-native-archive-keyring.gpg"})
            with self.assertRaises(ValueError):
                apt.bootstrap("https://updates.invalid.example", self.signed, self.keyring, self.fingerprint, output, self.now)

    def test_given_signed_snapshot_when_key_signature_index_pool_or_paths_are_tampered_then_verification_fails(self):
        # Arrange / Act / Assert: never mock the verifier or signature authority.
        wrong_keyring, wrong_fingerprint = self.keys[1]
        for keyring, fingerprint in [(wrong_keyring, wrong_fingerprint), (self.keyring, wrong_fingerprint)]:
            with self.subTest(key=fingerprint), self.assertRaises((ValueError, subprocess.CalledProcessError)):
                apt.verify(self.signed, keyring, fingerprint, self.now)
        base = Path("dists") / apt.SUITE
        index = base / "main/binary-amd64"
        pool = Path("pool/main/r/rquickshare/rquickshare_0.11.5+native1_amd64.deb")
        for name in [base / "Release", base / "InRelease", base / "Release.gpg", index / "Packages", index / "Packages.gz", pool, Path(str(pool) + ".sha256")]:
            with self.subTest(file=name), tempfile.TemporaryDirectory() as temporary:
                root = self.copy(Path(temporary))
                path = root / name
                path.write_bytes(path.read_bytes() + b"tampered")
                with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    apt.verify(root, self.keyring, self.fingerprint, self.now)
        for kind in ["by-hash", "extra", "missing", "symlink", "directory-link", "fifo"]:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                root = self.copy(Path(temporary))
                if kind == "by-hash":
                    next((root / index / "by-hash/SHA256").iterdir()).write_bytes(b"wrong")
                elif kind == "extra":
                    (root / "unexpected").write_bytes(b"extra")
                elif kind == "missing":
                    (root / index / "Packages.gz").unlink()
                elif kind == "symlink":
                    (root / "outside").symlink_to(self.package)
                elif kind == "directory-link":
                    shutil.rmtree(root / "pool")
                    (root / "pool").symlink_to(self.signed / "pool", target_is_directory=True)
                else:
                    os.mkfifo(root / "special")
                with self.assertRaises((ValueError, OSError, subprocess.CalledProcessError)):
                    apt.verify(root, self.keyring, self.fingerprint, self.now)

    def test_given_invalid_package_selection_or_output_when_preparing_then_partial_and_concurrent_snapshots_are_not_published(self):
        # Arrange / Act / Assert: real package verification and atomic no-replace.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for packages in [[], [self.package, self.package], [self.package] * 3, [root / "missing.deb"]]:
                with self.subTest(packages=len(packages)), self.assertRaises((ValueError, OSError)):
                    apt.prepare(packages, root / "invalid", self.now)
                self.assertFalse((root / "invalid").exists())
            for version in ["0.11.5+native2.debug", "0.11.5"]:
                fixture = root / version
                fixture.mkdir()
                package = deb_test.NativeDebTests().create(fixture, lambda tree: self.change_version(tree, version))
                with self.subTest(version=version), self.assertRaises(ValueError):
                    apt.prepare([package], root / "invalid", self.now)
                self.assertFalse((root / "invalid").exists())
            target = root / "concurrent"
            with self.assertRaises(OSError):
                with apt.stage(target) as staging:
                    (staging / "ours").write_text("owned")
                    target.mkdir()
                    (target / "theirs").write_text("concurrent")
            self.assertEqual([path.name for path in target.iterdir()], ["theirs"])
            self.assertFalse(any(path.name.startswith(".rqs-apt-") for path in root.iterdir()))
            with self.assertRaises(ValueError):
                apt.sign(self.unsigned, self.unsigned / "recursive", self.home, self.keyring, self.fingerprint, self.now)

    @staticmethod
    def change_version(tree, version):
        control = tree / "DEBIAN/control"
        control.write_text(control.read_text().replace("Version: 0.11.5+native1\n", f"Version: {version}\n"))

    def test_given_two_architectures_when_staged_then_indexes_remain_separate_and_mixed_versions_or_architecture_removal_fail(self):
        # Arrange: metadata/ELF-header fixture only, never an executable ARM build claim.
        def arm_fixture(tree):
            control = tree / "DEBIAN/control"
            control.write_text(control.read_text().replace("Architecture: amd64\n", "Architecture: arm64\n"))
            for name in ["usr/bin/rquickshare", "usr/libexec/rquickshare-daemon"]:
                path = tree / name
                data = bytearray(path.read_bytes())
                data[18:20] = (183).to_bytes(2, "little")
                path.write_bytes(data)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = root / "arm"
            fixture.mkdir()
            arm = deb_test.NativeDebTests().create(fixture, arm_fixture)
            unsigned = root / "unsigned"
            apt.prepare([arm, self.package], unsigned, self.now)
            signed = root / "signed"
            apt.sign(unsigned, signed, self.home, self.keyring, self.fingerprint, self.now, self.signed)
            current = apt.verify(signed, self.keyring, self.fingerprint, self.now)
            self.assertEqual(set(current["packages"]), {"amd64", "arm64"})
            with self.assertRaises(ValueError):
                apt.verify(self.signed, self.keyring, self.fingerprint, self.now, signed)
            mixed = root / "mixed"
            mixed.mkdir()
            other = deb_test.NativeDebTests().create(mixed, lambda tree: (arm_fixture(tree), self.change_version(tree, "0.11.5+native2")))
            with self.assertRaises(ValueError):
                apt.prepare([self.package, other], root / "invalid", self.now)
            self.assertFalse((root / "invalid").exists())

    def test_given_previous_authenticated_snapshot_when_refresh_upgrade_drift_or_downgrade_then_revision_order_and_bytes_are_enforced(self):
        # Arrange / Act / Assert: same release refresh is safe; changed bytes require a new revision.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            refresh = root / "refresh"
            apt.sign(self.unsigned, refresh, self.home, self.keyring, self.fingerprint, self.now, self.signed)
            self.assertEqual(apt.verify(refresh, self.keyring, self.fingerprint, self.now, self.signed)["version"], "0.11.5+native1")
            for version, success in [("0.11.5+native1", False), ("0.11.5+native2", True), ("0.11.4+native9", False)]:
                fixture = root / version
                fixture.mkdir()
                package = deb_test.NativeDebTests().create(fixture, lambda tree: (self.change_version(tree, version), (tree / "usr/bin/rquickshare").write_bytes(Path("/bin/true").read_bytes() + b"new release")))
                unsigned = root / (version + "-unsigned")
                signed = root / (version + "-signed")
                apt.prepare([package], unsigned, self.now)
                if success:
                    apt.sign(unsigned, signed, self.home, self.keyring, self.fingerprint, self.now, self.signed)
                    self.assertEqual(apt.verify(signed, self.keyring, self.fingerprint, self.now, self.signed)["version"], version)
                else:
                    with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                        apt.sign(unsigned, signed, self.home, self.keyring, self.fingerprint, self.now, self.signed)
                    self.assertFalse(signed.exists())

    def test_given_real_apt_with_isolated_state_when_signed_channel_is_acquired_then_candidate_and_download_match_and_wrong_key_fails(self):
        self.assert_apt_acquisition(self.signed.as_uri())

    def assert_apt_acquisition(self, repository_uri):
        # Arrange: no host apt config, trust, status, hooks, lists or installation.
        with tempfile.TemporaryDirectory(prefix="rqs-apt-client-") as temporary:
            root = Path(temporary)
            for relative in ["etc/apt", "state/lists/partial", "cache/archives/partial", "log", "downloads"]:
                (root / relative).mkdir(parents=True)
            (root / "state/status").write_text("")
            keyring = root / "keyring.gpg"
            shutil.copyfile(self.keyring, keyring)
            (root / "etc/apt/sources.sources").write_text(f"Types: deb\nURIs: {repository_uri}\nSuites: {apt.SUITE}\nComponents: main\nArchitectures: amd64\nBy-Hash: force\nSigned-By: {keyring} {self.fingerprint}\n")
            config = root / "apt.conf"
            config.write_text(f'''Dir "{root}";
Dir::Etc "etc/apt";
Dir::Etc::main "-";
Dir::Etc::parts "-";
Dir::Etc::sourcelist "sources.sources";
Dir::Etc::sourceparts "-";
Dir::Etc::trusted "trusted.gpg";
Dir::Etc::trustedparts "-";
Dir::State "state";
Dir::State::status "{root}/state/status";
Dir::State::lists "lists";
Dir::Cache "cache";
Dir::Cache::archives "archives";
Dir::Log "log";
APT::Architecture "amd64";
APT::Architectures {{ "amd64"; }};
APT::Sandbox::User "{pwd.getpwuid(os.geteuid()).pw_name}";
Acquire::Languages "none";
Acquire::Retries "0";
Acquire::AllowInsecureRepositories "false";
APT::Get::AllowUnauthenticated "false";
''')
            env = {"PATH": os.environ["PATH"], "HOME": str(root), "LC_ALL": "C", "APT_CONFIG": str(config)}
            def client(binary, *arguments, success=True):
                result = subprocess.run([binary, *arguments], cwd=root / "downloads", env=env, capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
                return result.stdout
            # Act / Assert: APT itself validates signatures, indexes and downloaded package bytes.
            client("apt-get", "update", "--error-on=any")
            policy = client("apt-cache", "policy", "rquickshare")
            self.assertIn("Candidate: 0.11.5+native1", policy)
            client("apt-get", "download", "rquickshare=0.11.5+native1")
            downloads = list((root / "downloads").glob("*.deb"))
            self.assertEqual(len(downloads), 1)
            self.assertEqual(hashlib.sha256(downloads[0].read_bytes()).digest(), hashlib.sha256(self.package.read_bytes()).digest())
            shutil.copyfile(self.keys[1][0], keyring)
            client("apt-get", "update", "--error-on=any", success=False)
            self.assertEqual((root / "state/status").read_text(), "")


if __name__ == "__main__":
    unittest.main()
