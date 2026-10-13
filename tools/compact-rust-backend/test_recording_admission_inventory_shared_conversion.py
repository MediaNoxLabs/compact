# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Complete shared conversion ownership and immutable predecessor law targeting."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import recording_admission_inventory as inventory
import recording_admission_inventory_shared_arithmetic as shared_arithmetic
from recording_admission_inventory_shared_arithmetic_test_support import install_historical_reads as install_arithmetic_reads
import recording_admission_inventory_shared_conversion as conversion
import recording_admission_inventory_shared_leaf as leaf
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
from recording_admission_inventory_shared_conversion_test_support import (
    install_historical_reads, qualified_read_data,
)
from recording_admission_inventory_shared_leaf_test_support import project_candidate
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.shared-conversion-622d1b.json')


class SharedConversionTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(shared_arithmetic.latest_checkpoint(inventory.ROOT, inventory.BASELINE))
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = conversion.predecessor_sources(self.root, self.extension)
        install_arithmetic_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'shared_conversion_extension': self.extension, **overrides})

    def args(self):
        args = ['--shared-conversion-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args

    def mutation(self, owner, before, after, diagnostic):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaisesRegex(ValueError, diagnostic):
            conversion.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_actual_D1a_C_B_preserve_all_reports_counts_and_pending_body_scope(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(conversion.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for k in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[k], prior[k])
        for k in prior:
            if k.startswith('reviewed_'):
                self.assertEqual(current[k], prior[k])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_shared_conversion_extension']
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (72, 49))
        self.assertEqual(stage['slice'], 'D1b')
        self.assertIn('pending622', stage['production_closure']['body_syntax_bridge'])
        self.assertEqual(stage['production_closure']['conversion_source_operations'], 4)
        for key in ('reviewed_shared_leaf_extension', 'reviewed_unit_fragment_extension',
                    'reviewed_shared_coercion_extension'):
            forged = {**prior, key: {}}
            report = conversion.verify_extension(self.root, self.baseline, self.extension,
                                                 lambda _: forged, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('actual D1a/C/B', report['errors'][0])
        for key in ('shared_leaf_extension', 'unit_fragment_extension',
                    'shared_coercion_extension', 'shared_conversion_extension'):
            self.assertFalse(self.report(**{key: None})['passed'])

    def test_complete_inverse_owners_new_sources_history_and_original49_reject_drift(self):
        paths = {conversion.SELECTOR}
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

    def test_forged_partial_inverse_history_owner_or_retirement_cannot_claim_delivery(self):
        for kind in ('inverse', 'owner', 'history', 'retirement', 'scope'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][conversion.LIB]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'owner':
                forged['owners'].pop(conversion.SINK)
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            else:
                forged['claim'] = 'complete syntax-free body compiler and epic done'
            self.assertFalse(self.report(shared_conversion_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                conversion.checkpoint(self.root, forged)

    def test_current_conversion_and_accepted_families_have_specific_guard_attribution(self):
        self.mutation(conversion.LIB, 'Expr::Coerce { value, ty } => ConversionSource::Coerce { value, ty },',
                      'Expr::Coerce { value, ty } => ConversionSource::FieldCast { value },',
                      'current conversion adapter changed')
        for family in conversion.FAMILIES:
            anchor = 'fn pure_' + family + '_expression('
            self.mutation(conversion.LIB, anchor, anchor + '\n// changed family\n',
                          'unmigrated semantic family changed: ' + family)
        for name in ('pure_leaf_expression', 'pure_control_fragment'):
            anchor = 'fn ' + name + '('
            self.mutation(conversion.LIB, anchor, anchor + '\n// changed accepted owner\n',
                          'accepted leaf/control/entry changed: ' + name)
        self.mutation(conversion.TEST_MODULE, 'mod shared_conversion;',
                      'mod shared_conversion;\n// changed old registration', 'existing native test module changed')

    def test_shared_child_contract_and_native_consumer_cannot_duplicate_semantics(self):
        self.mutation(conversion.CONVERSION, 'pub(crate) enum ConversionSource',
                      'fn hidden() { crate::expression_with_calls(); }\npub(crate) enum ConversionSource',
                      'neutral conversion acquired native checking')
        self.mutation(conversion.FORMATTER, 'impl ConversionSink for NativeLeafSink {',
                      'fn recheck() { unsigned_maximum("0"); }\nimpl ConversionSink for NativeLeafSink {',
                      'native consumer acquired semantic checking')
        self.mutation(conversion.ENVIRONMENT, 'Ok(Typed { output, ty })',
                      'Ok(Typed { output, ty: ty.clone() })', 'native child environment changed')
        self.mutation(conversion.CHILDREN, 'pub(crate) trait Children<S: LeafSink> {',
                      'pub(crate) trait Children<S> {', 'shared conversion boundary changed')
        self.mutation(conversion.SINK, 'pub(crate) struct Typed<V> {',
                      'pub(crate) struct Typed<V> {\n    extra: bool,', 'accepted leaf fact/sink domain changed')

    def test_conversion_first_errors_plan_caps_and_formatter_visibility_are_preserved(self):
        self.mutation(conversion.CONVERSION, 'let target = unsigned_maximum(max)?;',
                      'let target = unsigned_maximum("0")?;', 'shared conversion boundary changed')
        self.mutation(conversion.CONVERSION, 'let source = unsigned_maximum(&source_max)?;',
                      'let source = target;', 'shared conversion boundary changed')
        self.mutation(conversion.CONVERSION, 'let plan = plan_coercion(&child.ty, ty)?;',
                      'let plan = plan_coercion(ty, ty)?;', 'shared conversion boundary changed')
        self.mutation(conversion.VALUE, 'pub(crate) mod coercion;',
                      'pub mod coercion;', 'native formatter module visibility exceeded')
        self.mutation(conversion.COERCION_FORMATTER, 'pub(crate) fn render_unsigned_cast(',
                      'pub(crate) fn render_unsigned_cast(\n// changed interpreter\n',
                      'existing native interpreter body changed')

    def test_authenticated_historical_data_keeps_conversion_mutations_and_current_drift_fails_closed(self):
        actual = {owner: (inventory.ROOT / owner).read_bytes() for owner in self.extension['owners']}
        for owner, data in qualified_read_data(self.root, self.extension).items():
            self.assertEqual(digest(data.encode()), self.extension['owners'][owner]['previous_sha256'])
        changed = actual[conversion.LIB].decode().replace('ConversionSource::Coerce { value, ty },',
                                                       'ConversionSource::FieldCast { value },', 1).encode()
        self.root.changes[conversion.LIB] = changed
        with self.assertRaisesRegex(ValueError, 'current owner changed'):
            qualified_read_data(self.root, self.extension)
        self.root.changes.clear()
        forged = copy.deepcopy(self.extension)
        forged['owners'][conversion.LIB]['predecessor_reconstruction'][0]['current'] += '\n'
        with self.assertRaises(ValueError):
            qualified_read_data(self.root, forged)
        old_root = conversion.checkpoint(self.root, self.extension)
        prior_leaf = self.previous['shared_leaf_extension']
        prior_leaf_sources = leaf.predecessor_sources(old_root, prior_leaf)
        case = unittest.TestCase()
        try:
            install_historical_reads(case, inventory.ROOT, inventory.BASELINE)
            for owner, row in self.extension['owners'].items():
                self.assertEqual(digest((inventory.ROOT / owner).read_bytes()), row['previous_sha256'])
            # These are the original D1a source data, before an intentional old
            # conversion mutation. Never invert a mutated new conversion owner.
            source = (inventory.ROOT / conversion.LIB).read_text()
            self.assertEqual(function(source, 'pure_conversion_expression'),
                             function(self.predecessor[conversion.LIB], 'pure_conversion_expression'))
            overlay = _Overlay(old_root)
            overlay.changes[conversion.LIB] = source.replace('fn pure_conversion_expression(',
                'fn pure_conversion_expression(\n// historical mutation retained\n', 1).encode()
            with self.assertRaisesRegex(ValueError, 'unmigrated semantic family changed: conversion'):
                leaf.source_closure(overlay, prior_leaf, prior_leaf_sources)
            for owner, row in prior_leaf['owners'].items():
                self.assertEqual(digest(project_candidate(owner, (inventory.ROOT / owner).read_bytes(), prior_leaf)),
                                 row['previous_sha256'])
        finally:
            case.doCleanups()
        for owner, data in actual.items():
            self.assertEqual((inventory.ROOT / owner).read_bytes(), data)

    def test_cli_protects_all_transitive_current_sources_and_historical_oracle_authorities(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (conversion.LIB, conversion.CONVERSION, conversion.CHILDREN,
                      conversion.ENVIRONMENT, conversion.FORMATTER,
                      'tools/compact-rust-backend/recording_admission_inventory_shared_conversion_test_support.py'):
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
