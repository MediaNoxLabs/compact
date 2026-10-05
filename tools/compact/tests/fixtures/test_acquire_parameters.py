#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import hashlib
import io
import tempfile
import unittest
from pathlib import Path
from unittest import mock
import zipfile

import acquire_parameters as params


class ParameterAcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.bytes = b"public parameter test bytes"
        self.row = {"name": "test-params", "size": len(self.bytes),
                    "sha256": hashlib.sha256(self.bytes).hexdigest()}

    def test_explicit_download_then_offline_verification(self):
        opener = mock.Mock(return_value=io.BytesIO(self.bytes))
        path, origin = params.acquire_parameter(self.row, self.root, opener=opener)
        self.assertEqual(origin, "official-download")
        self.assertEqual(path.read_bytes(), self.bytes)
        self.assertEqual(opener.call_count, 1)
        opener = mock.Mock(side_effect=AssertionError("network forbidden"))
        _, origin = params.acquire_parameter(self.row, self.root, verify_only=True, opener=opener)
        self.assertEqual(origin, "verified-cache")
        opener.assert_not_called()

    def test_missing_offline_refuses_without_network(self):
        opener = mock.Mock(side_effect=AssertionError("network forbidden"))
        with self.assertRaisesRegex(ValueError, "missing public parameter"):
            params.acquire_parameter(self.row, self.root, verify_only=True, opener=opener)
        opener.assert_not_called()

    def test_wrong_size_or_hash_never_publishes(self):
        for data in [b"short", b"x" * len(self.bytes), self.bytes + b"extra"]:
            with self.subTest(data=data):
                with self.assertRaises(ValueError):
                    params.acquire_parameter(self.row, self.root, opener=lambda *a, **kw: io.BytesIO(data))
                self.assertEqual(list(self.root.iterdir()), [])

    def test_corrupt_existing_cache_and_symlink_refuse(self):
        path = self.root / self.row["name"]
        path.write_bytes(b"x" * len(self.bytes))
        with self.assertRaisesRegex(ValueError, "SHA256 mismatch"):
            params.acquire_parameter(self.row, self.root)
        path.unlink()
        seed = self.root / "seed"
        seed.write_bytes(self.bytes)
        path.symlink_to(seed)
        with self.assertRaisesRegex(ValueError, "regular fixture"):
            params.acquire_parameter(self.row, self.root)

    def test_explicit_seed_is_verified_and_never_changed(self):
        seed = self.root / "seed"
        seed.mkdir()
        source = seed / self.row["name"]
        source.write_bytes(self.bytes)
        path, origin = params.acquire_parameter(self.row, self.root / "cache", seed=seed)
        self.assertEqual(origin, "verified-seed")
        self.assertEqual(path.read_bytes(), source.read_bytes())
        self.assertEqual(source.read_bytes(), self.bytes)

    def test_archive_tool_must_embed_expected_parameter_digest(self):
        archive = self.root / "source.zip"
        with zipfile.ZipFile(archive, "w") as bundle:
            bundle.writestr("zkir", b"not the expected managed parameter table")
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        archive.rename(self.root / (digest + ".zip"))
        manifest = {"archives": [{"version": "0.24.0", "platform": "test",
                                  "sha256": digest, "size": (self.root / (digest + ".zip")).stat().st_size}]}
        with self.assertRaisesRegex(ValueError, "lacks expected parameter digest"):
            params.archive_tools(manifest, self.root, "test")


    def test_old_compiler_retains_archive_bundled_parameters(self):
        for wrapper, accepted in [(b'export ZKIR_PP="$thisdir"', True), (b'export ZKIR_PP=/other', False)]:
            with self.subTest(wrapper=wrapper):
                archive = self.root / "old.zip"
                with zipfile.ZipFile(archive, "w") as bundle:
                    bundle.writestr("zkir", b"historical-tool")
                    bundle.writestr("compactc", wrapper)
                    bundle.writestr("kzg", b"bundled-prover-parameters")
                    bundle.writestr("kzg.vp", b"bundled-verifier-parameters")
                digest = hashlib.sha256(archive.read_bytes()).hexdigest()
                size = archive.stat().st_size
                archive.rename(self.root / (digest + ".zip"))
                manifest = {"archives": [{"version": "0.22.0", "platform": "test", "sha256": digest, "size": size}]}
                if accepted:
                    [tool] = params.archive_tools(manifest, self.root, "test")
                    self.assertEqual(tool["parameter_source"], "archive-bundled-ZKIR_PP")
                    self.assertEqual(tool["bundled_parameters"][0]["sha256"], hashlib.sha256(b"bundled-prover-parameters").hexdigest())
                else:
                    with self.assertRaisesRegex(ValueError, "wrapper no longer selects"):
                        params.archive_tools(manifest, self.root, "test")


if __name__ == "__main__":
    unittest.main()
