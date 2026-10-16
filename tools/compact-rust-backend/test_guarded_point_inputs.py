# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Input authentication and fixture lifetime for retained nested graph controls."""
import json
import os
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import guarded_point_zkir as gate
import test_guarded_point_zkir as graph_tests


class RetainedGraphInputTests(unittest.TestCase):
    def copy_inputs(self, directory):
        source = os.environ.get(gate.RETAINED_GRAPH_ENVIRONMENT)
        if source is None:
            self.skipTest("manual graph artifacts required: supply "
                          + gate.RETAINED_GRAPH_ENVIRONMENT)
        directory = Path(directory)
        artifacts = directory / "private graph"
        shutil.copytree(source, artifacts)
        source_root = directory / "private source"
        target = source_root / gate.RETAINED_GRAPH_AUTHORITY["source"]["path"]
        target.parent.mkdir(parents=True)
        shutil.copyfile(gate.FIXTURES / "nested.compact", target)
        return artifacts.resolve(), source_root.resolve()

    def first_graph(self, artifacts):
        return artifacts / next(iter(gate.RETAINED_GRAPH_AUTHORITY["artifact_fingerprints"]))

    def test_positive_receipt_has_original_identities_and_retained_scope(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts, source = self.copy_inputs(directory)
            receipt = gate.authenticate_retained_artifacts(artifacts, source_root=source)
            self.assertEqual(receipt["root"], str(artifacts))
            self.assertEqual(receipt["producer_identities"], gate.RETAINED_GRAPH_AUTHORITY["producer_identities"])
            self.assertEqual(len(receipt["artifact_fingerprints"]), 2)
            self.assertEqual(len(receipt["authority_fingerprints"]), 4)
            self.assertIn("not fresh current emission", receipt["qualification"])

    def test_missing_relative_and_wrong_inputs_have_no_fallback(self):
        with self.assertRaisesRegex(ValueError, "explicit retained guarded-point inputs"):
            gate.authenticate_retained_artifacts(None)
        with self.assertRaisesRegex(ValueError, "must be absolute"):
            gate.authenticate_retained_artifacts("relative")
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(FileNotFoundError):
                gate.authenticate_retained_artifacts(Path(directory) / "missing")
            with self.assertRaises(FileNotFoundError):
                gate.authenticate_retained_artifacts(directory)

    def test_graph_content_and_deletion_are_live(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts, source = self.copy_inputs(directory)
            gate.authenticate_retained_artifacts(artifacts, source_root=source)
            graph = self.first_graph(artifacts)
            graph.write_bytes(graph.read_bytes() + b"\nchanged")
            with self.assertRaisesRegex(ValueError, "retained input drift"):
                gate.authenticate_retained_artifacts(artifacts, source_root=source)
            graph.unlink()
            with self.assertRaises(FileNotFoundError):
                gate.authenticate_retained_artifacts(artifacts, source_root=source)

    def test_original_authority_files_cannot_be_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts, source = self.copy_inputs(directory)
            authority = artifacts / next(iter(gate.RETAINED_GRAPH_AUTHORITY["authority_fingerprints"]))
            authority.write_bytes(authority.read_bytes() + b"\nchanged")
            with self.assertRaisesRegex(ValueError, "retained input drift"):
                gate.authenticate_retained_artifacts(artifacts, source_root=source)

    def test_current_nested_source_is_authenticated(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts, source = self.copy_inputs(directory)
            target = source / gate.RETAINED_GRAPH_AUTHORITY["source"]["path"]
            target.write_bytes(target.read_bytes() + b"\nchanged")
            with self.assertRaisesRegex(ValueError, "current fixture drift"):
                gate.authenticate_retained_artifacts(artifacts, source_root=source)

    def test_matching_bytes_outside_graph_root_do_not_authorize_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts, source = self.copy_inputs(directory)
            graph = self.first_graph(artifacts)
            outside = Path(directory) / "outside"
            outside.write_bytes(graph.read_bytes())
            graph.unlink()
            graph.symlink_to(outside)
            with self.assertRaisesRegex(ValueError, "escapes root"):
                gate.authenticate_retained_artifacts(artifacts, source_root=source)

    def test_context_cleanup_preserves_original_failure_and_global_selection(self):
        before = graph_tests.ARTIFACTS
        with tempfile.TemporaryDirectory() as directory:
            artifacts, source = self.copy_inputs(directory)
            with self.assertRaisesRegex(RuntimeError, "original case failed"):
                with gate.retained_artifact_root(artifacts, source_root=source):
                    raise RuntimeError("original case failed")
        self.assertIs(graph_tests.ARTIFACTS, before)

    def test_context_cleanup_rejects_graph_or_source_mutation(self):
        for kind in ("graph", "source"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as directory:
                artifacts, source = self.copy_inputs(directory)
                target = (self.first_graph(artifacts) if kind == "graph" else
                          source / gate.RETAINED_GRAPH_AUTHORITY["source"]["path"])
                with self.assertRaisesRegex(ValueError, "drift"):
                    with gate.retained_artifact_root(artifacts, source_root=source):
                        target.write_bytes(target.read_bytes() + b"\nchanged during case")

    def test_alias_retarget_cannot_hide_selected_graph_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            first = Path(directory) / "first"
            second = Path(directory) / "second"
            first.mkdir()
            second.mkdir()
            artifacts, source = self.copy_inputs(first)
            replacement, _ = self.copy_inputs(second)
            alias = Path(directory) / "selected"
            alias.symlink_to(artifacts, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "retained input drift"):
                with gate.retained_artifact_root(alias, source_root=source):
                    graph = self.first_graph(artifacts)
                    graph.write_bytes(graph.read_bytes() + b"\nselected graph changed")
                    alias.unlink()
                    alias.symlink_to(replacement, target_is_directory=True)

    def test_explicit_cli_selection_keeps_its_original_unpinned_input_flow(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts, _ = self.copy_inputs(directory)
            case = graph_tests.GuardedPointGraphTests("test_unknown_opcode_fails_closed")
            with patch.object(graph_tests, "ARTIFACTS", artifacts), \
                 patch.object(gate, "authenticate_retained_artifacts", side_effect=AssertionError("unexpected retained selection")):
                case.setUp()
                self.assertEqual(case.before, json.loads(self.first_graph(artifacts).read_text()))
                self.assertIs(graph_tests.ARTIFACTS, artifacts)
                self.assertTrue(case.doCleanups())
