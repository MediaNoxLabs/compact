# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Complete aggregate ownership, scoped signatures and qualified historical guards."""
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
import recording_admission_inventory_shared_aggregate as aggregate
import recording_admission_inventory_shared_crypto as crypto
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
from recording_admission_inventory_shared_aggregate_test_support import (
    install_historical_reads, qualified_read_data,
)
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.shared-aggregate-622d3.json')


class SharedAggregateTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(lexical_symbols.latest_checkpoint(
            native_control_fold.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE))
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = aggregate.predecessor_sources(self.root, self.extension)
        install_native_fold_reads(self, inventory.ROOT, inventory.BASELINE)
        install_symbol_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'shared_aggregate_extension': self.extension, **overrides})

    def args(self):
        args = ['--shared-aggregate-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args

    def mutation(self, owner, before, after, diagnostic):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaisesRegex(ValueError, diagnostic):
            aggregate.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_actual_prior_reports_complete_eight_family_and_pending_body_accounting(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(aggregate.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[key], prior[key])
        for key in prior:
            if key.startswith('reviewed_'):
                self.assertEqual(current[key], prior[key])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_shared_aggregate_extension']
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (75, 49))
        self.assertEqual(stage['slice'], 'D3')
        closure = stage['production_closure']
        self.assertEqual(len(closure['unique_source_variants']), 8)
        self.assertIn('pending622', closure['body_syntax_bridge'])
        self.assertIn('initial child/plan then source', closure['fold_static_order'])
        self.assertIn('source initializer before initial', closure['fold_runtime_order'])
        self.assertEqual(closure['output_assembly_630']['issue'], 630)
        for key in ('reviewed_shared_crypto_extension', 'reviewed_shared_arithmetic_extension',
                    'reviewed_shared_conversion_extension', 'reviewed_shared_leaf_extension',
                    'reviewed_unit_fragment_extension', 'reviewed_shared_coercion_extension'):
            forged = {**prior, key: {}}
            report = aggregate.verify_extension(self.root, self.baseline, self.extension,
                                                 lambda _: forged, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('actual D2/D1c/D1b/D1a/C/B', report['errors'][0])
        for key in ('shared_crypto_extension', 'shared_arithmetic_extension', 'shared_conversion_extension',
                    'shared_leaf_extension', 'unit_fragment_extension', 'shared_coercion_extension', 'shared_aggregate_extension'):
            self.assertFalse(self.report(**{key: None})['passed'])

    def test_all_current_owners_helpers_history_original49_and_selector_reject_drift(self):
        paths = {aggregate.SELECTOR}
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

    def test_forged_partial_inverse_owners_history_scope_and_retirement_cannot_claim_delivery(self):
        for kind in ('inverse', 'argument_inverse', 'item_inverse', 'file_inverse', 'owner', 'history', 'scope', 'retirement'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][aggregate.LIB]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind.endswith('_inverse'):
                index = {'argument_inverse': 1, 'item_inverse': 2, 'file_inverse': 3}[kind]
                forged['owners'][aggregate.LIB]['predecessor_reconstruction'].pop(index)
            elif kind == 'owner':
                forged['owners'].pop(aggregate.ENVIRONMENT)
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            else:
                forged['claim'] = 'all Compact formally covered by complete CheckSink'
            self.assertFalse(self.report(shared_aggregate_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                aggregate.checkpoint(self.root, forged)

    def test_actual_adapter_prior_families_and_original_child_environment_protocol_are_guarded(self):
        self.mutation(aggregate.LIB, 'fn pure_aggregate_expression(',
                      'fn pure_aggregate_expression(\n// changed adapter\n', 'current aggregate adapter changed')
        for name in ('pure_crypto_expression', 'pure_arithmetic_expression', 'pure_conversion_expression',
                     'pure_leaf_expression', 'pure_control_fragment', 'native_fragment_with_calls'):
            anchor = 'fn ' + name + '('
            self.mutation(aggregate.LIB, anchor, anchor + '\n// changed accepted owner\n',
                          'accepted semantic owner/entry changed: ' + name)
        self.mutation(aggregate.LIB, 'let arg = pure_native::item::argument(arg_name, arg_ty);',
                      'let arg = pure_native::item::argument(arg_name, unchecked);', 'issue630 argument assembly boundary changed')
        self.mutation(aggregate.LIB, 'let item = pure_native::item::pure(',
                      'let item = pure_native::item::pure(\n// changed item boundary\n', 'issue630 pure_item assembly boundary changed')
        self.mutation(aggregate.LIB, 'let mut file = pure_native::item::file(',
                      'let mut file = pure_native::item::file(\n// changed file boundary\n', 'issue630 final_file assembly boundary changed')
        self.mutation(aggregate.CHILDREN, 'pub(crate) trait Children<S: LeafSink>',
                      'pub(crate) trait Children<S: LeafSink /* drift */>', 'existing generic protocol/environment changed')
        self.mutation(aggregate.ENVIRONMENT, '    fn visit(\n',
                      '    fn visit(\n// changed old visit\n', 'existing generic protocol/environment changed')
        self.mutation(aggregate.SINK, 'pub(crate) struct Typed<V> {',
                      'pub(crate) struct Typed<V> {\n    unchecked: bool,', 'existing generic protocol/environment changed')
        self.mutation(aggregate.TEST_MODULE, 'mod shared_aggregate;',
                      'mod shared_aggregate;\n// prior test owner changed', 'existing native test module changed')

    def test_borrowed_raw_signatures_call_early_mismatch_and_fold_static_order_are_guarded(self):
        self.mutation(aggregate.DECLARATIONS, 'self.get(raw_source_name)',
                      'self.get("renamed")', 'shared aggregate boundary changed')
        self.mutation(aggregate.DECLARATIONS, 'pub(crate) struct PureSignature',
                      'fn eager_body() { circuit.body; }\npub(crate) struct PureSignature',
                      'borrowed raw signature provider acquired checking/clones')
        self.mutation(aggregate.CALL, 'if arguments.len() != circuit.parameters.len()',
                      'if arguments.len() == circuit.parameters.len()', 'shared aggregate boundary changed')
        self.mutation(aggregate.CALL, 'if value.ty != parameter.ty',
                      'if value.ty == parameter.ty', 'shared aggregate boundary changed')
        self.mutation(aggregate.VECTOR, 'let initial = children.visit(initial, sink)?;',
                      'let initial = children.visit(source, sink)?;', 'shared aggregate boundary changed')
        self.mutation(aggregate.VECTOR, 'circuit.result != accumulator',
                      'circuit.result == accumulator', 'shared aggregate boundary changed')
        self.mutation(aggregate.AGGREGATE, 'pub(crate) enum AggregateSource',
                      'fn fallback() { crate::expression_with_calls(); }\npub(crate) enum AggregateSource',
                      'neutral aggregate acquired native checking')

    def test_scoped_map_preserves_raw_binding_and_native_fold_runtime_order(self):
        self.mutation(aggregate.CHILDREN, '        source_key: &str,',
                      '        source_key: String,', 'existing generic protocol/environment changed')
        self.mutation(aggregate.ENVIRONMENT, 'scope.insert(source_key, (ty, binding.clone()));',
                      'scope.insert("renamed", (ty, binding.clone()));', 'existing generic protocol/environment changed')
        self.mutation(aggregate.VECTOR, 'let name = check_name(&parameter.name)?;',
                      'let name = check_name("renamed")?;', 'shared aggregate boundary changed')
        self.mutation(aggregate.VECTOR, 'children.visit_bound(body, &parameter.name, &parameter.ty, &binding, sink)?',
                      'children.visit_bound(source, &parameter.name, &parameter.ty, &binding, sink)?', 'shared aggregate boundary changed')
        self.mutation(aggregate.NATIVE_VECTOR, 'let #source_name = #source;\n        let mut #acc = #initial;',
                      'let mut #acc = #initial;\n        let #source_name = #source;', 'first-error ordering changed')
        self.mutation(aggregate.NATIVE_VECTOR, '*push.args.first_mut().expect("Map body argument slot") = body;',
                      '*push.args.first_mut().expect("Map body argument slot") = source;', 'issue630 current Map formatter changed')
        self.mutation(aggregate.NATIVE_VECTOR, '.expr = source;',
                      '.expr = body;', 'issue630 current Map formatter changed')

    def test_retention_literal_projection_and_checked_output_closure_preserve_legacy_policy(self):
        self.mutation(aggregate.VALUE, 'let ownership = if copy_type(ty) {',
                      'let ownership = if !copy_type(ty) {', 'shared aggregate boundary changed')
        self.mutation(aggregate.VALUE, '    match ownership {',
                      '    copy_type(unchecked);\n    match ownership {', 'retention formatter acquired semantic decisions')
        self.mutation(aggregate.PROJECTION, '.filter(|declaration| declaration.name == field)',
                      '.filter(|declaration| declaration.name != field)', 'shared aggregate boundary changed')
        self.mutation(aggregate.LITERAL, 'if values.is_empty()',
                      'if !values.is_empty()', 'shared aggregate boundary changed')
        self.mutation(aggregate.NATIVE_LITERAL, 'if tuple.elems.len() > 1',
                      'if tuple.elems.len() > 0', 'shared aggregate boundary changed')
        self.mutation(aggregate.FORMATTER, 'impl AggregateSink for NativeLeafSink {',
                      'fn recheck() { plan_coercion(); }\nimpl AggregateSink for NativeLeafSink {',
                      'native consumer acquired semantic decisions')
        self.mutation(aggregate.ITEM, 'pub(crate) fn argument(',
                      'fn recheck() { rust_type(); }\npub(crate) fn argument(',
                      'issue630 item/file formatter acquired semantic decisions')
        self.mutation(aggregate.ITEM, '*call.args.first_mut().expect("Ok value template slot") = other;',
                      '*call.args.first_mut().expect("Ok value template slot") = expression;',
                      'shared aggregate boundary changed')
        old_helper = aggregate.BASE + '/pure_native/tests/shared_aggregate/old_retained.rs'
        self.mutation(old_helper, 'if copy_type(ty) {',
                      'if !copy_type(ty) {', 'literal original retention helper changed')

    def test_authenticated_old_aggregate_mutation_and_transitive_output_protection_fail_closed(self):
        actual = {owner: (inventory.ROOT / owner).read_bytes() for owner in self.extension['owners']}
        for owner, data in qualified_read_data(self.root, self.extension).items():
            self.assertEqual(digest(data.encode()), self.extension['owners'][owner]['previous_sha256'])
        self.root.changes[aggregate.LIB] = actual[aggregate.LIB].decode().replace(
            'fn pure_aggregate_expression(', 'fn pure_aggregate_expression(\n// actual drift\n', 1).encode()
        with self.assertRaisesRegex(ValueError, 'current owner changed'):
            qualified_read_data(self.root, self.extension)
        self.root.changes.clear()
        forged = copy.deepcopy(self.extension)
        forged['owners'][aggregate.LIB]['predecessor_reconstruction'][0]['current'] += '\n'
        with self.assertRaises(ValueError):
            qualified_read_data(self.root, forged)
        old_root = aggregate.checkpoint(self.root, self.extension)
        old_stage = self.previous['shared_crypto_extension']
        old_sources = crypto.predecessor_sources(old_root, old_stage)
        case = unittest.TestCase()
        try:
            install_historical_reads(case, inventory.ROOT, inventory.BASELINE)
            for owner, row in self.extension['owners'].items():
                self.assertEqual(digest((inventory.ROOT / owner).read_bytes()), row['previous_sha256'])
            source = (inventory.ROOT / aggregate.LIB).read_text()
            overlay = _Overlay(old_root)
            overlay.changes[aggregate.LIB] = source.replace('fn pure_aggregate_expression(',
                'fn pure_aggregate_expression(\n// original aggregate guard mutation\n', 1).encode()
            with self.assertRaisesRegex(ValueError, 'unmigrated semantic family changed: aggregate'):
                crypto.source_closure(overlay, old_stage, old_sources)
        finally:
            case.doCleanups()
        for owner, data in actual.items():
            self.assertEqual((inventory.ROOT / owner).read_bytes(), data)
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (aggregate.LIB, aggregate.VALUE, aggregate.ITEM, aggregate.DECLARATIONS, aggregate.CHILDREN,
                      aggregate.ENVIRONMENT, aggregate.AGGREGATE, aggregate.NATIVE_VECTOR,
                      aggregate.BASE + '/pure_native/tests/shared_aggregate/workers.rs',
                      aggregate.BASE + '/pure_native/tests/shared_aggregate/runtime_probe.rs',
                      'tools/compact-rust-backend/recording_admission_inventory_shared_aggregate_test_support.py'):
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
