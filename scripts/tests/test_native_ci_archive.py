import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
import native_apt as apt
import native_r2 as r2
import native_ci_archive as archive
import test_native_apt as fixtures


class StreamingBody:
    def __init__(self, data):
        self.data = data
        self.closed = False

    def iter_chunks(self, chunk_size=65536):
        for i in range(0, len(self.data), chunk_size):
            yield self.data[i:i + chunk_size]

    def close(self):
        self.closed = True


class FakeS3:
    """Minimal external S3 boundary; no APT domain logic is faked."""
    def __init__(self, objects):
        self.objects = dict(objects)
        self.fail_reads = False
        self.length_override = {}

    def get_object(self, Bucket, Key):
        assert Bucket == r2.BUCKET
        if self.fail_reads:
            raise OSError("Private bucket unavailable")
        if Key not in self.objects:
            error = OSError("Missing key")
            error.response = {"Error": {"Code": "NoSuchKey"}}
            raise error
        data = self.objects[Key]
        return {"ContentLength": self.length_override.get(Key, len(data)),
                "ETag": '"fixture-etag"',
                "Body": StreamingBody(data)}

    def list_objects_v2(self, Bucket, Prefix, MaxKeys):
        assert Bucket == r2.BUCKET
        entries = [{"Key": key} for key in sorted(self.objects) if key.startswith(Prefix)]
        return {"IsTruncated": len(entries) > MaxKeys,
                "Contents": entries[:MaxKeys]}

    def close(self):
        pass


class NativeCIArchiveTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        fixtures.NativeAptTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        fixtures.NativeAptTests.tearDownClass.__func__(cls)

    def channel(self):
        objects, active = r2.publication(self.signed, self.keyring, self.fingerprint, now=self.now)
        return FakeS3({**{item.key: item.source.read_bytes() for item in objects},
                       active.key: active.source.read_bytes()}), active

    def test_given_signed_private_snapshot_when_recovered_then_exact_current_signature_and_package_bytes_are_verified(self):
        client, active = self.channel()
        with tempfile.TemporaryDirectory() as temporary:
            recovered = Path(temporary) / "previous"
            self.assertTrue(archive.restore(r2.S3Store(client), recovered, self.keyring, self.fingerprint))
            self.assertEqual(apt.verify(recovered, self.keyring, self.fingerprint, self.now)["version"], "1.0.0-1")
            self.assertEqual((recovered / r2.ACTIVE).read_bytes(), active.source.read_bytes())
            package = next(recovered.glob("pool/**/*.deb"))
            self.assertEqual(hashlib.sha256(package.read_bytes()).hexdigest(),
                             hashlib.sha256(self.package.read_bytes()).hexdigest())

    def test_given_missing_active_authority_when_checked_then_only_confirmed_absence_means_first_release(self):
        client, active = self.channel()
        del client.objects[active.key]
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "previous"
            self.assertFalse(archive.restore(r2.S3Store(client), output, self.keyring, self.fingerprint))
            self.assertFalse(output.exists())

    def test_given_missing_corrupt_extra_or_oversized_objects_when_recovered_then_no_snapshot_is_committed(self):
        for kind in ("missing", "corrupt", "traversal", "oversize", "remote-outage"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                client, active = self.channel()
                prefix = f"snapshots/{active.sha256}/"
                if kind == "missing":
                    del client.objects[prefix + r2.ACTIVE]
                elif kind == "corrupt":
                    client.objects[prefix + r2.ACTIVE] = b"tampered signed metadata"
                elif kind == "traversal":
                    client.objects[prefix + "../escape"] = b"untrusted"
                elif kind == "oversize":
                    key = next(key for key in client.objects if key.startswith(prefix) and key.endswith(".deb"))
                    client.length_override[key] = apt.MAX_PACKAGE + 1
                else:
                    client.fail_reads = True
                output = Path(temporary) / "previous"
                with self.assertRaises((ValueError, OSError, subprocess.CalledProcessError, r2.Unavailable)):
                    archive.restore(r2.S3Store(client), output, self.keyring, self.fingerprint)
                self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
