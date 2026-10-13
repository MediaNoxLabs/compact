# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Shared pure slice A source authority; full expression-body fold is pending."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock

import recording_admission_inventory as inventory
import recording_admission_inventory_shared_leaf as shared_leaf
import recording_admission_inventory_unit_fragment as unit_fragment
import recording_admission_inventory_shared_coercion as shared_coercion
import recording_admission_inventory_target_edition as target_edition
import recording_admission_inventory_shared_pure as shared
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.shared-pure-semantics-622a.json')


class SharedPureTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        latest_root = unit_fragment.latest_checkpoint(
            shared_leaf.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE)
        latest_root = shared_coercion.latest_checkpoint(latest_root, inventory.BASELINE)
        self.source_root = target_edition.latest_checkpoint(latest_root, inventory.BASELINE)
        self.root = _Overlay(self.source_root)
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.baseline_bytes)
        self.previous = {key: json.loads((inventory.ROOT / path).read_text())
                         for key, path in self.extension['predecessor_options'].items()}
        self.predecessor = shared.predecessor_sources(self.root, self.extension)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.baseline_bytes,
                                **{**self.previous, 'shared_pure_semantics_extension': self.extension,
                                   **overrides})

    def cli_arguments(self):
        args = ['--shared-pure-semantics-extension', str(MANIFEST)]
        for key, path in self.extension['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        return args

    def test_exact_signed_predecessor_retains_all_counts_and_every_earlier_report(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(shared.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.baseline_bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[key], prior[key])
        for key in prior:
            if key.startswith('reviewed_'):
                self.assertEqual(current[key], prior[key])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        summary = current['reviewed_shared_pure_semantics_extension']
        self.assertEqual(summary['new_retired_routes'], [])
        self.assertEqual((summary['historical_manifest_count'], summary['original_oracles']), (67, 49))
        self.assertIn('still pending', summary['production_closure']['body_syntax_bridge'])

    def test_complete_owners_new_policies_history_and_original_oracles_reject_drift(self):
        paths = set()
        for label in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            paths.update(self.extension[label])
        authority = self.extension['original_oracle_preservation']['authority']
        original = json.loads((inventory.ROOT / authority).read_text())['files']
        self.assertEqual(len(original), 49)
        paths.update(row['path'] for row in original)
        paths.add(shared.SELECTOR)
        for owner in paths:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_missing_forged_partial_or_unqualified_latest_checkpoint_fails(self):
        for overrides in ({'shared_pure_semantics_extension': None},
                          {'stateful_tuple_extension': None}, {'native_tuple_extension': None}):
            self.assertFalse(self.report(**overrides)['passed'])
        for kind in ('inverse', 'retirement', 'history', 'new_source', 'completion'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][shared.OWNERS[0]]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'new_source':
                forged['new_and_shared_sources'].pop(next(iter(forged['new_and_shared_sources'])))
            else:
                forged['claim'] = 'body bridge removed and epic complete'
            with self.subTest(kind=kind):
                self.assertFalse(self.report(shared_pure_semantics_extension=forged)['passed'])
                with self.assertRaises(ValueError):
                    shared.checkpoint(self.root, forged)

    def mutation(self, owner, before, after):
        source = (self.source_root / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaises(ValueError):
            shared.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_neutral_policies_and_output_sinks_cannot_acquire_syntax_or_refusal_authority(self):
        for leaf in ('identifier', 'numeric', 'carrier'):
            owner = shared.BASE + '/pure_semantics/' + leaf + '.rs'
            with self.subTest(owner=owner):
                self.mutation(owner, '#[cfg(test)]\nmod tests;',
                              'fn creates_syntax() { syn::parse_quote!(()); }\n#[cfg(test)]\nmod tests;')
        self.mutation(shared.BASE + '/pure_semantics/carrier.rs',
                      'fn leaf(&mut self, leaf: Leaf<\'_>) -> Self::Value;',
                      'fn leaf(&mut self, leaf: Leaf<\'_>) -> Result<Self::Value, RenderError>;')
        self.mutation(shared.BASE + '/value_lowering/carrier.rs',
                      'pub(crate) struct NativeCarrierSink;',
                      'pub(crate) struct NativeCarrierSink;\nfn refuse() -> Result<(), RenderError> { todo!() }')
        self.mutation(shared.BASE + '/naming.rs',
                      '/// Output adapter: no lexical or name-support decisions occur here.',
                      'fn repeats_parser() { syn::parse_str::<syn::Ident>("x"); }')

    def test_one_bounded_carrier_fold_and_both_consumers_cannot_be_bypassed(self):
        mutations = [
            (shared.BASE + '/pure_semantics/carrier.rs', 'drop(recording_type_nodes(ty)?);', '// unbounded fold'),
            (shared.BASE + '/pure_semantics/carrier.rs', 'type Value = ();', 'type Value = syn::Type;'),
            (shared.BASE + '/value_lowering.rs',
             'crate::pure_semantics::carrier::fold_carrier(ty, &mut carrier::NativeCarrierSink)',
             'legacy_carrier_match(ty)'),
            (shared.OWNERS[-1], 'crate::pure_semantics::carrier::check_carrier(&source.result)?;',
             'crate::rust_type(&source.result)?;'),
            (shared.BASE + '/value_lowering.rs', 'checked_unsigned_cast(source_max, target_max)?',
             'legacy_cast_match(source_max, target_max)?'),
        ]
        for owner, before, after in mutations:
            with self.subTest(owner=owner, anchor=before):
                self.mutation(owner, before, after)

    def test_body_syntax_bridge_is_recorded_as_pending_and_not_a_completed_fold(self):
        self.mutation(shared.OWNERS[-1],
                      'crate::expression_with_calls(&source.body, &scope, pure)?;',
                      'unchecked_body_accept(&source.body)?;')
        self.mutation(shared.OWNERS[-1],
                      'crate::naming::parameter_idents(&source.parameters, &[])',
                      'Vec::new()')
        closure = shared.source_closure(self.root, self.extension, self.predecessor)
        self.assertIn('pending', closure['body_syntax_bridge'])
        self.assertEqual(closure['new_retired_routes'], [])

    def test_each_lock_uses_its_own_predecessor_and_no_package_cohort_upgrade_is_hidden(self):
        result = shared.dependency_closure(self.root, self.predecessor)
        self.assertEqual(set(result), {'Cargo.lock', 'tools/compact-rust-backend/Cargo.lock'})
        for owner in result:
            source = (self.source_root / owner).read_text()
            package = 'name = "unicode-ident"\nversion = "1.0.24"'
            self.assertEqual(source.count(package), 1)
            for replacement in ('name = "unicode-ident"\nversion = "1.0.25"',
                                'name = "unicode-ident"\nversion = "1.0.24"\nnew_field = true'):
                with self.subTest(owner=owner, replacement=replacement):
                    self.root.changes[owner] = source.replace(package, replacement, 1).encode()
                    with self.assertRaises(ValueError):
                        shared.dependency_closure(self.root, self.predecessor)
                    self.assertFalse(self.report()['passed'])
                    self.root.changes.clear()
        wrong = {**self.predecessor, 'Cargo.lock': self.predecessor['tools/compact-rust-backend/Cargo.lock']}
        with self.assertRaises(ValueError):
            shared.dependency_closure(self.root, wrong)

    def test_cli_protects_all_transitive_locks_policies_owners_and_historical_authorities(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (*shared.OWNERS, shared.BASE + '/pure_semantics/carrier.rs'):
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
