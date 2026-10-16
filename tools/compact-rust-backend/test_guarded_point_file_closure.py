# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Synthetic file mechanics only: no retained graph, compiler or proof evidence.

The public retained entry point keeps its original immutable authority. These
unit tests supply a separate authority to the internal file primitive; they do
not patch the retained authority, digest function, filesystem or assertions.
"""
from contextlib import contextmanager
from copy import deepcopy
from functools import partial
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import guarded_point_zkir as gate


class SyntheticInputClosure:
    """One private, disposable file closure; never a historical artifact fixture."""

    def __init__(self, directory):
        self.root = Path(directory) / "synthetic artifacts"
        self.source = Path(directory) / "synthetic source"
        self.root.mkdir(parents=True)
        self.source.mkdir()
        self.authority = {"artifact_fingerprints": {}, "authority_fingerprints": {}}
        for table, files in (
                ("artifact_fingerprints", ("graphs/before", "graphs/after")),
                ("authority_fingerprints", ("authority/one", "authority/two"))):
            for relative in files:
                content = ("synthetic file mechanics: " + relative).encode()
                path = self.root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
                self.authority[table][relative] = hashlib.sha256(content).hexdigest()
        source = self.source / "fixture.compact"
        source.write_bytes(b"synthetic source identity; not a compiled contract")
        self.authority["source"] = {"path": "fixture.compact", "sha256": gate.digest(source)}
        self.authenticate = partial(gate._authenticate_file_closure, authority=self.authority)

    def receipt(self):
        return self.authenticate(self.root, source_root=self.source)

    def selected(self, root=None, source=None):
        return gate._authenticated_artifact_root(
            self.root if root is None else root,
            source_root=self.source if source is None else source,
            authenticate=self.authenticate)

    def targets(self):
        return [self.root / p for table in ("artifact_fingerprints", "authority_fingerprints")
                for p in self.authority[table]] + [self.source / self.authority["source"]["path"]]


class FileClosureTests(unittest.TestCase):
    @contextmanager
    def fixture(self):
        with tempfile.TemporaryDirectory() as directory:
            yield SyntheticInputClosure(directory)

    def test_positive_receipt_only_identifies_canonical_roots(self):
        with self.fixture() as fixture:
            self.assertEqual(fixture.receipt(), {
                "root": str(fixture.root.resolve()),
                "source_root": str(fixture.source.resolve()),
            })
            self.assertNotIn("producer_identities", fixture.receipt())
            self.assertNotIn("qualification", fixture.receipt())

    def test_every_file_content_is_checked_on_each_read(self):
        with self.fixture() as fixture:
            for target in fixture.targets():
                with self.subTest(path=target.relative_to(target.parents[1])):
                    original = target.read_bytes()
                    fixture.receipt()
                    target.write_bytes(original + b" changed")
                    with self.assertRaisesRegex(ValueError, "drift"):
                        fixture.receipt()
                    target.write_bytes(original)
                    fixture.receipt()

    def test_every_file_deletion_is_live(self):
        with self.fixture() as fixture:
            for target in fixture.targets():
                with self.subTest(file=target.name):
                    original = target.read_bytes()
                    fixture.receipt()
                    target.unlink()
                    with self.assertRaises(FileNotFoundError):
                        fixture.receipt()
                    target.write_bytes(original)

    def test_symlink_escape_refuses_matching_bytes_for_all_input_kinds(self):
        with self.fixture() as fixture:
            for target in fixture.targets():
                with self.subTest(file=target.name):
                    original = target.read_bytes()
                    outside = fixture.root.parent / "outside"
                    outside.write_bytes(original)
                    target.unlink()
                    target.symlink_to(outside)
                    with self.assertRaisesRegex(ValueError, "escapes root"):
                        fixture.receipt()
                    target.unlink()
                    target.write_bytes(original)

    def test_descriptor_paths_cannot_escape_or_be_absolute(self):
        with self.fixture() as fixture:
            outside = fixture.root.parent / "outside"
            outside.write_bytes(b"synthetic escape")
            for relative in ("../outside", str(outside)):
                with self.subTest(path=relative):
                    authority = deepcopy(fixture.authority)
                    authority["artifact_fingerprints"] = {relative: gate.digest(outside)}
                    with self.assertRaisesRegex(ValueError, "escapes root"):
                        gate._authenticate_file_closure(fixture.root, source_root=fixture.source,
                                                       authority=authority)

    def test_source_descriptor_cannot_escape(self):
        with self.fixture() as fixture:
            authority = deepcopy(fixture.authority)
            authority["source"] = {"path": "../synthetic artifacts/graphs/before",
                                   "sha256": gate.digest(fixture.targets()[0])}
            with self.assertRaisesRegex(ValueError, "escapes root"):
                gate._authenticate_file_closure(fixture.root, source_root=fixture.source,
                                               authority=authority)

    def test_invalid_explicit_roots_refuse_without_fallback(self):
        with self.fixture() as fixture:
            for value, exception in ((None, ValueError), ("", ValueError),
                                     (Path("relative"), ValueError),
                                     (fixture.root / "missing", FileNotFoundError)):
                with self.subTest(value=value), self.assertRaises(exception):
                    fixture.authenticate(value, source_root=fixture.source)
            with self.assertRaisesRegex(ValueError, "must be a directory"):
                fixture.authenticate(fixture.targets()[0], source_root=fixture.source)

    def test_context_keeps_the_selected_canonical_artifact_root(self):
        with self.fixture() as first, self.fixture() as replacement:
            alias = first.root.parent / "alias"
            alias.symlink_to(first.root, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "drift"):
                with first.selected(root=alias) as selected:
                    self.assertEqual(selected, first.root.resolve())
                    first.targets()[0].write_bytes(b"selected input changed")
                    alias.unlink()
                    alias.symlink_to(replacement.root, target_is_directory=True)

    def test_context_keeps_the_selected_canonical_source_root(self):
        with self.fixture() as first, self.fixture() as replacement:
            alias = first.source.parent / "source alias"
            alias.symlink_to(first.source, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "current fixture drift"):
                with first.selected(source=alias):
                    first.targets()[-1].write_bytes(b"selected source changed")
                    alias.unlink()
                    alias.symlink_to(replacement.source, target_is_directory=True)

    def test_cleanup_authenticates_each_kind_of_input(self):
        with self.fixture() as fixture:
            for target in fixture.targets():
                with self.subTest(file=target.name):
                    original = target.read_bytes()
                    with self.assertRaisesRegex(ValueError, "drift"):
                        with fixture.selected():
                            target.write_bytes(original + b" changed during case")
                    target.write_bytes(original)

    def test_unchanged_cleanup_preserves_original_exception(self):
        with self.fixture() as fixture:
            failure = RuntimeError("original failure")
            with self.assertRaises(RuntimeError) as caught:
                with fixture.selected():
                    raise failure
            self.assertIs(caught.exception, failure)
            fixture.receipt()

    def test_cleanup_refusal_retains_the_original_exception_context(self):
        with self.fixture() as fixture:
            failure = RuntimeError("case and cleanup both fail")
            with self.assertRaisesRegex(ValueError, "drift") as caught:
                with fixture.selected():
                    fixture.targets()[0].write_bytes(b"changed")
                    raise failure
            self.assertIs(caught.exception.__context__, failure)

    def test_fixtures_are_isolated_and_removed(self):
        with self.fixture() as first, self.fixture() as second:
            with first.selected(), second.selected():
                self.assertNotEqual(first.root, second.root)
                self.assertEqual(first.targets()[0].read_bytes(), second.targets()[0].read_bytes())
            first.targets()[0].write_bytes(b"only first changed")
            second.receipt()
            with self.assertRaises(ValueError):
                first.receipt()
            roots = (first.root, first.source, second.root, second.source)
        self.assertTrue(all(not p.exists() for p in roots))

    def test_synthetic_inputs_cannot_authenticate_as_retained_material(self):
        original = json.dumps(gate.RETAINED_GRAPH_AUTHORITY, sort_keys=True)
        with self.fixture() as fixture:
            for relative in gate.RETAINED_GRAPH_AUTHORITY["artifact_fingerprints"]:
                target = fixture.root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(fixture.targets()[0], target)
            with self.assertRaisesRegex(ValueError, "retained input drift"):
                gate.authenticate_retained_artifacts(fixture.root, source_root=fixture.source)
        self.assertEqual(json.dumps(gate.RETAINED_GRAPH_AUTHORITY, sort_keys=True), original)

    def test_cleanup_detects_deletion(self):
        with self.fixture() as fixture:
            for target in fixture.targets():
                with self.subTest(file=target.name):
                    original = target.read_bytes()
                    with self.assertRaises(FileNotFoundError):
                        with fixture.selected():
                            target.unlink()
                    target.write_bytes(original)

    def test_cleanup_detects_symlink_escape(self):
        with self.fixture() as fixture:
            for target in fixture.targets():
                with self.subTest(file=target.name):
                    original = target.read_bytes()
                    outside = fixture.root.parent / "outside"
                    outside.write_bytes(original)
                    with self.assertRaisesRegex(ValueError, "escapes root"):
                        with fixture.selected():
                            target.unlink()
                            target.symlink_to(outside)
                    target.unlink()
                    target.write_bytes(original)


class RetainedReceiptTests(unittest.TestCase):
    def test_projection_preserves_fixed_metadata_after_file_authentication(self):
        # This is receipt assembly only. It does not authenticate synthetic files
        # as historical artifacts, and cannot supply another retained authority.
        authority = deepcopy(gate.RETAINED_GRAPH_AUTHORITY)
        roots = {"root": "/unit/selected", "source_root": "/unit/source"}
        with patch.object(gate, "_authenticate_file_closure", return_value=roots) as checked:
            receipt = gate.authenticate_retained_artifacts("selected", source_root="source")
        checked.assert_called_once_with("selected", source_root="source",
                                        authority=gate.RETAINED_GRAPH_AUTHORITY)
        for field in ("artifact_fingerprints", "authority_fingerprints", "source",
                      "producer_identities"):
            self.assertEqual(receipt[field], authority[field])
            self.assertIsNot(receipt[field], gate.RETAINED_GRAPH_AUTHORITY[field])
        self.assertEqual(receipt["root"], roots["root"])
        self.assertEqual(receipt["source_root"], roots["source_root"])
        self.assertIn("not fresh current", receipt["qualification"])
        self.assertEqual(gate.RETAINED_GRAPH_AUTHORITY, authority)
