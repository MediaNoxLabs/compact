# This file is part of Compact.
# Copyright (C) 2026 contributors to Minokawa Compact
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# You may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Focused packaging controls; no npm, dependency downloads or package execution."""

import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from npm_scope_compat import (
    METADATA, ProfileMismatch, adapt, inventory, select_profile, write_archive,
)


class PackageScopeCompatibilityCases:
    """The same assertions run against each fixed package profile."""

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.source = self.root / "upstream"
        self.source.mkdir()
        self.profile = select_profile(self.PROFILE_NAME)
        self.package = {
            "name": self.profile.upstream_name, "version": self.profile.version, "type": "module",
            "files": ["runtime.js", "runtime.wasm", "runtime.d.ts", "snippets"],
            "exports": {"node": "./runtime.js"},
        }
        self.lock = {
            "name": self.profile.upstream_name, "version": self.profile.version, "lockfileVersion": 3,
            "requires": True,
            "packages": {"": {"name": self.profile.upstream_name, "version": self.profile.version}},
        }
        self.write_metadata()
        (self.source / "runtime.js").write_bytes(b"export const value = 1;\n")
        (self.source / "runtime.wasm").write_bytes(b"\x00asm\x01\x00\x00\x00")
        (self.source / "runtime.d.ts").write_bytes(b"export declare const value: number;\n")
        (self.source / "snippets").mkdir()
        (self.source / "snippets/helper.js").write_bytes(b"// retained helper\n")

    def adapt(self, source, output):
        return adapt(source, output, profile_name=self.PROFILE_NAME)

    def write_metadata(self):
        for name, value in (("package.json", self.package), ("package-lock.json", self.lock)):
            (self.source / name).write_text(json.dumps(value, indent=2) + "\n")

    def assert_refused(self):
        before = inventory(self.source)
        output = self.root / "refused-output"
        with self.assertRaises((ProfileMismatch, OSError)):
            self.adapt(self.source, output)
        self.assertFalse(output.exists())
        self.assertEqual(inventory(self.source), before)

    def test_exact_three_metadata_values_and_every_payload_byte(self):
        before = inventory(self.source)
        output = self.root / "adapted"
        receipt = self.adapt(self.source, output)
        actual = inventory(output / self.profile.lib_path)
        self.assertEqual(actual.keys(), before.keys())
        for name in before.keys() - METADATA:
            self.assertEqual(actual[name], before[name], name)
        expected_package, expected_lock = copy.deepcopy(self.package), copy.deepcopy(self.lock)
        expected_package["name"] = self.profile.compatibility_name
        expected_lock["name"] = self.profile.compatibility_name
        expected_lock["packages"][""]["name"] = self.profile.compatibility_name
        self.assertEqual(json.loads(actual["package.json"]), expected_package)
        self.assertEqual(json.loads(actual["package-lock.json"]), expected_lock)
        for name, count in (("package.json", 1), ("package-lock.json", 2)):
            self.assertEqual(before[name].count(self.profile.upstream_name.encode()), count)
            self.assertEqual(actual[name], before[name].replace(self.profile.upstream_name.encode(), self.profile.compatibility_name.encode()))
        self.assertEqual(inventory(self.source), before)
        self.assertEqual(len(receipt["metadata_fields_changed"]), 3)
        self.assertEqual(receipt["upstream_files_sha256"].keys(), before.keys())
        self.assertNotIn("nix-support/npm-scope-compatibility.json", actual)

    def test_tarball_is_identical_across_outputs_and_source_mtime_changes(self):
        first, second = self.root / "first", self.root / "second"
        self.adapt(self.source, first)
        for path in self.source.rglob("*"):
            os.utime(path, (123456789, 123456789))
        self.adapt(self.source, second)
        self.assertEqual((first / self.profile.tar_path).read_bytes(), (second / self.profile.tar_path).read_bytes())

    def test_unlisted_payload_is_retained_and_authenticated(self):
        (self.source / "NOTICE").write_bytes(b"upstream copyright\n")
        output = self.root / "extra"
        receipt = self.adapt(self.source, output)
        self.assertEqual((output / self.profile.lib_path / "NOTICE").read_bytes(), b"upstream copyright\n")
        self.assertEqual(receipt["upstream_files_sha256"]["NOTICE"],
                         receipt["compatibility_files_sha256"]["NOTICE"])

    def test_wrong_name_or_version_is_refused(self):
        other_version = "2.1.1" if self.PROFILE_NAME == "onchain-runtime-v3" else "3.1.2"
        for field, value in (("name", self.profile.compatibility_name), ("name", "unrelated"),
                             ("version", "3.1.3"), ("version", other_version)):
            with self.subTest(field=field, value=value):
                old = self.package[field]
                self.package[field] = value
                self.write_metadata()
                self.assert_refused()
                self.package[field] = old

    def test_missing_or_malformed_lock_is_refused_without_creating_one(self):
        path = self.source / "package-lock.json"
        path.unlink()
        self.assert_refused()
        self.assertFalse(path.exists())
        path.write_bytes(b"{invalid")
        self.assert_refused()

    def test_duplicate_metadata_keys_are_refused(self):
        path = self.source / "package.json"
        path.write_text(path.read_text().replace('"version":', '"name": "duplicate", "version":'))
        self.assert_refused()

    def test_changed_lock_identity_or_dependencies_are_refused(self):
        original = copy.deepcopy(self.lock)
        for apply in (
            lambda: self.lock.update(name="other"),
            lambda: self.lock.update(version="3.1.3"),
            lambda: self.lock.update(lockfileVersion=2),
            lambda: self.lock["packages"][""].update(name="other"),
            lambda: self.lock["packages"].update({"node_modules/other": {"version": "1"}}),
            lambda: self.lock.update(requires=False),
            lambda: self.lock.update(dependencies={"other": "1"}),
        ):
            self.lock = copy.deepcopy(original)
            apply()
            self.write_metadata()
            self.assert_refused()

    def test_package_dependency_is_refused(self):
        self.package["dependencies"] = {"other": "1"}
        self.write_metadata()
        self.assert_refused()

    def test_symlink_payload_and_source_are_refused(self):
        alias = self.source / "alias.js"
        alias.symlink_to("runtime.js")
        with self.assertRaises(ProfileMismatch):
            self.adapt(self.source, self.root / "linked-payload")
        alias.unlink()
        linked_source = self.root / "source-link"
        linked_source.symlink_to(self.source, target_is_directory=True)
        with self.assertRaises(ProfileMismatch):
            self.adapt(linked_source, self.root / "linked-source")

    def test_existing_output_and_source_overlap_are_refused(self):
        existing = self.root / "existing"
        existing.mkdir()
        (existing / "keep").write_bytes(b"keep")
        for output in (existing, self.source / "nested"):
            with self.assertRaises(ProfileMismatch):
                self.adapt(self.source, output)
        self.assertEqual((existing / "keep").read_bytes(), b"keep")
        self.assertFalse((self.source / "nested").exists())

    def test_broken_output_symlink_is_refused_without_removing_it(self):
        output = self.root / "broken-output"
        output.symlink_to(self.root / "missing")
        with self.assertRaises(ProfileMismatch):
            self.adapt(self.source, output)
        self.assertTrue(output.is_symlink())

    def test_special_payload_file_is_refused(self):
        os.mkfifo(self.source / "pipe")
        with self.assertRaisesRegex(ProfileMismatch, "special file"):
            self.adapt(self.source, self.root / "special-output")
        self.assertFalse((self.root / "special-output").exists())

    def test_live_source_change_during_archive_is_refused(self):
        def change_then_archive(path, files, source):
            (source / "runtime.js").write_bytes(b"// changed during adaptation\n")
            write_archive(path, files, source)

        output = self.root / "changed-source"
        with patch("npm_scope_compat.write_archive", side_effect=change_then_archive):
            with self.assertRaisesRegex(ProfileMismatch, "source changed"):
                self.adapt(self.source, output)
        self.assertFalse(output.exists())

    def test_archive_failure_cleans_only_its_new_output(self):
        before = inventory(self.source)
        output = self.root / "failed"
        with patch("npm_scope_compat.write_archive", side_effect=OSError("injected failure")):
            with self.assertRaisesRegex(OSError, "injected failure"):
                self.adapt(self.source, output)
        self.assertFalse(output.exists())
        self.assertEqual(inventory(self.source), before)

    def test_selecting_the_other_fixed_profile_is_refused(self):
        other = next(name for name in ("onchain-runtime-v3", "zkir-v2") if name != self.PROFILE_NAME)
        output = self.root / "wrong-profile"
        with self.assertRaisesRegex(ProfileMismatch, "unexpected upstream package name"):
            adapt(self.source, output, profile_name=other)
        self.assertFalse(output.exists())


class OnchainRuntimeNpmScopeCompatibilityTests(PackageScopeCompatibilityCases, unittest.TestCase):
    PROFILE_NAME = "onchain-runtime-v3"


class ZkirV2NpmScopeCompatibilityTests(PackageScopeCompatibilityCases, unittest.TestCase):
    PROFILE_NAME = "zkir-v2"


class ProfileSelectionTests(unittest.TestCase):
    def test_unknown_profile_refuses_before_source_or_output_access(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            source, output = root / "absent-source", root / "absent-output"
            for profile_name in ("arbitrary-name-and-version", "zkir-v3"):
                with self.subTest(profile_name=profile_name):
                    with self.assertRaisesRegex(ProfileMismatch, "unsupported package profile"):
                        adapt(source, output, profile_name=profile_name)
            self.assertFalse(source.exists())
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
