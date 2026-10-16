# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Archive isolation and refusal laws for the single historical baseline case."""
import os
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import conformance_baseline_archive as archive


class ArchiveFixtureTests(unittest.TestCase):
    def setUp(self):
        # Default only at this test caller; explicit overrides and the strict
        # authenticator keep their original refusal behavior. patch restores
        # the prior environment after each case, including failures.
        self.enterContext(patch.dict(os.environ, {
            archive.ENVIRONMENT: str(archive.selected_test_root())}))

    def copy_inputs(self, directory):
        source = os.environ.get(archive.ENVIRONMENT)
        if source is None:
            self.fail("these archive controls require explicit " + archive.ENVIRONMENT)
        target = Path(directory) / "private archive"
        shutil.copytree(source, target)
        return target.resolve()

    def first_source(self, root):
        baseline = archive.conformance.load_json(
            Path(archive.conformance.__file__).with_suffix(".json").read_text())
        return root / next(iter(baseline["sources"]))

    def test_missing_explicit_root_has_no_current_root_fallback(self):
        before = archive.conformance.ROOT
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(ValueError, "explicit retained conformance inputs"):
                with archive.repository_baseline_root():
                    self.fail("missing input acquired archive context")
        self.assertEqual(archive.conformance.ROOT, before)

    def test_relative_missing_and_current_roots_refuse(self):
        with self.assertRaisesRegex(ValueError, "must be absolute"):
            archive.authenticate_archive("relative")
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(FileNotFoundError):
                archive.authenticate_archive(Path(directory) / "absent")
        with self.assertRaisesRegex(ValueError, "archive refused"):
            archive.authenticate_archive(archive.conformance.ROOT)

    def test_baseline_authority_cannot_be_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(Path, "read_bytes", return_value=b"{}"):
                with self.assertRaisesRegex(ValueError, "baseline authority changed"):
                    archive.authenticate_archive(directory)

    def test_only_context_root_changes_and_normal_cleanup_restores_it(self):
        before = archive.conformance.ROOT
        with tempfile.TemporaryDirectory() as directory:
            root = self.copy_inputs(directory)
            with archive.repository_baseline_root(root) as receipt:
                self.assertEqual(archive.conformance.ROOT, root)
                self.assertEqual(receipt["case_id"], archive.CASE_ID)
                self.assertIn("not current-source conformance", receipt["scope"])
            self.assertEqual(archive.conformance.ROOT, before)

    def test_test_failure_restores_current_root(self):
        before = archive.conformance.ROOT
        with tempfile.TemporaryDirectory() as directory:
            root = self.copy_inputs(directory)
            with self.assertRaisesRegex(RuntimeError, "original test failure"):
                with archive.repository_baseline_root(root):
                    raise RuntimeError("original test failure")
        self.assertEqual(archive.conformance.ROOT, before)

    def test_content_mutation_and_deletion_never_fall_back(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.copy_inputs(directory)
            source = self.first_source(root)
            original = source.read_bytes()
            source.write_bytes(original + b"\nmutation")
            with self.assertRaisesRegex(ValueError, "archive refused"):
                archive.authenticate_archive(root)
            source.unlink()
            with self.assertRaises(FileNotFoundError):
                archive.authenticate_archive(root)

    def test_mutation_during_context_is_live_and_root_is_restored(self):
        before = archive.conformance.ROOT
        with tempfile.TemporaryDirectory() as directory:
            root = self.copy_inputs(directory)
            with self.assertRaisesRegex(ValueError, "archive refused"):
                with archive.repository_baseline_root(root):
                    source = self.first_source(root)
                    source.write_bytes(source.read_bytes() + b"\nchanged while running")
        self.assertEqual(archive.conformance.ROOT, before)

    def test_current_source_refusal_remains_live_after_archived_case(self):
        before = archive.conformance.ROOT
        baseline = archive.conformance.load_json(
            Path(archive.conformance.__file__).with_suffix(".json").read_text())
        first = archive.conformance.check(before, baseline["grammar_inventory"], baseline)
        self.assertEqual(first["status"], "failed")
        self.assertTrue(first["errors"])
        with archive.repository_baseline_root():
            self.assertNotEqual(archive.conformance.ROOT, before)
        second = archive.conformance.check(before, baseline["grammar_inventory"], baseline)
        self.assertEqual(second, first)
        self.assertEqual(archive.conformance.ROOT, before)

    def test_symlink_escape_cannot_supply_a_matching_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.copy_inputs(directory)
            source = self.first_source(root)
            outside = Path(directory) / "same-bytes-outside"
            outside.write_bytes(source.read_bytes())
            source.unlink()
            source.symlink_to(outside)
            with self.assertRaisesRegex(ValueError, "escapes repository"):
                archive.authenticate_archive(root)

    def test_root_alias_retarget_cannot_hide_mutation_of_selected_inputs(self):
        before = archive.conformance.ROOT
        with tempfile.TemporaryDirectory() as directory:
            first = Path(directory) / "first"
            second = Path(directory) / "second"
            first.mkdir()
            second.mkdir()
            selected = self.copy_inputs(first)
            replacement = self.copy_inputs(second)
            alias = Path(directory) / "selected"
            alias.symlink_to(selected, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "archive refused"):
                with archive.repository_baseline_root(alias):
                    source = self.first_source(selected)
                    source.write_bytes(source.read_bytes() + b"\nselected source changed")
                    alias.unlink()
                    alias.symlink_to(replacement, target_is_directory=True)
        self.assertEqual(archive.conformance.ROOT, before)
