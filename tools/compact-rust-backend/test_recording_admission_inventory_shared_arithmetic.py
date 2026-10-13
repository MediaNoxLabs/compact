# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Shared arithmetic ownership, exact formatter move and qualified old laws."""
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
import recording_admission_inventory_lexical_symbols as lexical_symbols
from recording_admission_inventory_lexical_symbols_test_support import install_historical_reads as install_symbol_reads
import recording_admission_inventory_shared_aggregate as shared_aggregate
from recording_admission_inventory_shared_aggregate_test_support import install_historical_reads as install_aggregate_reads
import recording_admission_inventory_shared_crypto as shared_crypto
from recording_admission_inventory_shared_crypto_test_support import install_historical_reads as install_crypto_reads
import recording_admission_inventory_shared_arithmetic as arithmetic
import recording_admission_inventory_shared_conversion as conversion
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
from recording_admission_inventory_shared_arithmetic_test_support import (
    install_historical_reads, qualified_read_data,
)
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.shared-arithmetic-622d1c.json')


class SharedArithmeticTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(shared_crypto.latest_checkpoint(
            shared_aggregate.latest_checkpoint(
            lexical_symbols.latest_checkpoint(
            native_control_fold.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE))
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = arithmetic.predecessor_sources(self.root, self.extension)
        install_native_fold_reads(self, inventory.ROOT, inventory.BASELINE)
        install_symbol_reads(self, inventory.ROOT, inventory.BASELINE)
        install_aggregate_reads(self, inventory.ROOT, inventory.BASELINE)
        install_crypto_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'shared_arithmetic_extension': self.extension, **overrides})

    def args(self):
        args = ['--shared-arithmetic-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args

    def mutation(self, owner, before, after, diagnostic):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaisesRegex(ValueError, diagnostic):
            arithmetic.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_actual_D1b_D1a_C_B_preserve_reports_accounting_and_pending_body_scope(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(arithmetic.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for k in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[k], prior[k])
        for k in prior:
            if k.startswith('reviewed_'):
                self.assertEqual(current[k], prior[k])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_shared_arithmetic_extension']
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (73, 49))
        self.assertEqual(stage['slice'], 'D1c')
        self.assertEqual(len(stage['production_closure']['unique_source_variants']), 9)
        self.assertIn('pending622', stage['production_closure']['body_syntax_bridge'])
        self.assertIn('legacy nested-equality parser panic correction', stage['production_closure']['equality_output'])
        for key in ('reviewed_shared_conversion_extension', 'reviewed_shared_leaf_extension',
                    'reviewed_unit_fragment_extension', 'reviewed_shared_coercion_extension'):
            forged = {**prior, key: {}}
            report = arithmetic.verify_extension(self.root, self.baseline, self.extension,
                                                 lambda _: forged, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('actual D1b/D1a/C/B', report['errors'][0])
        for key in ('shared_conversion_extension', 'shared_leaf_extension',
                    'unit_fragment_extension', 'shared_coercion_extension', 'shared_arithmetic_extension'):
            self.assertFalse(self.report(**{key: None})['passed'])

    def test_every_inverse_current_source_history_and_original49_authority_rejects_drift(self):
        paths = {arithmetic.SELECTOR}
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

    def test_forged_partial_inverse_owner_history_scope_and_retirement_cannot_claim_delivery(self):
        for kind in ('inverse', 'owner', 'history', 'scope', 'retirement'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][arithmetic.LIB]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'owner':
                forged['owners'].pop(arithmetic.VALUE)
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            else:
                forged['claim'] = 'all Compact formally covered by complete CheckSink'
            self.assertFalse(self.report(shared_arithmetic_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                arithmetic.checkpoint(self.root, forged)

    def test_current_adapter_unmigrated_and_accepted_owners_have_specific_guard_attribution(self):
        self.mutation(arithmetic.LIB, 'fn pure_arithmetic_expression(',
                      'fn pure_arithmetic_expression(\n// changed adapter\n', 'current arithmetic adapter changed')
        for family in arithmetic.FAMILIES:
            anchor = 'fn pure_' + family + '_expression('
            self.mutation(arithmetic.LIB, anchor, anchor + '\n// changed family\n',
                          'unmigrated semantic family changed: ' + family)
        for name in ('pure_conversion_expression', 'pure_leaf_expression', 'pure_control_fragment'):
            anchor = 'fn ' + name + '('
            self.mutation(arithmetic.LIB, anchor, anchor + '\n// changed accepted owner\n',
                          'accepted semantic owner/entry changed: ' + name)
        self.mutation(arithmetic.TEST_MODULE, 'mod shared_arithmetic;',
                      'mod shared_arithmetic;\n// old module changed', 'existing native test module changed')

    def test_both_child_visits_and_operand_bound_trait_order_precede_fact_output(self):
        self.mutation(arithmetic.ARITHMETIC, 'let right = children.visit(right_source, sink)?;',
                      'let right = children.visit(left_source, sink)?;', 'shared arithmetic boundary changed')
        self.mutation(arithmetic.ARITHMETIC, 'checked_unsigned_arithmetic(kind, &left_max, &right_max, max)?',
                      'checked_unsigned_arithmetic(kind, &right_max, &left_max, max)?', 'shared arithmetic boundary changed')
        self.mutation(arithmetic.ARITHMETIC, 'if right.ty != left.ty',
                      'if left.ty != left.ty', 'shared arithmetic boundary changed')
        self.mutation(arithmetic.ARITHMETIC, 'for actual in [left.ty, right.ty]',
                      'for actual in [right.ty, left.ty]', 'shared arithmetic boundary changed')
        self.mutation(arithmetic.ARITHMETIC, 'pub(crate) enum ArithmeticSource',
                      'fn fallback() { crate::expression_with_calls(); }\npub(crate) enum ArithmeticSource',
                      'neutral arithmetic acquired native checking')

    def test_exact_unsigned_renderer_and_boxed_equality_correction_cannot_be_replaced(self):
        self.mutation(arithmetic.VALUE, 'let strategy = checked_unsigned_arithmetic(kind, left_max, right_max, result_max)?;',
                      'let strategy = checked_unsigned_arithmetic(kind, right_max, left_max, result_max)?;',
                      'shared arithmetic boundary changed')
        self.mutation(arithmetic.VALUE, 'runtime::add_wide_unsigned::<#high,#low,_,_>',
                      'runtime::subtract_wide_unsigned::<#high,#low,_,_>', 'exact moved native rendering match changed')
        self.mutation(arithmetic.FORMATTER, 'syn::BinOp::Eq(syn::token::EqEq::default())',
                      'syn::BinOp::Ne(syn::token::Ne::default())', 'shared arithmetic boundary changed')
        self.mutation(arithmetic.FORMATTER, 'impl ArithmeticSink for NativeLeafSink {',
                      'fn recheck() { unsigned_maximum("0"); }\nimpl ArithmeticSink for NativeLeafSink {',
                      'native consumer acquired semantic decisions')
        self.mutation(arithmetic.SINK, 'pub(crate) struct Typed<V> {',
                      'pub(crate) struct Typed<V> {\n    duplicate_domain: bool,', 'accepted leaf/conversion sink domain changed')

    def test_authentic_historical_arithmetic_mutation_survives_and_actual_drift_fails_closed(self):
        actual = {owner: (inventory.ROOT / owner).read_bytes() for owner in self.extension['owners']}
        for owner, data in qualified_read_data(self.root, self.extension).items():
            self.assertEqual(digest(data.encode()), self.extension['owners'][owner]['previous_sha256'])
        self.root.changes[arithmetic.LIB] = actual[arithmetic.LIB].decode().replace(
            'fn pure_arithmetic_expression(', 'fn pure_arithmetic_expression(\n// current drift\n', 1).encode()
        with self.assertRaisesRegex(ValueError, 'current owner changed'):
            qualified_read_data(self.root, self.extension)
        self.root.changes.clear()
        forged = copy.deepcopy(self.extension)
        forged['owners'][arithmetic.LIB]['predecessor_reconstruction'][0]['current'] += '\n'
        with self.assertRaises(ValueError):
            qualified_read_data(self.root, forged)
        old_root = arithmetic.checkpoint(self.root, self.extension)
        old_conversion = self.previous['shared_conversion_extension']
        old_sources = conversion.predecessor_sources(old_root, old_conversion)
        case = unittest.TestCase()
        try:
            install_historical_reads(case, inventory.ROOT, inventory.BASELINE)
            for owner, row in self.extension['owners'].items():
                self.assertEqual(digest((inventory.ROOT / owner).read_bytes()), row['previous_sha256'])
            source = (inventory.ROOT / arithmetic.LIB).read_text()
            self.assertEqual(function(source, 'pure_arithmetic_expression'),
                             function(self.predecessor[arithmetic.LIB], 'pure_arithmetic_expression'))
            overlay = _Overlay(old_root)
            overlay.changes[arithmetic.LIB] = source.replace('fn pure_arithmetic_expression(',
                'fn pure_arithmetic_expression(\n// original arithmetic guard mutation\n', 1).encode()
            with self.assertRaisesRegex(ValueError, 'unmigrated semantic family changed: arithmetic'):
                conversion.source_closure(overlay, old_conversion, old_sources)
        finally:
            case.doCleanups()
        for owner, data in actual.items():
            self.assertEqual((inventory.ROOT / owner).read_bytes(), data)

    def test_cli_protects_all_current_sources_helpers_and_transitive_historical_authorities(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (arithmetic.LIB, arithmetic.VALUE, arithmetic.ARITHMETIC, arithmetic.FORMATTER,
                      'tools/compact-rust-backend/recording_admission_inventory_shared_arithmetic_test_support.py'):
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
