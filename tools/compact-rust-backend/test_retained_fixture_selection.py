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
"""Portable discovery selection; strict historical authentication stays separate."""
import os
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import call, patch

import conformance_baseline_archive as archive
import test_guarded_point_zkir as graph_tests
import test_specification_conformance as conformance_tests


class RetainedFixtureSelectionTests(unittest.TestCase):
    def baseline_case(self):
        return conformance_tests.RepositoryBaselineTests(
            "test_checked_in_baseline_matches_current_sources_and_mappings")

    def graph_case(self):
        return graph_tests.GuardedPointGraphTests("test_unknown_opcode_fails_closed")

    def test_bundled_baseline_selection_restores_current_root(self):
        before = archive.conformance.ROOT
        case = self.baseline_case()
        with patch.dict(os.environ, {}, clear=True):
            self.assertEqual(archive.selected_test_root(), archive.BUNDLED_ROOT)
            case.setUp()
            try:
                self.assertEqual(archive.conformance.ROOT, archive.BUNDLED_ROOT.resolve())
                self.assertNotEqual(archive.conformance.ROOT, before)
            finally:
                self.assertTrue(case.doCleanups())
        self.assertEqual(archive.conformance.ROOT, before)

    def test_explicit_bad_archive_never_uses_the_bundled_root(self):
        before = archive.conformance.ROOT
        with tempfile.TemporaryDirectory() as temporary:
            missing = str(Path(temporary) / "missing")
            with patch.dict(os.environ, {archive.ENVIRONMENT: missing}):
                self.assertEqual(archive.selected_test_root(), missing)
                with self.assertRaises(FileNotFoundError):
                    self.baseline_case().setUp()
        self.assertEqual(archive.conformance.ROOT, before)

    def test_none_and_current_source_are_still_refused_by_the_authenticator(self):
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(ValueError, "explicit retained conformance inputs"):
                archive.authenticate_archive(None)
            with self.assertRaisesRegex(ValueError, "archive refused"):
                archive.authenticate_archive(archive.conformance.ROOT)

    def test_mutation_or_deletion_during_context_refuses_and_restores_root(self):
        before = archive.conformance.ROOT
        baseline = archive.conformance.load_json(
            Path(archive.conformance.__file__).with_suffix(".json").read_text())
        relative = next(iter(baseline["sources"]))
        for kind in ("changed", "deleted"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                selected = Path(temporary) / "archive"
                shutil.copytree(archive.BUNDLED_ROOT, selected)
                entered = False
                with self.assertRaises((ValueError, FileNotFoundError)):
                    with archive.repository_baseline_root(selected):
                        entered = True
                        target = selected / relative
                        if kind == "changed":
                            target.write_bytes(target.read_bytes() + b"\nchanged while selected")
                        else:
                            target.unlink()
                self.assertTrue(entered, "mutation control must reach an authenticated context")
                self.assertEqual(archive.conformance.ROOT, before)

    def test_changed_baseline_authority_cannot_authorize_bundled_data(self):
        authority = Path(archive.conformance.__file__).with_suffix(".json")
        original = Path.read_bytes

        def changed(path):
            return b"{}" if path == authority else original(path)

        with patch.object(Path, "read_bytes", changed):
            with self.assertRaisesRegex(ValueError, "baseline authority changed"):
                archive.authenticate_archive(archive.BUNDLED_ROOT)

    def test_graph_skip_is_runtime_only_and_late_cli_selection_is_observed(self):
        self.assertFalse(getattr(graph_tests.GuardedPointGraphTests, "__unittest_skip__", False))
        with patch.dict(os.environ, {}, clear=True), \
             patch.object(graph_tests, "ARTIFACTS", None):
            with self.assertRaisesRegex(unittest.SkipTest, "manual graph artifacts required"):
                self.graph_case().setUp()
            # CLI/sharder assigns ARTIFACTS after module/class creation. This is
            # selection plumbing only; the ten genuine graph laws run separately.
            selected = Path("late-explicit-selection")
            graph_tests.ARTIFACTS = selected
            case = self.graph_case()
            with patch.object(case, "load", side_effect=[{"before": True}, {"after": True}]) as load:
                case.setUp()
            self.assertIs(case.artifacts, selected)
            self.assertEqual(load.call_args_list, [call("baseline"), call("candidate")])
            self.assertTrue(case.doCleanups())

    def test_explicit_missing_manual_graph_is_an_error_not_a_skip(self):
        with tempfile.TemporaryDirectory() as temporary, \
             patch.dict(os.environ, {}, clear=True), \
             patch.object(graph_tests, "ARTIFACTS", Path(temporary) / "missing"):
            with self.assertRaises(FileNotFoundError):
                self.graph_case().setUp()

    def test_explicit_bad_retained_graph_root_is_an_error_not_a_skip(self):
        with tempfile.TemporaryDirectory() as temporary, \
             patch.dict(os.environ, {graph_tests.gate.RETAINED_GRAPH_ENVIRONMENT:
                                     str(Path(temporary) / "missing")}), \
             patch.object(graph_tests, "ARTIFACTS", None):
            with self.assertRaises(FileNotFoundError):
                self.graph_case().setUp()
