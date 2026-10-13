# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Native tuple authority, nested trait policy, immutable history and no writes."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock

import recording_admission_inventory as inventory
import recording_admission_inventory_lexical_symbols as lexical_symbols
from recording_admission_inventory_lexical_symbols_test_support import install_historical_reads as install_symbol_reads
import recording_admission_inventory_shared_aggregate as shared_aggregate
from recording_admission_inventory_shared_aggregate_test_support import install_historical_reads as install_aggregate_reads
import recording_admission_inventory_shared_crypto as shared_crypto
from recording_admission_inventory_shared_crypto_test_support import install_historical_reads as install_crypto_reads
import recording_admission_inventory_shared_arithmetic as shared_arithmetic
from recording_admission_inventory_shared_arithmetic_test_support import install_historical_reads as install_arithmetic_reads
import recording_admission_inventory_shared_conversion as shared_conversion
from recording_admission_inventory_shared_conversion_test_support import install_historical_reads
import recording_admission_inventory_shared_leaf as shared_leaf
import recording_admission_inventory_unit_fragment as unit_fragment
import recording_admission_inventory_shared_coercion as shared_coercion
import recording_admission_inventory_target_edition as target_edition
import recording_admission_inventory_shared_pure as shared_pure
import recording_admission_inventory_native_tuple as native
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.native-tuple-621.json')


class NativeTupleTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        latest_root = unit_fragment.latest_checkpoint(
            shared_leaf.latest_checkpoint(
            shared_conversion.latest_checkpoint(
            shared_arithmetic.latest_checkpoint(
            shared_crypto.latest_checkpoint(
            shared_aggregate.latest_checkpoint(
            lexical_symbols.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE)
        latest_root = shared_coercion.latest_checkpoint(latest_root, inventory.BASELINE)
        latest_root = target_edition.latest_checkpoint(latest_root, inventory.BASELINE)
        self.source_root = shared_pure.latest_checkpoint(latest_root, inventory.BASELINE)
        self.root = _Overlay(self.source_root)
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.baseline_bytes)
        self.previous = {
            key: json.loads((inventory.ROOT / path).read_text())
            for key, path in self.extension['predecessor_options'].items()
        }
        install_symbol_reads(self, inventory.ROOT, inventory.BASELINE)
        install_aggregate_reads(self, inventory.ROOT, inventory.BASELINE)
        install_crypto_reads(self, inventory.ROOT, inventory.BASELINE)
        install_arithmetic_reads(self, inventory.ROOT, inventory.BASELINE)
        install_historical_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        args = {**self.previous, 'native_tuple_extension': self.extension, **overrides}
        return inventory.verify(self.root, self.baseline,
                                baseline_bytes=self.baseline_bytes, **args)

    def cli_arguments(self):
        args = ['--native-tuple-extension', str(MANIFEST)]
        for key, path in self.extension['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        return args

    def test_exact_signed_predecessor_preserves_all_route_accounting(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(native.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.baseline_bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        self.assertEqual(current['route_count'], 84)
        self.assertEqual(len(current['retired_routes']), 69)
        for name in ('retired_routes', 'route_checks', 'current_selector',
                     'reviewed_generic_qualification', 'reviewed_generic_retirement',
                     'reviewed_semantic_helper_discovery'):
            self.assertEqual(current[name], prior[name])
        extension = current['reviewed_native_tuple_extension']
        self.assertEqual(extension['new_retired_routes'], [])
        self.assertEqual(extension['historical_manifest_count'], 65)
        self.assertEqual(extension['original_oracles'], 49)
        closure = extension['production_closure']
        self.assertEqual(closure['native_guard_sites'], 7)
        self.assertEqual(closure['checked_operations'], 8)
        self.assertEqual(closure['equality_tuple_limit'], 12)
        self.assertEqual(closure['crypto_tuple_limit'], 11)

    def test_complete_owner_new_policy_and_every_historical_authority_reject_drift(self):
        for label in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            for owner in self.extension[label]:
                with self.subTest(label=label, owner=owner):
                    self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n'
                    self.assertFalse(self.report()['passed'])
                    self.root.changes.clear()

    def test_missing_forged_or_incomplete_prior_authority_fails(self):
        for overrides in ({'native_tuple_extension': None},
                          {'helper_discovery_transition': None},
                          {'generic_retirement_transition': None},
                          {'vector_map_extension': None},
                          {'native_tuple_extension': {**self.extension, 'claim': 'unreviewed'}}):
            with self.subTest(overrides=list(overrides)):
                self.assertFalse(self.report(**overrides)['passed'])

    def test_partial_inverse_false_retirement_or_incomplete_history_fails(self):
        for kind in ('inverse', 'retirement', 'history', 'new_source', 'selector'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][native.OWNER]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'new_source':
                forged['new_and_shared_sources'].pop(native.POLICY)
            else:
                forged['current_selector'] = {}
            self.assertFalse(self.report(native_tuple_extension=forged)['passed'])

    def test_specific_policy_resource_and_operation_site_mutations_fail(self):
        mutations = [
            (native.OWNER, 'mod native_representation;', '// bypassed native policy'),
            (native.OWNER, 'native_representation::check_tuple_traits(operation, &left_ty)?;', '// skipped equality trait'),
            (native.OWNER, 'native_representation::Operation::PersistentCommit,', 'native_representation::Operation::Keccak256,'),
            (native.POLICY, 'Self::Equal | Self::NotEqual => 12,', 'Self::Equal | Self::NotEqual => 13,'),
            (native.POLICY, '_ => 11,', '_ => 12,'),
            (native.POLICY, 'let nodes = recording_type_nodes(ty)?;', 'let nodes = vec![ty];'),
            (native.POLICY, '&& elements.len() > limit', '&& elements.len() < limit'),
            (native.POLICY, 'Self::Keccak256 => "Into<Value>",', 'Self::Keccak256 => "Aligned",'),
            (native.POLICY, 'Type::OpaqueString | Type::OpaqueBytes', 'Type::OpaqueString'),
            (native.BASE + '/resource_limits/borrowed_types.rs', 'children(visit, &mut self.scan)?;', '// skipped nested traversal'),
            (native.BASE + '/resource_limits/policy.rs', 'syntax_depth: 48,', 'syntax_depth: usize::MAX,'),
        ]
        for owner, before, after in mutations:
            with self.subTest(owner=owner, anchor=before):
                source = (self.source_root / owner).read_text()
                self.assertEqual(source.count(before), 1)
                self.root.changes[owner] = source.replace(before, after, 1).encode()
                with self.assertRaises(ValueError):
                    native.source_closure(self.root, self.extension)
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_operand_type_and_opening_error_order_cannot_move_after_trait_guard(self):
        source = (self.source_root / native.OWNER).read_text()
        before = '''            let (opening, actual) = expression_with_calls(opening, parameters, circuits)?;
            if actual != Type::Field {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Field,
                    actual,
                });
            }
            native_representation::check_tuple_traits(
                native_representation::Operation::TransientCommit,
                &value_type,
            )?;'''
        self.assertEqual(source.count(before), 1)
        guard = '''            native_representation::check_tuple_traits(
                native_representation::Operation::TransientCommit,
                &value_type,
            )?;'''
        after = guard + '\n' + before.replace('\n' + guard, '')
        self.root.changes[native.OWNER] = source.replace(before, after, 1).encode()
        with self.assertRaises(ValueError):
            native.source_closure(self.root, self.extension)
        self.assertFalse(self.report()['passed'])

    def test_all_original_oracles_are_checked_by_the_retained_authority_chain(self):
        authority = self.extension['original_oracle_preservation']['authority']
        original = json.loads((inventory.ROOT / authority).read_text())['files']
        self.assertEqual(len(original), 49)
        for row in original:
            with self.subTest(owner=row['path']):
                self.root.changes[row['path']] = (inventory.ROOT / row['path']).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_unrelated_recording_compiler_drift_is_not_hidden(self):
        self.root.changes[native.SELECTOR] = (self.source_root / native.SELECTOR).read_bytes() + b'\n'
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_any_transitive_source_or_authority(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        self.assertIn(inventory.ROOT / native.POLICY, paths)
        self.assertIn(inventory.ROOT / native.OWNER, paths)
        self.assertIn(inventory.ROOT / self.extension['original_oracle_preservation']['authority'], paths)
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
