# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Full native fold ownership, exhaustive recursion and authenticated prior guards."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import recording_admission_inventory as inventory
import recording_admission_inventory_body_checker as body_checker
from recording_admission_inventory_body_checker_test_support import install_historical_reads as install_body_checker_reads
import recording_admission_inventory_native_control_fold as native
import recording_admission_inventory_lexical_symbols as symbols
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
from recording_admission_inventory_native_control_fold_test_support import (
    install_historical_reads, qualified_read_data,
)
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.native-control-fold-622d4b.json')


class NativeControlFoldTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(body_checker.latest_checkpoint(inventory.ROOT, inventory.BASELINE))
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = native.predecessor_sources(self.root, self.extension)
        install_body_checker_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'native_control_fold_extension': self.extension, **overrides})

    def args(self):
        args = ['--native-control-fold-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args

    def mutation(self, owner, before, after, diagnostic):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaisesRegex(ValueError, diagnostic):
            native.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_actual_D4a_and_prior_reports_explicit_source_map_accounting_and_pending_body_scope(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(native.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[key], prior[key])
        for key in prior:
            if key.startswith('reviewed_'):
                self.assertEqual(current[key], prior[key])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_native_control_fold_extension']
        self.assertEqual((stage['issue'], stage['slice']), (622, 'D4b'))
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (77, 49))
        closure = stage['production_closure']
        self.assertEqual(set(closure['explicit_ir_variants']), native.declared_expr_variants((inventory.ROOT / native.IR).read_text()))
        self.assertEqual(len(closure['explicit_ir_variants']), 70)
        self.assertIn('BodyCheckSink pending622', closure['body_syntax_bridge'])
        self.assertIn('no native dispatch/family fallback', closure['production_recursion'])
        for key in ('reviewed_lexical_symbols_extension', 'reviewed_shared_aggregate_extension',
                    'reviewed_shared_crypto_extension', 'reviewed_shared_arithmetic_extension',
                    'reviewed_shared_conversion_extension', 'reviewed_shared_leaf_extension',
                    'reviewed_unit_fragment_extension', 'reviewed_shared_coercion_extension'):
            report = native.verify_extension(self.root, self.baseline, self.extension,
                                                lambda _: {**prior, key: {}}, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('actual D4a/D3/D2/D1c/D1b/D1a/C/B', report['errors'][0])
        for key in ('lexical_symbols_extension', 'shared_aggregate_extension', 'shared_crypto_extension',
                    'shared_arithmetic_extension', 'shared_conversion_extension', 'shared_leaf_extension',
                    'unit_fragment_extension', 'shared_coercion_extension', 'native_control_fold_extension'):
            self.assertFalse(self.report(**{key: None})['passed'])

    def test_every_actual_owner_new_recursion_helper_history_original49_and_selector_refuses_drift(self):
        paths = {native.SELECTOR}
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

    def test_forged_incomplete_removed_control_cfg_header_protocol_history_and_retirement_cannot_claim_delivery(self):
        for kind in ('removed_region', 'cfg_header', 'owner', 'history', 'scope', 'retirement'):
            forged = copy.deepcopy(self.extension)
            if kind in ('removed_region', 'cfg_header'):
                forged['owners'][native.LIB]['predecessor_reconstruction'].pop(0 if kind == 'removed_region' else 1)
            elif kind == 'owner':
                forged['owners'].pop(native.SCOPE)
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'scope':
                forged['claim'] = 'production syntax-free checker completed; all language formally covered'
            else:
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            self.assertFalse(self.report(native_control_fold_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                native.checkpoint(self.root, forged)

    def test_full_wrappers_explicit_classification_and_production_recursion_cannot_use_legacy_fallback(self):
        self.mutation(native.LIB, 'fn native_fragment_with_calls(',
                      'fn native_fragment_with_calls(\n// changed root wrapper\n',
                      'current full-fold wrapper changed: native_fragment_with_calls')
        self.mutation(native.LIB, 'fn expression_with_calls(',
                      'fn expression_with_calls(\n// changed root wrapper\n',
                      'current full-fold wrapper changed: expression_with_calls')
        self.mutation(native.LIB, '#[cfg(test)]\nfn pure_leaf_expression(',
                      'fn pure_leaf_expression(', 'shared native expression boundary changed')
        self.mutation(native.SOURCE, 'match expr {',
                      'match expr {\n        _ => Source::Leaf(LeafSource::Unit),', 'wildcard source fallback returned')
        self.mutation(native.SOURCE, 'Expr::Unit =>',
                      'Expr::Default =>', 'complete explicit IR variant map changed')
        self.mutation(native.VISITOR, 'pub(crate) struct ExpressionVisitor',
                      'fn fallback() { NativeChildren::new(); }\npub(crate) struct ExpressionVisitor',
                      'neutral fold acquired native fallback')
        self.mutation(native.NATIVE, 'pub(crate) mod control;',
                      'pub(crate) mod control;\npub(crate) mod dispatch;', 'historical dispatch module became registered')

    def test_control_first_refusals_both_equal_arms_concrete_scopes_and_post_type_unit_projection_are_guarded(self):
        self.mutation(native.CONTROL, 'expect(step.ty, Type::Unit)?;',
                      'expect(step.ty, Type::Boolean)?;', 'shared native expression boundary changed')
        self.mutation(native.CONTROL, 'let otherwise_value = children.visit(otherwise, sink)?;',
                      'let otherwise_value = children.visit(then, sink)?;', 'shared native expression boundary changed')
        self.mutation(native.CONTROL, 'let name = check_name(&binding.name)?;',
                      'let name = check_name("renamed")?;', 'shared native expression boundary changed')
        self.mutation(native.CONTROL, 'scope.insert(binding.name.as_str(), &binding.ty, local);',
                      'scope.insert("renamed", &binding.ty, local);', 'shared native expression boundary changed')
        self.mutation(native.SCOPE, 'self.bindings.insert(key, (annotation, binding));',
                      'self.bindings.insert(key, (annotation.clone(), binding));', 'shared native expression boundary changed')
        self.mutation(native.VISITOR, 'scope.insert(source_key, ty, binding.clone());',
                      'scope.insert("renamed", ty, binding.clone());', 'shared native expression boundary changed')
        self.mutation(native.EXPRESSION, 'Expr::Unit | Expr::Default { ty: Type::Unit }',
                      'Expr::Unit | Expr::Tuple { elements: _ }', 'shared native expression boundary changed')
        self.mutation(native.EXPRESSION, 'let view = if typed.ty == Type::Unit',
                      'let view = if typed.ty != Type::Unit', 'shared native expression boundary changed')

    def test_native_checked_value_mapping_and_retained_policies_cannot_recheck_or_recurse(self):
        self.mutation(native.NATIVE_VALUE, 'pub(crate) fn crypto(',
                      'fn retype() { fold_expression(); }\npub(crate) fn crypto(',
                      'native fragment/value consumer acquired semantic checking')
        self.mutation(native.NATIVE_SINK, 'super::local::allocate(name, raw_key, occupied)',
                      'super::local::allocate(name, "renamed", occupied)', 'shared native expression boundary changed')
        self.mutation(native.NATIVE_SINK, 'syn::parse_quote!(#expression;)',
                      'syn::parse_quote!(let _ = #expression;)', 'shared native expression boundary changed')
        for owner in ('pure_native/local.rs', 'pure_native/control.rs', 'pure_native/fragment.rs',
                      'pure_native/aggregate/vector.rs', 'pure_native/item.rs', 'pure_semantics/leaf.rs',
                      'pure_semantics/arithmetic.rs', 'recorded/effect_plan/pure_certification/native.rs',
                      'pure_native/dispatch.rs', 'pure_native/children.rs'):
            path = native.BASE + '/' + owner
            self.root.changes[path] = (inventory.ROOT / path).read_bytes() + b'\n'
            with self.assertRaisesRegex(ValueError, 'protected current semantic/output source changed'):
                native.source_closure(self.root, self.extension, self.predecessor)
            self.assertFalse(self.report()['passed'])
            self.root.changes.clear()

    def test_current_visitor_authenticates_before_old_control_data_and_all_outputs_protect_transitive_authority(self):
        actual = {owner: (inventory.ROOT / owner).read_bytes() for owner in self.extension['owners']}
        for owner, data in qualified_read_data(self.root, self.extension).items():
            self.assertEqual(digest(data.encode()), self.extension['owners'][owner]['previous_sha256'])
        self.root.changes[native.VISITOR] = (inventory.ROOT / native.VISITOR).read_bytes() + b'\n'
        with self.assertRaisesRegex(ValueError, 'new_and_shared_sources changed'):
            qualified_read_data(self.root, self.extension)
        self.root.changes.clear()
        self.root.changes[native.LIB] = actual[native.LIB].decode().replace(
            'fn native_fragment_with_calls(', 'fn native_fragment_with_calls(\n// current drift\n', 1).encode()
        with self.assertRaisesRegex(ValueError, 'current owner changed'):
            qualified_read_data(self.root, self.extension)
        self.root.changes.clear()
        old_root = native.checkpoint(self.root, self.extension)
        old_stage = self.previous['lexical_symbols_extension']
        old_sources = symbols.predecessor_sources(old_root, old_stage)
        case = unittest.TestCase()
        try:
            install_historical_reads(case, inventory.ROOT, inventory.BASELINE)
            for owner, row in self.extension['owners'].items():
                self.assertEqual(digest((inventory.ROOT / owner).read_bytes()), row['previous_sha256'])
            source = (inventory.ROOT / native.LIB).read_text()
            overlay = _Overlay(old_root)
            overlay.changes[native.LIB] = source.replace('evaluate_condition: condition_needs_statement(condition),',
                'evaluate_condition: true,', 1).encode()
            with self.assertRaisesRegex(ValueError, 'control owner outside approved Let slots changed'):
                symbols.source_closure(overlay, old_stage, old_sources)
        finally:
            case.doCleanups()
        for owner, data in actual.items():
            self.assertEqual((inventory.ROOT / owner).read_bytes(), data)
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (native.LIB, native.EXPRESSION, native.VISITOR, native.CONTROL, native.NATIVE_VALUE,
                      native.BASE + '/pure_native/item.rs', native.BASE + '/pure_native/dispatch.rs',
                      'tools/compact-rust-backend/recording_admission_inventory_native_control_fold_test_support.py'):
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
