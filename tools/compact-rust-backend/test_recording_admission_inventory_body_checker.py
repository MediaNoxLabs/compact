# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Formal unit checker, exact representation guards and safe retired-source authority."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import recording_admission_inventory as inventory
import recording_admission_inventory_body_checker as checker
import recording_admission_inventory_native_control_fold as native
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
from recording_admission_inventory_body_checker_test_support import (
    install_historical_reads, qualified_read_data,
)
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.body-checker-622d4c.json')


class BodyCheckerTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(inventory.ROOT)
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = checker.predecessor_sources(self.root, self.extension)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'body_checker_extension': self.extension, **overrides})

    def args(self):
        args = ['--body-checker-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args

    def mutation(self, owner, before, after, diagnostic):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaisesRegex(ValueError, diagnostic):
            checker.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_actual_D4b_and_prior_reports_preserve_accounting_and_limited_formal_checker_claim(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(checker.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[key], prior[key])
        for key in prior:
            if key.startswith('reviewed_'):
                self.assertEqual(current[key], prior[key])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_body_checker_extension']
        self.assertEqual((stage['issue'], stage['slice']), (622, 'D4c'))
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (78, 49))
        self.assertIn('BodyCheckSink Value=()/Binding=()/Carriers=CheckSink', stage['production_closure']['monomorphized_consumer'])
        self.assertIn('EA13 still pending', stage['production_closure']['resource_policy'])
        for key in ('reviewed_native_control_fold_extension', 'reviewed_lexical_symbols_extension',
                    'reviewed_shared_aggregate_extension', 'reviewed_shared_crypto_extension',
                    'reviewed_shared_arithmetic_extension', 'reviewed_shared_conversion_extension',
                    'reviewed_shared_leaf_extension', 'reviewed_unit_fragment_extension', 'reviewed_shared_coercion_extension'):
            report = checker.verify_extension(self.root, self.baseline, self.extension,
                                                lambda _: {**prior, key: {}}, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('actual D4b/D4a/D3/D2/D1c/D1b/D1a/C/B', report['errors'][0])
        for key in ('native_control_fold_extension', 'lexical_symbols_extension', 'shared_aggregate_extension',
                    'shared_crypto_extension', 'shared_arithmetic_extension', 'shared_conversion_extension',
                    'shared_leaf_extension', 'unit_fragment_extension', 'shared_coercion_extension', 'body_checker_extension'):
            self.assertFalse(self.report(**{key: None})['passed'])

    def test_all_current_moved_sources_history_original49_and_retired_bridge_resurrection_refuse(self):
        paths = {checker.SELECTOR}
        for key in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            paths.update(self.extension[key])
        original = json.loads((inventory.ROOT / self.extension['original_oracle_preservation']['authority']).read_text())['files']
        self.assertEqual(len(original), 49)
        paths.update(row['path'] for row in original)
        self.assertNotIn(checker.RETIRED, paths)
        for owner in paths:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (inventory.ROOT / owner).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()
        for source in (b'', self.extension['retired_sources'][checker.RETIRED]['previous_source'].encode()):
            self.root.changes[checker.RETIRED] = source
            with self.assertRaisesRegex(ValueError, 'retired production native bridge returned'):
                qualified_read_data(self.root, self.extension)
            self.assertFalse(self.report()['passed'])
            self.root.changes.clear()
        with self.assertRaises(FileNotFoundError):
            (inventory.ROOT / checker.RETIRED).read_bytes()

    def test_forged_partial_move_guard_owner_retired_source_and_new_retirement_cannot_claim_delivery(self):
        for kind in ('inverse', 'owner', 'retired_bytes', 'retired_owner', 'guard', 'scope', 'retirement'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][checker.CERTIFIER]['predecessor_reconstruction'].pop()
            elif kind == 'owner':
                forged['owners'].pop(checker.SEMANTICS)
            elif kind == 'retired_bytes':
                forged['retired_sources'][checker.RETIRED]['previous_source'] += '\n'
            elif kind == 'retired_owner':
                forged['retired_sources'].clear()
            elif kind == 'guard':
                forged['production_closure']['moved_representation_guards']['check_defaults'] = '0' * 64
            elif kind == 'scope':
                forged['claim'] = 'allocation-free formally proved complete language checker and goal closure'
            else:
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            self.assertFalse(self.report(body_checker_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                checker.checkpoint(self.root, forged)

    def test_formal_name_carrier_duplicate_result_body_order_and_moved_obligations_have_exact_guards(self):
        self.mutation(checker.CHECKER, 'check_carrier(&parameter.ty)?;',
                      'check_carrier(&source.result)?;', 'formal body checker boundary changed')
        self.mutation(checker.CHECKER, '.insert(parameter.name.as_str(), (&parameter.ty, ()))',
                      '.insert("renamed", (&parameter.ty, ()))', 'formal body checker boundary changed')
        self.mutation(checker.CHECKER, 'fold_expression(&source.body, &scope, pure, &mut BodyCheckSink::new())?',
                      'fold_expression(&source.body, &scope, pure, &mut NativeSink::new())?',
                      'formal body checker boundary changed')
        self.mutation(checker.CHECKER, 'default_tuple_traits_supported(&nodes)',
                      'default_tuple_traits_supported(&[])', 'exact moved representation guard changed: check_defaults')
        self.mutation(checker.CHECKER, 'named_struct_derives_supported(ty)',
                      'named_struct_derives_supported(&Type::Unit)', 'exact moved representation guard changed: check_named_structs')
        self.mutation(checker.CERTIFIER, 'mod checker;',
                      'mod native;', 'formal body checker boundary changed')
        self.mutation(checker.CERTIFIER, 'self.certificates.entries.insert(',
                      'self.certificates.entries.clear();\n            self.certificates.entries.insert(',
                      'Certifier executable/cache/binding authority changed')

    def test_unit_associated_receiver_has_no_output_allocator_checker_or_second_body_matcher(self):
        self.mutation(checker.BODY, 'type Value = ();',
                      'type Value = syn::Expr;', 'formal body checker boundary changed')
        self.mutation(checker.BODY, 'type Binding = ();',
                      'type Binding = syn::Ident;', 'formal body checker boundary changed')
        self.mutation(checker.BODY, 'pub(crate) struct BodyCheckSink',
                      'fn native_fallback() { crate::pure_native::local::allocate(); }\npub(crate) struct BodyCheckSink',
                      'unit output callback acquired checking/native syntax')
        self.mutation(checker.BODY, 'impl ExpressionSink for BodyCheckSink {',
                      'fn recheck() { check_name(); }\nimpl ExpressionSink for BodyCheckSink {',
                      'unit output callback acquired checking/native syntax')
        reference = checker.BASE + '/recorded/effect_plan/pure_certification/checker/tests/old_bridge.rs'
        self.mutation(reference, '.zip(crate::naming::parameter_idents(&source.parameters, &[]))',
                      '.zip(std::iter::empty())', 'independent signed old bridge check body changed')

    def test_specific_neutral_functions_are_allowed_but_shared_fold_and_native_callgraph_authority_cannot_drift(self):
        self.mutation(checker.BASE + '/naming.rs', 'pub(crate) fn semantic_identifier(',
                      'pub(crate) fn semantic_identifier(\n// modified neutral routine\n',
                      'allowed specific neutral function changed: semantic_identifier')
        for owner in ('pure_semantics/expression.rs', 'pure_semantics/visitor.rs', 'pure_semantics/control.rs',
                      'pure_semantics/scope.rs', 'pure_semantics/leaf.rs', 'pure_semantics/carrier.rs',
                      'pure_semantics/coercion.rs', 'pure_native/sink.rs', 'pure_native/local.rs',
                      'native_representation.rs', 'resource_limits/pure.rs'):
            path = checker.BASE + '/' + owner
            self.root.changes[path] = (inventory.ROOT / path).read_bytes() + b'\n'
            with self.assertRaisesRegex(ValueError, 'protected shared/native/transitive source changed'):
                checker.source_closure(self.root, self.extension, self.predecessor)
            self.assertFalse(self.report()['passed'])
            self.root.changes.clear()

    def test_retired_file_staging_requires_actual_checker_first_and_absent_outputs_remain_protected(self):
        actual = {owner: (inventory.ROOT / owner).read_bytes() for owner in self.extension['owners']}
        data = qualified_read_data(self.root, self.extension)
        self.assertEqual(digest(data[checker.RETIRED].encode()), self.extension['retired_sources'][checker.RETIRED]['previous_sha256'])
        self.root.changes[checker.CHECKER] = (inventory.ROOT / checker.CHECKER).read_bytes() + b'\n'
        with self.assertRaisesRegex(ValueError, 'new_and_shared_sources changed'):
            qualified_read_data(self.root, self.extension)
        self.root.changes.clear()
        old_root = checker.checkpoint(self.root, self.extension)
        old_stage = self.previous['native_control_fold_extension']
        old_sources = native.predecessor_sources(old_root, old_stage)
        case = unittest.TestCase()
        try:
            install_historical_reads(case, inventory.ROOT, inventory.BASELINE)
            for owner, row in self.extension['owners'].items():
                self.assertEqual(digest((inventory.ROOT / owner).read_bytes()), row['previous_sha256'])
            self.assertEqual((inventory.ROOT / checker.RETIRED).read_text(), data[checker.RETIRED])
            overlay = _Overlay(old_root)
            overlay.changes[checker.RETIRED] = data[checker.RETIRED].replace('pub(super) fn check(',
                'pub(super) fn check(\n// original bridge guard mutation\n', 1).encode()
            with self.assertRaisesRegex(ValueError, 'protected current semantic/output source changed'):
                native.source_closure(overlay, old_stage, old_sources)
        finally:
            case.doCleanups()
        for owner, raw in actual.items():
            self.assertEqual((inventory.ROOT / owner).read_bytes(), raw)
        with self.assertRaises(FileNotFoundError):
            (inventory.ROOT / checker.RETIRED).read_bytes()
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (checker.RETIRED, checker.CHECKER, checker.BODY, checker.CERTIFIER,
                      'tools/compact-rust-backend/recording_admission_inventory_body_checker_test_support.py'):
            self.assertIn(inventory.ROOT / owner, paths)
        for path in paths:
            try:
                before = path.read_bytes()
            except FileNotFoundError:
                before = None
            with self.subTest(owner=str(path)), mock.patch.object(
                    Path, 'write_text', side_effect=AssertionError('authority write attempted')) as write:
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    inventory.main([*self.args(), '--output', str(path)])
                write.assert_not_called()
            if before is None:
                with self.assertRaises(FileNotFoundError):
                    path.read_bytes()
            else:
                self.assertEqual(path.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
