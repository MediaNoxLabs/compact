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

import os
import json
from unittest.mock import patch
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import local_parity_gate as gate


class WorkspaceTestPlanTests(unittest.TestCase):
    def test_full_gate_reports_missing_fee_fixture_before_compiler_or_build(self):
        environment = os.environ.copy()
        environment.pop("MIDNIGHT_LEDGER_TEST_STATIC_DIR", None)
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = Path(temporary) / "gate"
            result = subprocess.run(
                [sys.executable, str(Path(gate.__file__)), "--full", "--compiler",
                 str(Path(temporary) / "missing-compiler"), "--run-dir", str(run_dir)],
                env=environment, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 1)
            self.assertIn("MIDNIGHT_LEDGER_TEST_STATIC_DIR", result.stderr)
            self.assertIn("ledger/static", result.stderr)
            self.assertFalse(run_dir.exists())

    def test_full_gate_invokes_unit_composition_registry_and_both_scenario_driver(self):
        import did_proof_gate
        calls = []
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            compiler = root / "compiler"
            compiler.write_text("#!/bin/sh\nexit 0\n")
            compiler.chmod(0o755)
            def record(command, label, directory, receipt, **kwargs):
                calls.append((label, command))
                if label == "workspace-test-metadata":
                    kwargs["stdout_path"].write_text(json.dumps({"workspace_members": [], "packages": []}))
                if label == "did-proof-lifecycles":
                    raise gate.GateError("controlled stop after mandatory DID invocation")
            with patch.object(sys, "argv", ["gate", "--full", "--compiler", str(compiler),
                    "--scheme", str(compiler), "--run-dir", str(root / "run")]), \
                 patch.dict(os.environ, {"MIDNIGHT_LEDGER_TEST_STATIC_DIR": str(root)}), \
                 patch.object(gate.shutil, "which", return_value=str(compiler)), \
                 patch.object(did_proof_gate, "prerequisites", return_value={}), \
                 patch.object(gate, "select_sources", return_value=[]), \
                 patch.object(gate.inventory, "source_paths", return_value=[]), \
                 patch.object(gate, "compare_baseline"), \
                 patch.object(gate.inventory, "receipt_metadata", return_value={}), \
                 patch.object(gate, "run", side_effect=record):
                self.assertEqual(gate.main(), 1)
            by_label = dict(calls)
            registry = by_label["unit-composition-source-scope"]
            self.assertEqual(registry[registry.index("--manifest") + 1],
                             str(gate.ROOT / "tools/compact-rust-backend/parity_positive_unit_composition_sources.json"))
            self.assertIn("did_proof_gate.py", by_label["did-proof-lifecycles"][1])
            self.assertIn("consumer-proof-ledger", by_label)
            self.assertNotIn("--skip", by_label["did-proof-lifecycles"])

    def test_generated_library_guard_falls_back_for_test_or_unknown_expansion(self):
        safe = '#[derive(Clone)]\npub fn f() { if !(true) { assert!(false); } }'
        self.assertTrue(gate.generated_library_has_no_test_hooks(safe))
        for extra in ['#[test] fn unit() {}', '#[cfg_attr(test, test)] fn unit() {}',
                      '#[cfg(test)] mod tests {}', 'include!("tests.rs");',
                      'mod hidden_tests;', 'custom_tests!();',
                      '#[derive(UnknownMacro)] struct T;', 'r#if!();']:
            with self.subTest(extra=extra):
                self.assertFalse(gate.generated_library_has_no_test_hooks(safe + extra))

    def test_added_inline_test_and_new_package_keep_exhaustive_selection(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / 'lib.rs'
            source.write_text('pub fn value() -> bool { true }')
            def package(name):
                return {'id': name, 'name': name, 'targets': [
                    {'name': name, 'kind': ['lib'], 'src_path': str(source)},
                    {'name': 'behavior', 'kind': ['test'], 'src_path': str(Path(tmp) / 'behavior.rs')}]}
            generated = package('generated')
            core = package('new-core')
            core['targets'][0]['src_path'] = str(Path(tmp) / 'unverified.rs')
            metadata = {'workspace_members': ['generated', 'new-core'], 'packages': [generated, core]}
            plan = gate.workspace_test_plan(metadata, {source})
            self.assertEqual(plan['integration_packages'], ['generated'])
            self.assertEqual(plan['all_target_packages'], ['new-core'])
            self.assertEqual(plan['omitted_empty_library_harnesses'][0]['integration_targets'], ['behavior'])
            source.write_text(source.read_text() + '\n#[test] fn added_unit() { assert!(value()); }')
            plan = gate.workspace_test_plan(metadata, {source})
            self.assertEqual(plan['integration_packages'], [])
            self.assertEqual(plan['all_target_packages'], ['generated', 'new-core'])
            command = gate.planned_test_commands(plan)[0][1]
            self.assertIn('--all-targets', command)
            self.assertNotIn('--test', command)

    def test_fixture_extra_target_or_no_integration_test_falls_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / 'lib.rs'
            source.write_text('pub fn value() {}')
            lib = {'name': 'fixture', 'kind': ['lib'], 'src_path': str(source)}
            test = {'name': 'behavior', 'kind': ['test'], 'src_path': str(source)}
            example = {'name': 'example', 'kind': ['example'], 'src_path': str(source)}
            for targets in [[lib], [lib, test, example]]:
                metadata = {'workspace_members': ['fixture'], 'packages': [
                    {'id': 'fixture', 'name': 'fixture', 'targets': targets}]}
                plan = gate.workspace_test_plan(metadata, {source})
                self.assertEqual(plan['all_target_packages'], ['fixture'])
                self.assertFalse(plan['integration_packages'])


if __name__ == '__main__':
    unittest.main()
