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

import tempfile
import unittest
from pathlib import Path

import local_parity_gate as gate


class WorkspaceTestPlanTests(unittest.TestCase):
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
