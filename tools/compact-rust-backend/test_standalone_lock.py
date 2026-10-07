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

"""Bounded lock-closure controls; no downloads or Cargo builds."""

import copy
import json
from pathlib import Path
import tempfile
import unittest

from check_standalone_lock import compare_locks

SOURCE = "registry+https://github.com/rust-lang/crates.io-index"


def package(name, dependencies=(), version="1.0.0"):
    result = {"name": name, "version": version, "dependencies": list(dependencies)}
    if name != "compact-rust-backend":
        result.update(source=SOURCE, checksum="ab" * 32)
    return result


def write_lock(path, packages):
    parts = ["version = 4\n"]
    for item in packages:
        parts.append("[[package]]\n")
        for key, value in item.items():
            parts.append(key + " = " + json.dumps(value) + "\n")
    path.write_text("".join(parts))


class StandaloneLockTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.approved = [package("compact-rust-backend", ["a"], "0.2.0"),
                         package("a", ["b 1.0.0", "c"]), package("b"), package("c")]
        self.selected = copy.deepcopy(self.approved[:3])
        self.selected[1]["dependencies"] = ["b"]

    def run_check(self):
        write_lock(self.root / "workspace", self.approved)
        write_lock(self.root / "standalone", self.selected)
        return compare_locks(self.root / "workspace", self.root / "standalone")

    def test_exact_retained_edges_allow_unambiguous_shorthand_and_report_pruning(self):
        result = self.run_check()
        self.assertEqual(result["standalone_packages"], 3)
        self.assertEqual(result["retained_edges"], 2)
        self.assertEqual(result["removed_feature_edges"][0]["removed_edges"][0][0], "c")

    def test_git_source_is_not_reported_as_registry(self):
        for graph in [self.approved, self.selected]:
            graph[2]["source"] = "git+https://example.invalid/repo?rev=abc#abc"
            del graph[2]["checksum"]
        result = self.run_check()
        self.assertEqual(result["registry_packages"], 1)
        self.assertEqual(result["git_packages"], 1)
        self.assertEqual(result["local_packages"], 1)

    def test_changed_checksum_refused(self):
        self.selected[1]["checksum"] = "cd" * 32
        with self.assertRaisesRegex(RuntimeError, "checksum differs"):
            self.run_check()

    def test_version_and_source_drift_refused(self):
        for key, value in [("version", "1.1.0"), ("source", "registry+https://example.invalid/index")]:
            with self.subTest(key=key):
                saved = self.selected[1][key]
                self.selected[1][key] = value
                with self.assertRaisesRegex(RuntimeError, "unapproved locked package identity"):
                    self.run_check()
                self.selected[1][key] = saved

    def test_added_edge_refused_even_when_both_packages_approved(self):
        self.selected[2]["dependencies"] = ["a"]
        with self.assertRaisesRegex(RuntimeError, "unapproved dependency edge"):
            self.run_check()

    def test_removed_backend_direct_edge_refused(self):
        self.selected[0]["dependencies"] = []
        with self.assertRaisesRegex(RuntimeError, "direct dependency edges differ"):
            self.run_check()

    def test_ambiguous_dependency_refused(self):
        extra = package("b", version="2.0.0")
        self.approved.append(extra)
        self.selected.append(extra)
        with self.assertRaisesRegex(RuntimeError, "ambiguous locked edge"):
            self.run_check()

    def test_missing_dependency_refused(self):
        self.selected[1]["dependencies"] = ["absent"]
        with self.assertRaisesRegex(RuntimeError, "missing or ambiguous locked edge"):
            self.run_check()

    def test_unreachable_package_refused(self):
        self.selected.append(copy.deepcopy(self.approved[3]))
        with self.assertRaisesRegex(RuntimeError, "unreachable"):
            self.run_check()

    def test_duplicate_identity_refused(self):
        self.selected.append(copy.deepcopy(self.selected[1]))
        with self.assertRaisesRegex(RuntimeError, "duplicate locked package"):
            self.run_check()


if __name__ == "__main__":
    unittest.main()
