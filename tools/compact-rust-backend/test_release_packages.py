#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Focused release inspection tests; no publishing or Cargo execution."""

import importlib.util
import io
import json
from pathlib import Path
import shutil
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("release", Path(__file__).with_name("check_release_packages.py"))
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)
SOURCE_ROOT = release.ROOT


class ReleaseInspectionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "source"
        for name in ("runtime-rs/Cargo.toml", "runtime-rs/compatibility.json",
                     "runtime-rs-macros/Cargo.toml", "tools/compact-rust-backend/src/compatibility.json"):
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(SOURCE_ROOT / name, target)
        self.packages = Path(self.temp.name) / "external-target/package"
        self.packages.mkdir(parents=True)
        for name, value in (("ROOT", self.root), ("MACROS", self.root / "runtime-rs-macros")):
            context = patch.object(release, name, value)
            context.start()
            self.addCleanup(context.stop)
        context = patch.object(release, "package_directory", return_value=self.packages)
        context.start()
        self.addCleanup(context.stop)

    def archive(self, dependency="=0.1.0", record=True):
        name = "midnight-compact-runtime"
        path = self.packages / f"{name}-0.1.0.crate"
        files = {
            "Cargo.toml": f'[dependencies.midnight-compact-runtime-macros]\nversion = "{dependency}"\n',
            "LICENSE": "license", "README.md": "readme", "src/lib.rs": "// source",
        }
        if record:
            files["compatibility.json"] = (self.root / "runtime-rs/compatibility.json").read_text()
        with tarfile.open(path, "w:gz") as archive:
            for relative, text in files.items():
                data = text.encode()
                member = tarfile.TarInfo(f"{name}-0.1.0/{relative}")
                member.size = len(data)
                archive.addfile(member, io.BytesIO(data))
        return path

    def test_current_pair_and_external_target_archive(self):
        self.assertEqual(release.compatibility_record()["runtime_abi"], 50)
        path = self.archive()
        receipt = release.inspect_package("midnight-compact-runtime")
        self.assertEqual(receipt["archive"], str(path))
        self.assertEqual(receipt["sha256"], release.digest(path))

    def test_macro_version_comes_from_source_not_hardcoded_expectation(self):
        for relative in ("runtime-rs-macros/Cargo.toml", "runtime-rs/Cargo.toml"):
            path = self.root / relative
            text = path.read_text()
            if relative.startswith("runtime-rs-macros"):
                text = text.replace('version = "0.1.0"', 'version = "0.2.7"')
            else:
                text = text.replace('version = "=0.1.0"', 'version = "=0.2.7"')
            path.write_text(text)
        for relative in ("runtime-rs/compatibility.json", "tools/compact-rust-backend/src/compatibility.json"):
            path = self.root / relative
            record = json.loads(path.read_text())
            record["macros_version"] = "0.2.7"
            path.write_text(json.dumps(record))
        self.archive(dependency="=0.2.7")
        release.inspect_package("midnight-compact-runtime")
        self.archive(dependency="=0.1.0")
        with self.assertRaisesRegex(RuntimeError, "macro dependency is not publishable"):
            release.inspect_package("midnight-compact-runtime")

    def test_stale_source_record_refused(self):
        path = self.root / "runtime-rs/compatibility.json"
        path.write_text(path.read_text().replace('"runtime_abi": 50', '"runtime_abi": 49'))
        with self.assertRaisesRegex(RuntimeError, "differs from compiler"):
            release.compatibility_record()

    def test_mixed_manifest_graph_refused(self):
        path = self.root / "runtime-rs/Cargo.toml"
        path.write_text(path.read_text().replace('midnight-onchain-vm = "=3.0.0"', 'midnight-onchain-vm = "=4.0.0"'))
        with self.assertRaisesRegex(RuntimeError, "midnight-onchain-vm"):
            release.compatibility_record()

    def test_missing_packaged_record_refused(self):
        self.archive(record=False)
        with self.assertRaises((KeyError, RuntimeError)):
            release.inspect_package("midnight-compact-runtime")


if __name__ == "__main__":
    unittest.main()
