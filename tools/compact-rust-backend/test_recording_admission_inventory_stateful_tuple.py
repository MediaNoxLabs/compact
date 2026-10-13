# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Stateful tuple authority, evaluation order and immutable-history controls."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock

import recording_admission_inventory as inventory
import recording_admission_inventory_shared_coercion as shared_coercion
import recording_admission_inventory_target_edition as target_edition
import recording_admission_inventory_shared_pure as shared_pure
import recording_admission_inventory_stateful_tuple as stateful
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.stateful-tuple-624.json')


class StatefulTupleTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        latest_root = shared_coercion.latest_checkpoint(inventory.ROOT, inventory.BASELINE)
        latest_root = target_edition.latest_checkpoint(latest_root, inventory.BASELINE)
        self.source_root = shared_pure.latest_checkpoint(latest_root, inventory.BASELINE)
        self.root = _Overlay(self.source_root)
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.baseline_bytes)
        self.previous = {key: json.loads((inventory.ROOT / path).read_text())
                         for key, path in self.extension['predecessor_options'].items()}

    def report(self, **overrides):
        args = {**self.previous, 'stateful_tuple_extension': self.extension, **overrides}
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.baseline_bytes, **args)

    def cli_arguments(self):
        args = ['--stateful-tuple-extension', str(MANIFEST)]
        for key, path in self.extension['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        return args

    def test_exact_signed_predecessor_preserves_retirements_and_native_policy(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(stateful.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.baseline_bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        self.assertEqual(current['route_count'], 84)
        self.assertEqual(len(current['retired_routes']), 69)
        for name in ('retired_routes', 'route_checks', 'current_selector',
                     'reviewed_native_tuple_extension', 'reviewed_semantic_helper_discovery'):
            self.assertEqual(current[name], prior[name])
        extension = current['reviewed_stateful_tuple_extension']
        self.assertEqual(extension['new_retired_routes'], [])
        self.assertEqual(extension['historical_manifest_count'], 66)
        self.assertEqual(extension['original_oracles'], 49)
        closure = extension['production_closure']
        self.assertEqual(closure['native_guard_sites'], 6)
        self.assertEqual(closure['checked_operations'], 8)
        self.assertEqual((closure['equality_tuple_limit'], closure['crypto_tuple_limit']), (12, 11))
        self.assertEqual(closure['shared_policy_sha256'],
                         prior['reviewed_native_tuple_extension']['production_closure']['policy_sha256'])

    def test_complete_emitter_new_tests_shared_policy_and_all_history_reject_drift(self):
        for label in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            for owner in self.extension[label]:
                with self.subTest(label=label, owner=owner):
                    self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n'
                    self.assertFalse(self.report()['passed'])
                    self.root.changes.clear()

    def test_missing_and_forged_authorities_cannot_claim_extra_retirements(self):
        # #621 did not fingerprint this owner. Without #624 its report can pass,
        # but must not claim the new stateful emitter was authenticated.
        prior = self.report(stateful_tuple_extension=None)
        self.assertTrue(prior['passed'], prior['errors'])
        self.assertNotIn('reviewed_stateful_tuple_extension', prior)
        for overrides in ({'native_tuple_extension': None}, {'helper_discovery_transition': None}):
            self.assertFalse(self.report(**overrides)['passed'])
        for kind in ('inverse', 'retirement', 'history', 'policy', 'selector'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][stateful.OWNER]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'policy':
                forged['new_and_shared_sources'].pop(stateful.POLICY)
            else:
                forged['current_selector'] = {}
            self.assertFalse(self.report(stateful_tuple_extension=forged)['passed'])

    def test_each_shared_operation_guard_and_test_only_boundary_reject_mutation(self):
        source = (self.source_root / stateful.OWNER).read_text()
        mutations = [('use crate::native_representation;', '// bypassed shared policy'),
                     ('native_representation::check_tuple_traits(operation, &left_ty)?;', '// unchecked equality'),
                     ('native_representation::check_tuple_traits(tuple_operation, &input_type)?;', '// unchecked commitment'),
                     ('#[cfg(test)]\nmod tuple_capabilities;', 'mod tuple_capabilities;')]
        for op in ('Equal', 'NotEqual', 'TransientHash', 'PersistentHash', 'Keccak256',
                   'HashToCurve', 'TransientCommit', 'PersistentCommit'):
            mutations.append(('native_representation::Operation::' + op, '// removed ' + op))
        for before, after in mutations:
            with self.subTest(anchor=before):
                self.assertEqual(source.count(before), 1)
                self.root.changes[stateful.OWNER] = source.replace(before, after, 1).encode()
                with self.assertRaises(ValueError):
                    stateful.source_closure(self.root, self.extension)
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_opening_validation_and_evaluation_temporaries_cannot_be_reordered(self):
        source = (self.source_root / stateful.OWNER).read_text()
        before = '''            if actual != expected {
                return Err(RenderError::TypeMismatch { expected, actual });
            }
            let tuple_operation = if matches!(value, Expr::TransientCommit { .. }) {
                native_representation::Operation::TransientCommit
            } else {
                native_representation::Operation::PersistentCommit
            };
            native_representation::check_tuple_traits(tuple_operation, &input_type)?;'''
        self.assertEqual(source.count(before), 1)
        index = before.index('            let tuple_operation')
        reordered = before[index:] + '\n' + before[:index].rstrip()
        begin = source.index('        Expr::Equal { left, right } | Expr::NotEqual { left, right } => {',
                             source.index('fn render_arithmetic_operation('))
        equality = source[begin:source.index('        Expr::Compare {', begin)]
        left_binding = 'statements.push(syn::parse_quote!(let #left_name = #left;));'
        self.assertEqual(equality.count(left_binding), 1)
        mutations = [(before, reordered),
                     (equality, equality.replace(left_binding, '// lost left sequencing', 1)),
                     ('statements.push(syn::parse_quote!(let #input_name = #input;));', '// lost input sequencing')]
        for before, after in mutations:
            with self.subTest(anchor=before):
                self.assertEqual(source.count(before), 1)
                self.root.changes[stateful.OWNER] = source.replace(before, after, 1).encode()
                with self.assertRaises(ValueError):
                    stateful.source_closure(self.root, self.extension)
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_original_oracle_chain_and_unrelated_recording_authority_remain_live(self):
        authority = self.extension['original_oracle_preservation']['authority']
        original = json.loads((inventory.ROOT / authority).read_text())['files']
        self.assertEqual(len(original), 49)
        for owner in [row['path'] for row in original] + [stateful.SELECTOR]:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_cli_cannot_overwrite_any_transitive_source_or_authority(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (stateful.OWNER, stateful.POLICY,
                      self.extension['original_oracle_preservation']['authority']):
            self.assertIn(inventory.ROOT / owner, paths)
        for path in paths:
            before = path.read_bytes()
            with self.subTest(owner=str(path)), mock.patch.object(
                    Path, 'write_text', side_effect=AssertionError('authority write attempted')) as write:
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    inventory.main([*self.cli_arguments(), '--output', str(path)])
                write.assert_not_called()
            self.assertEqual(path.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
