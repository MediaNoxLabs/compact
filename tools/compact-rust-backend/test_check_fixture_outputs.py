#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

"""Regression controls for retained outputs, separate from current-emitter hosts."""

import contextlib
import io
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import check_fixture_outputs as checker
import fixture_inventory as inventory


class RetainedEmissionTests(unittest.TestCase):
    def run_check(self, root, row, source_bytes, *, update=False, compile_exit=0):
        commands = []

        def execute(command, **kwargs):
            commands.append(command)
            if command[0] == "test-compactc":
                if compile_exit:
                    return subprocess.CompletedProcess(command, compile_exit, "", "compile refused")
                contract = Path(command[-1]) / "contract"
                contract.mkdir()
                (contract / "lib.rs").write_bytes(source_bytes)
                (contract / "rust-capabilities.json").write_text(json.dumps({
                    "schema_version": 3, "circuits": []}))
            return subprocess.CompletedProcess(command, 0, "", "")

        with patch.object(checker, "ROOT", root), \
             patch.object(inventory, "source_registry", return_value=(row,)), \
             patch.object(checker, "run", side_effect=execute), \
             patch.dict(checker.os.environ, {"COMPACTC": "test-compactc"}), \
             patch.object(checker.sys, "argv", ["check_fixture_outputs.py", *(["--update"] if update else [])]), \
             contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            result = checker.main()
        return result, commands

    def registration(self, root):
        source = root / "examples/rust_backend/circuit.compact"
        return SimpleNamespace(source=source, role=inventory.SourceRole.GENERATED_CONSUMER,
                               flat_fixture=None, frontend_flags=("--feature-zkir-v3",))

    def test_current_output_drift_is_rejected_and_never_updates_the_oracle(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            row = self.registration(root)
            snapshot = inventory.emission_fixture(row, root)
            snapshot.parent.mkdir(parents=True)
            snapshot.write_bytes(b"reviewed emitter output\n")
            self.assertEqual(self.run_check(root, row, snapshot.read_bytes())[0], 0)
            # Even if both frontend and Cargo use this new emitter output, the
            # independent reviewed oracle must still reject its unreviewed drift.
            result, commands = self.run_check(root, row, b"new current-emitter output\n")
            self.assertEqual(result, 1)
            self.assertEqual(snapshot.read_bytes(), b"reviewed emitter output\n")
            self.assertIn("--feature-zkir-v3", commands[0])

    def test_missing_oracle_is_a_failure_until_explicit_update(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            row = self.registration(root)
            snapshot = inventory.emission_fixture(row, root)
            self.assertEqual(self.run_check(root, row, b"fresh output\n")[0], 1)
            self.assertFalse(snapshot.exists())
            self.assertEqual(self.run_check(root, row, b"fresh output\n", update=True)[0], 0)
            self.assertEqual(snapshot.read_bytes(), b"fresh output\n")

    def test_failed_compilation_cannot_replace_an_existing_oracle(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            row = self.registration(root)
            snapshot = inventory.emission_fixture(row, root)
            snapshot.parent.mkdir(parents=True)
            snapshot.write_bytes(b"reviewed output\n")
            self.assertEqual(self.run_check(root, row, b"bad output", update=True, compile_exit=1)[0], 1)
            self.assertEqual(snapshot.read_bytes(), b"reviewed output\n")


if __name__ == "__main__":
    unittest.main()
