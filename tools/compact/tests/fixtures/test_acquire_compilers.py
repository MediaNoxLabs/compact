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

import copy
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest

import acquire_compilers as helper


class AcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.manifest = Path(__file__).with_name("compiler-archives.json")
        self.data = json.loads(self.manifest.read_text())
        self.bytes = b"genuine bytes stand-in for acquisition unit test only"
        self.row = dict(self.data["archives"][0], size=len(self.bytes), sha256=hashlib.sha256(self.bytes).hexdigest())

    def load(self, data):
        path = self.root / "manifest.json"
        path.write_text(json.dumps(data))
        return helper.load_manifest(path)

    def test_actual_runner_platform_mapping(self):
        for system, machine, expected in [("Darwin", "arm64", "aarch64-darwin"), ("Darwin", "x86_64", "x86_64-apple-darwin"), ("Linux", "x86_64", "x86_64-unknown-linux-musl"), ("Linux", "aarch64", "aarch64-unknown-linux-musl")]:
            self.assertEqual(helper.host_platform(system, machine), expected)
        with self.assertRaises(ValueError): helper.host_platform("Windows", "AMD64")

    def test_manifest_has_all_current_platforms_and_faithful_absences(self):
        data = helper.load_manifest(self.manifest)
        self.assertEqual(len(data["catalogue"]), 9)
        self.assertEqual(len(data["archives"]), 15)
        catalogue = {r["version"]: r["platforms"] for r in data["catalogue"]}
        self.assertNotIn("aarch64-darwin", catalogue["0.22.0"])
        self.assertNotIn("x86_64-apple-darwin", catalogue["0.23.0"])
        self.assertEqual(set(catalogue["0.31.0"]), helper.PLATFORMS)

    def test_duplicate_and_missing_archive_pins_refused(self):
        for operation in [lambda d: d["archives"].append(d["archives"][0]), lambda d: d["archives"].pop()]:
            data = copy.deepcopy(self.data); operation(data)
            with self.assertRaises(ValueError): self.load(data)

    def test_manifest_identity_url_and_digest_refused(self):
        for operation in [lambda d: d.update(repository="other/repo"), lambda d: d["archives"][0].update(asset_id=999), lambda d: d["catalogue"][0]["platforms"]["x86_64-apple-darwin"].update(browser_download_url="https://example.invalid/archive.zip"), lambda d: d["archives"][0].update(reported_digest="sha256:" + "0" * 64)]:
            data = copy.deepcopy(self.data); operation(data)
            with self.assertRaises(ValueError): self.load(data)

    def test_missing_verify_only_never_acquires(self):
        with self.assertRaisesRegex(ValueError, "not acquired"):
            helper.acquire(self.row, self.root, verify_only=True, opener=lambda *a, **k: self.fail("network"))
        self.assertEqual(list(self.root.iterdir()), [])

    def test_download_then_cache_reuse(self):
        path, origin = helper.acquire(self.row, self.root, opener=lambda *a, **k: io.BytesIO(self.bytes))
        self.assertEqual(origin, "official-download")
        self.assertEqual(path.read_bytes(), self.bytes)
        self.assertEqual(helper.acquire(self.row, self.root, opener=lambda *a, **k: self.fail("network"))[1], "verified-cache")
        self.assertEqual(list(self.root.glob(".partial-*")), [])

    def test_bad_download_leaves_no_published_or_partial_cache(self):
        for content in [self.bytes[:-1], self.bytes+b"x", b"x"*len(self.bytes)]:
            with self.assertRaises(ValueError):
                helper.acquire(self.row, self.root, opener=lambda *a, **k: io.BytesIO(content))
            self.assertEqual(list(self.root.iterdir()), [])

    def test_corrupt_existing_cache_refused_without_network(self):
        path = self.root / (self.row["sha256"] + ".zip")
        path.write_bytes(b"x"*len(self.bytes))
        with self.assertRaisesRegex(ValueError, "SHA256"):
            helper.acquire(self.row, self.root, opener=lambda *a, **k: self.fail("network"))

    def test_seed_is_verified_before_atomic_publication(self):
        seed = self.root / "seed"; cache = self.root / "cache"
        source = seed / ("compactc-v"+self.row["version"]) / self.row["name"]
        source.parent.mkdir(parents=True); source.write_bytes(self.bytes)
        path, mode = helper.acquire(self.row, cache, seed, opener=lambda *a, **k: self.fail("network"))
        self.assertEqual(mode, "verified-seed"); self.assertEqual(path.read_bytes(), self.bytes)

    def test_symlink_cache_refused(self):
        source = self.root / "source"; source.write_bytes(self.bytes)
        (self.root / (self.row["sha256"] + ".zip")).symlink_to(source)
        with self.assertRaisesRegex(ValueError, "regular"):
            helper.acquire(self.row, self.root, verify_only=True)


if __name__ == "__main__":
    unittest.main()
