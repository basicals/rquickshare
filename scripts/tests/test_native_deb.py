import contextlib
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("native_deb", Path(__file__).parents[1] / "native_deb.py")
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)
DEPENDENCIES = "systemd, libpam-systemd, dbus-user-session, bluez, xdg-desktop-portal, libgtk-4-1 (>= 4.12), libgtk4-layer-shell0 (>= 1.3), bubblewrap, glycin-loaders (>= 2.1)"


def fields():
    return {"Package": "rquickshare-native", "Version": "1.0.0-1", "Architecture": "amd64", "Depends": DEPENDENCIES, "Conflicts": "rquickshare", "Replaces": "rquickshare"}


class NativeDebTests(unittest.TestCase):
    def test_given_metadata_when_identity_version_arch_or_dependencies_are_invalid_then_reject(self):
        # Arrange / Act / Assert
        package.validate_metadata(fields())
        for field, variants in {
            "Package": ["", "other", "rquickshare"],
            "Conflicts": ["", "rquickshare-native", "other"],
            "Replaces": ["", "rquickshare-native", "other"],
            "Version": ["", "1.0.0", "1.0.0+native1", "1.0.0-1.fake", "1.0.0\nInjected: field", "1.0.0-0", "1.0.0-01", "1.0.0-2147483648", "01.0.0-1"],
            "Architecture": ["", "i386", "amd64\n"],
            "Depends": ["", DEPENDENCIES.replace("systemd, ", "", 1), DEPENDENCIES.replace("libgtk4-layer-shell0", "wrong"), DEPENDENCIES.replace(" (>= 4.12)", ""), DEPENDENCIES.replace("bubblewrap", "wrong"), DEPENDENCIES.replace("glycin-loaders", "wrong"), DEPENDENCIES.replace("glycin-loaders (>= 2.1)", "glycin-loaders")],
        }.items():
            for value in variants:
                with self.subTest(field=field, value=value):
                    invalid = fields() | {field: value}
                    with self.assertRaises(ValueError):
                        package.validate_metadata(invalid)
        package.validate_metadata(fields() | {"Version": "1.0.0-1.debug", "Architecture": "arm64"})
        for revision in [1, 2, 10, 2147483647]:
            package.validate_metadata(fields() | {"Version": package.native_version("1.0.0", revision)})
        for revision in [None, True, "2", 0, -1, 2147483648]:
            with self.subTest(revision=revision), self.assertRaises(ValueError):
                package.native_version("1.0.0", revision)
        for left, right in [(1, 2), (2, 10), (10, 2147483647)]:
            subprocess.run(["dpkg", "--compare-versions", package.native_version("1.0.0", left), "lt", package.native_version("1.0.0", right)], check=True)

    def test_given_selected_assets_when_validated_then_process_ownership_supports_only_opt_in_login_without_direct_fallback(self):
        # Arrange / Act / Assert: actual shipped service/activation assets.
        package.validate_assets()
        self.assertEqual(len(package.FILES), 14)
        icons = {name for name in package.FILES if name.startswith("usr/share/icons/")}
        self.assertEqual(icons, {
            f"usr/share/icons/hicolor/{size}x{size}/apps/rquickshare.png"
            for size in [32, 128, 256]
        })
        self.assertFalse(any("autostart" in name for name in package.FILES))

    def test_given_elf_header_when_truncated_wrong_type_endian_or_machine_then_reject_relabelled_binary(self):
        # Arrange / Act / Assert: actual ELF machine identity, independent of file names.
        for architecture, machine in [("amd64", 62), ("arm64", 183)]:
            header = bytearray(64)
            header[:6] = b"\x7fELF\x02\x01"
            header[16:18] = (3).to_bytes(2, "little")
            header[18:20] = machine.to_bytes(2, "little")
            package.validate_elf(header, architecture)
            for offset, replacement in [(0, 0), (4, 1), (5, 2), (16, 1), (18, 0)]:
                invalid = header.copy()
                invalid[offset] = replacement
                with self.subTest(architecture=architecture, offset=offset), self.assertRaises(ValueError):
                    package.validate_elf(invalid, architecture)
            for size in [0, 5, 19]:
                with self.subTest(architecture=architecture, size=size), self.assertRaises(ValueError):
                    package.validate_elf(header[:size], architecture)

    def create(self, directory, mutation=None):
        root = directory / "root"
        root.mkdir()
        for destination, (source, mode) in package.FILES.items():
            target = root / destination
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(source.read_bytes() if source else Path("/bin/true").read_bytes())
            target.chmod(mode)
        control = root / "DEBIAN"
        control.mkdir()
        (control / "control").write_text("\n".join(f"{key}: {value}" for key, value in (fields() | {"Architecture": package.run("dpkg", "--print-architecture")}).items()) + "\nMaintainer: Test <noreply@github.com>\nDescription: Test-only offline fixture\n")
        (control / "control").chmod(0o644)
        for staged_directory in [root, *[path for path in root.rglob("*") if path.is_dir()]]:
            staged_directory.chmod(0o755)
        if mutation:
            mutation(root)
        artifact = directory / "fixture.deb"
        subprocess.run(["dpkg-deb", "--root-owner-group", "--build", str(root), str(artifact)], check=True, stdout=subprocess.DEVNULL)
        artifact.with_suffix(".deb.sha256").write_text(package.checksum(artifact))
        return artifact

    def test_given_actual_debian_archive_when_layout_mode_or_control_is_tampered_then_reject_even_with_valid_checksum(self):
        # Arrange / Act / Assert: real Debian tooling, no mocked package verifier.
        mutations = {
            "extra-file": lambda root: (root / "usr/private").write_text("unexpected"),
            "missing": lambda root: (root / "usr/bin/rquickshare").unlink(),
            "mode": lambda root: (root / "usr/bin/rquickshare").chmod(0o777),
            "link": lambda root: ((root / "usr/bin/rquickshare").unlink(), (root / "usr/bin/rquickshare").symlink_to("/bin/true")),
            "maintainer-script": lambda root: ((root / "DEBIAN/postinst").write_text("#!/bin/sh\nexit 0\n"), (root / "DEBIAN/postinst").chmod(0o755)),
            "asset": lambda root: (root / "usr/lib/systemd/user/rquickshare-daemon.service").write_text("[Service]\nType=dbus\n"),
            "desktop-unit": lambda root: (root / "usr/lib/systemd/user/rquickshare-desktop.service").write_text("[Service]\nExecStart=/usr/bin/rquickshare\n"),
            "desktop-activation": lambda root: (root / "usr/share/dbus-1/services/io.github.morozhenko_apps.RQuickShare.Desktop.service").write_text("[D-BUS Service]\nExec=/usr/bin/rquickshare\n"),
            "extra-directory": lambda root: (root / "etc").mkdir(),
        }
        for name, mutate in [("valid", None), *mutations.items()]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                artifact = self.create(Path(temporary), mutate)
                with contextlib.redirect_stdout(io.StringIO()):
                    if mutate:
                        with self.assertRaises(ValueError):
                            package.verify(artifact, smoke=False)
                    else:
                        package.verify(artifact, smoke=False)

    def test_given_checksum_when_corrupt_multiple_or_wrong_package_then_reject_before_extraction(self):
        # Arrange / Act / Assert
        with tempfile.TemporaryDirectory() as temporary:
            artifact = self.create(Path(temporary))
            valid = package.checksum(artifact)
            for invalid in ["", valid + valid, "0" * 64 + "  fixture.deb\n", valid.replace("fixture.deb", "other.deb")]:
                with self.subTest(checksum=invalid):
                    artifact.with_suffix(".deb.sha256").write_text(invalid)
                    with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
                        package.verify(artifact, smoke=False)


if __name__ == "__main__":
    unittest.main()
