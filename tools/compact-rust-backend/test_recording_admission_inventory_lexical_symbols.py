# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Native Let checked-symbol ownership and immutable signed-D3 law guards."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import recording_admission_inventory as inventory
import recording_admission_inventory_native_control_fold as native_control_fold
from recording_admission_inventory_native_control_fold_test_support import install_historical_reads as install_native_fold_reads
import recording_admission_inventory_lexical_symbols as symbols
import recording_admission_inventory_shared_aggregate as aggregate
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
from recording_admission_inventory_lexical_symbols_test_support import (
    install_historical_reads, qualified_read_data,
)
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.lexical-symbols-629d4a.json')


class LexicalSymbolTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(native_control_fold.latest_checkpoint(inventory.ROOT, inventory.BASELINE))
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = symbols.predecessor_sources(self.root, self.extension)
        install_native_fold_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'lexical_symbols_extension': self.extension, **overrides})

    def args(self):
        args = ['--lexical-symbols-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args

    def mutation(self, owner, before, after, diagnostic):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaisesRegex(ValueError, diagnostic):
            symbols.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_actual_D3_and_prior_reports_counts_native_scope_and_pending_body_are_preserved(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(symbols.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[key], prior[key])
        for key in prior:
            if key.startswith('reviewed_'):
                self.assertEqual(current[key], prior[key])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_lexical_symbols_extension']
        self.assertEqual((stage['issue'], stage['parent_issue'], stage['slice']), (629, 622, 'D4a'))
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (76, 49))
        self.assertIn('pending622', stage['production_closure']['body_syntax_bridge'])
        self.assertIn('same raw key exempt', stage['production_closure']['allocation'])
        for key in ('reviewed_shared_aggregate_extension', 'reviewed_shared_crypto_extension',
                    'reviewed_shared_arithmetic_extension', 'reviewed_shared_conversion_extension',
                    'reviewed_shared_leaf_extension', 'reviewed_unit_fragment_extension', 'reviewed_shared_coercion_extension'):
            report = symbols.verify_extension(self.root, self.baseline, self.extension,
                                                lambda _: {**prior, key: {}}, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('actual D3/D2/D1c/D1b/D1a/C/B', report['errors'][0])
        for key in ('shared_aggregate_extension', 'shared_crypto_extension', 'shared_arithmetic_extension',
                    'shared_conversion_extension', 'shared_leaf_extension', 'unit_fragment_extension',
                    'shared_coercion_extension', 'lexical_symbols_extension'):
            self.assertFalse(self.report(**{key: None})['passed'])

    def test_complete_current_sources_history_original49_and_recording_selector_refuse_drift(self):
        paths = {symbols.SELECTOR}
        for key in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            paths.update(self.extension[key])
        original = json.loads((inventory.ROOT / self.extension['original_oracle_preservation']['authority']).read_text())['files']
        self.assertEqual(len(original), 49)
        paths.update(row['path'] for row in original)
        for owner in paths:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (inventory.ROOT / owner).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_forged_missing_capture_allocation_owner_history_scope_or_retirement_refuses_delivery(self):
        for kind in ('capture', 'allocation', 'owner', 'history', 'scope', 'retirement'):
            forged = copy.deepcopy(self.extension)
            if kind in ('capture', 'allocation'):
                forged['owners'][symbols.LIB]['predecessor_reconstruction'].pop(0 if kind == 'capture' else 1)
            elif kind == 'owner':
                forged['owners'].pop(symbols.NATIVE)
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'scope':
                forged['claim'] = 'generic control fully migrated; all names/all language formally covered'
            else:
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            self.assertFalse(self.report(lexical_symbols_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                symbols.checkpoint(self.root, forged)

    def test_checked_capture_prior_initializer_scope_raw_lookup_and_neighboring_control_stay_exact(self):
        self.mutation(symbols.LIB, 'let checked_name = pure_semantics::identifier::check_name(&binding.name)?;',
                      'let checked_name = pure_semantics::identifier::check_name("renamed")?;',
                      'current Let checked_name boundary changed')
        self.mutation(symbols.LIB, 'locals.iter().map(|(key, (_, symbol))| (*key, symbol))',
                      'locals.iter().map(|(_, (_, symbol))| ("renamed", symbol))', 'current Let allocation boundary changed')
        self.mutation(symbols.LIB, 'statements.push(syn::parse_quote!(let #local_name: #local_ty = #value;));\n                locals.insert(binding.name.as_str(), (&binding.ty, local_name));',
                      'statements.push(syn::parse_quote!(let #local_name: #local_ty = #value;));\n                locals.insert("renamed", (&binding.ty, local_name));', 'control owner outside approved Let slots changed')
        self.mutation(symbols.LIB, 'evaluate_condition: condition_needs_statement(condition),',
                      'evaluate_condition: true,', 'control owner outside approved Let slots changed')
        for name in ('pure_aggregate_expression', 'pure_crypto_expression', 'pure_arithmetic_expression', 'pure_conversion_expression'):
            anchor = 'fn ' + name + '('
            self.mutation(symbols.LIB, anchor, anchor + '\n// changed neighboring family\n',
                          'accepted semantic owner/entry changed: ' + name)
        self.mutation(symbols.TEST_MODULE, 'mod local_symbols;',
                      'mod local_symbols;\n// prior test owner changed', 'existing native test owner changed')

    def test_output_allocator_excludes_only_same_raw_key_normalizes_live_occupancy_and_has_no_rechecking(self):
        self.mutation(symbols.LOCAL, '.filter(|(key, _)| *key != raw_source_key)',
                      '.filter(|(key, _)| *key == raw_source_key)', 'native Let symbol boundary changed')
        self.mutation(symbols.LOCAL, 'spelling.strip_prefix("r#").unwrap_or(&spelling).to_owned()',
                      'spelling.to_owned()', 'native Let symbol boundary changed')
        self.mutation(symbols.LOCAL, 'checked_name.text()',
                      'raw_source_key', 'native Let symbol boundary changed')
        self.mutation(symbols.LOCAL, 'while occupied.contains(&candidate)',
                      'while !occupied.contains(&candidate)', 'native Let symbol boundary changed')
        self.mutation(symbols.LOCAL, 'pub(crate) fn allocate',
                      'fn validate_again() { check_name(); }\npub(crate) fn allocate',
                      'output symbol allocator acquired source checking/global state')
        for owner in ('naming.rs', 'pure_semantics/identifier.rs', 'pure_native/control.rs',
                      'pure_native/fragment.rs', 'pure_native/aggregate/vector.rs', 'pure_native/item.rs'):
            path = symbols.BASE + '/' + owner
            self.root.changes[path] = (inventory.ROOT / path).read_bytes() + b'\n'
            with self.assertRaisesRegex(ValueError, 'protected current source changed'):
                symbols.source_closure(self.root, self.extension, self.predecessor)
            self.assertFalse(self.report()['passed'])
            self.root.changes.clear()

    def test_authentic_historical_control_mutations_survive_and_transitive_outputs_cannot_overwrite_authority(self):
        actual = {owner: (inventory.ROOT / owner).read_bytes() for owner in self.extension['owners']}
        for owner, data in qualified_read_data(self.root, self.extension).items():
            self.assertEqual(digest(data.encode()), self.extension['owners'][owner]['previous_sha256'])
        self.root.changes[symbols.LIB] = actual[symbols.LIB].decode().replace(
            'let checked_name =', 'let corrupted_name =', 1).encode()
        with self.assertRaisesRegex(ValueError, 'current owner changed'):
            qualified_read_data(self.root, self.extension)
        self.root.changes.clear()
        forged = copy.deepcopy(self.extension)
        forged['owners'][symbols.LIB]['predecessor_reconstruction'][0]['current'] += '\n'
        with self.assertRaises(ValueError):
            qualified_read_data(self.root, forged)
        old_root = symbols.checkpoint(self.root, self.extension)
        old_stage = self.previous['shared_aggregate_extension']
        old_sources = aggregate.predecessor_sources(old_root, old_stage)
        case = unittest.TestCase()
        try:
            install_historical_reads(case, inventory.ROOT, inventory.BASELINE)
            for owner, row in self.extension['owners'].items():
                self.assertEqual(digest((inventory.ROOT / owner).read_bytes()), row['previous_sha256'])
            source = (inventory.ROOT / symbols.LIB).read_text()
            overlay = _Overlay(old_root)
            overlay.changes[symbols.LIB] = source.replace('fn pure_control_fragment(',
                'fn pure_control_fragment(\n// original control guard mutation\n', 1).encode()
            with self.assertRaisesRegex(ValueError, 'accepted semantic owner/entry changed: pure_control_fragment'):
                aggregate.source_closure(overlay, old_stage, old_sources)
        finally:
            case.doCleanups()
        for owner, data in actual.items():
            self.assertEqual((inventory.ROOT / owner).read_bytes(), data)
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (symbols.LIB, symbols.LOCAL, symbols.BASE + '/naming.rs',
                      symbols.BASE + '/pure_native/item.rs',
                      'tools/compact-rust-backend/recording_admission_inventory_lexical_symbols_test_support.py'):
            self.assertIn(inventory.ROOT / owner, paths)
        for path in paths:
            before = path.read_bytes()
            with self.subTest(owner=str(path)), mock.patch.object(
                    Path, 'write_text', side_effect=AssertionError('authority write attempted')) as write:
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    inventory.main([*self.args(), '--output', str(path)])
                write.assert_not_called()
            self.assertEqual(path.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
