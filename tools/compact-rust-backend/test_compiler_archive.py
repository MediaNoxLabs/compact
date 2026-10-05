# SPDX-License-Identifier: Apache-2.0
# Copyright (C) 2026 Midnight Foundation

from pathlib import Path
import os
import tempfile
import unittest
import zipfile

from build_compiler_archive import REQUIRED, build_archive


class CompilerArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.package = self.root / "package"
        self.output = self.root / "compiler.zip"
        for name in REQUIRED:
            source = self.package / (name if name.startswith("share/") else
                                     f"{'lib' if name.startswith('zkir') else 'bin'}/{name}")
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text(name)
            source.chmod(0o644 if name.startswith("share/") else 0o755)

    def test_nested_sources_modes_and_optional_notes_survive(self):
        nested = self.package / "share/compactc/runtime-rs/src/recording/kernel.rs"
        nested.parent.mkdir()
        nested.write_text("kernel")
        notes = self.root / "release notes.md"
        notes.write_text("notes")
        build_archive(self.package, self.output, notes)
        with zipfile.ZipFile(self.output) as archive:
            self.assertEqual(archive.read(nested.relative_to(self.package).as_posix()), b"kernel")
            self.assertEqual(archive.read("release notes.md"), b"notes")
            self.assertEqual(archive.getinfo("compactc").external_attr >> 16 & 0o777, 0o755)
            self.assertTrue(set(REQUIRED).issubset(archive.namelist()))

    def test_nix_epoch_timestamps_are_clamped_to_zip_epoch(self):
        for source in self.package.rglob("*"):
            if source.is_file():
                os.utime(source, (1, 1))
        build_archive(self.package, self.output)
        with zipfile.ZipFile(self.output) as archive:
            self.assertEqual(archive.getinfo("compactc").date_time[:3], (1980, 1, 1))

    def test_incomplete_package_keeps_existing_archive(self):
        self.output.write_bytes(b"existing")
        (self.package / "share/compactc/runtime-rs/Cargo.toml").unlink()
        with self.assertRaisesRegex(ValueError, "incomplete compiler package"):
            build_archive(self.package, self.output)
        self.assertEqual(self.output.read_bytes(), b"existing")

    def test_colliding_flattened_commands_are_rejected(self):
        (self.package / "lib/compactc").write_text("collision")
        with self.assertRaisesRegex(ValueError, "duplicate archive path"):
            build_archive(self.package, self.output)

    def test_notes_cannot_replace_a_compiler_command(self):
        notes = self.root / "compactc"
        notes.write_text("collision")
        with self.assertRaisesRegex(ValueError, "duplicate archive path"):
            build_archive(self.package, self.output, notes)


if __name__ == "__main__":
    unittest.main()
