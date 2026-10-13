# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Conversion source authority and deliberate per-conversion work correction."""
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
import recording_admission_inventory_native_control_fold as native_control_fold
from recording_admission_inventory_native_control_fold_test_support import install_historical_reads as install_native_fold_reads
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
import recording_admission_inventory_shared_coercion as coercion
from test_recording_admission_inventory_generic_qualification import _Overlay
MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.shared-coercion-627.json')


class SharedCoercionTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.source_root = unit_fragment.latest_checkpoint(
            shared_leaf.latest_checkpoint(
            shared_conversion.latest_checkpoint(
            shared_arithmetic.latest_checkpoint(
            shared_crypto.latest_checkpoint(
            shared_aggregate.latest_checkpoint(
            lexical_symbols.latest_checkpoint(
            native_control_fold.latest_checkpoint(
            body_checker.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE)
        self.root = _Overlay(self.source_root)
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {key: json.loads((inventory.ROOT / path).read_text())
                         for key, path in self.extension['predecessor_options'].items()}
        self.predecessor = coercion.predecessor_sources(self.root, self.extension)
        install_body_checker_reads(self, inventory.ROOT, inventory.BASELINE)
        install_native_fold_reads(self, inventory.ROOT, inventory.BASELINE)
        install_symbol_reads(self, inventory.ROOT, inventory.BASELINE)
        install_aggregate_reads(self, inventory.ROOT, inventory.BASELINE)
        install_crypto_reads(self, inventory.ROOT, inventory.BASELINE)
        install_arithmetic_reads(self, inventory.ROOT, inventory.BASELINE)
        install_historical_reads(self, inventory.ROOT, inventory.BASELINE)
    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'shared_coercion_extension': self.extension, **overrides})
    def args(self):
        args = ['--shared-coercion-extension', str(MANIFEST)]
        for key, path in self.extension['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        return args
    def mutation(self, owner, before, after):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaises(ValueError):
            coercion.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_exact052_predecessor_preserves_all_recording_reports_and_scopes_the_new_cap(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(coercion.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[key], prior[key])
        for key in prior:
            if key.startswith('reviewed_'):
                self.assertEqual(current[key], prior[key])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_shared_coercion_extension']
        self.assertEqual((stage['issue'], stage['parent_issue'], stage['slice']), (627, 622, 'B'))
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (69, 49))
        work = stage['production_closure']['compiler_work']
        self.assertEqual((work['limit'], work['pending_limit']), (131072, 4096))
        self.assertIn('per conversion', work['scope'])
        self.assertIn('pending622', stage['production_closure']['body_syntax_bridge'])

    def test_every_owner_new_source_history_and_original_oracle_rejects_drift(self):
        paths = {coercion.SELECTOR}
        for key in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            paths.update(self.extension[key])
        authority = self.extension['original_oracle_preservation']['authority']
        original = json.loads((inventory.ROOT / authority).read_text())['files']
        self.assertEqual(len(original), 49)
        paths.update(row['path'] for row in original)
        for owner in paths:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (inventory.ROOT / owner).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_missing_forged_or_partial_authority_and_non626_parent_fail(self):
        for overrides in ({'shared_coercion_extension': None}, {'target_edition_identifiers_extension': None}):
            self.assertFalse(self.report(**overrides)['passed'])
        for kind in ('inverse', 'retirement', 'history', 'scope'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][coercion.NATIVE]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            else:
                forged['claim'] = 'all contract memory bounded and full bodyfold done'
            self.assertFalse(self.report(shared_coercion_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                coercion.checkpoint(self.root, forged)
        prior = inventory.verify(coercion.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        forged = {**prior, 'reviewed_target_edition_identifiers_extension': {}}
        report = coercion.verify_extension(self.root, self.baseline, self.extension,
                                            lambda _: forged, inventory.selector_signature)
        self.assertFalse(report['passed'])
        self.assertIn('actual626', report['errors'][0])

    def test_recorded_value_domain_and_unrelated_functions_cannot_be_relaxed(self):
        self.mutation(coercion.DOMAIN, '&& value_type(source)', '&& true')
        self.mutation(coercion.DOMAIN, '&& value_type(target)', '&& true')
        self.mutation(coercion.DOMAIN, 'elements.len() <= 8 && elements.iter().all(derives_supported)',
                      'elements.len() <= 12 && elements.iter().all(derives_supported)')
        self.mutation(coercion.NATIVE, 'pub(crate) fn copy_type(ty: &Type) -> bool {',
                      'pub(crate) fn copy_type(ty: &Type) -> bool {\n    // modified ownership')

    def test_neutral_relation_charges_before_allocations_and_uses_unchanged_default_limits(self):
        for before, after in [('preflight(actual, target)?;', '// skipped roots'),
                              ('work.charge(count)?;', '// uncharged aggregate'),
                              ('self.observed.saturating_add(units)', 'self.observed.wrapping_add(units)'),
                              ('if self.observed > self.limit', 'if false'),
                              ('mod equality;', 'fn syntax() { syn::parse_quote!(()); }\nmod equality;')]:
            self.mutation(coercion.POLICY, before, after)
        eq = coercion.BASE + '/pure_semantics/coercion/equality.rs'
        self.mutation(eq, 'if observed > work.pending_limit', 'if false')
        self.mutation(eq, 'pending.push(next);', '// missing bounded traversal')
        self.mutation(coercion.LIMITS, 'pub(crate) fn coercion_work_limit() -> usize {\n    Limits::DEFAULT.expanded_work\n}',
                      'pub(crate) fn coercion_work_limit() -> usize {\n    usize::MAX\n}')

    def test_native_formatter_accepts_checked_plans_and_never_source_type_pairs(self):
        owner = coercion.BASE + '/value_lowering/coercion.rs'
        self.mutation(owner, 'pub(super) fn render(plan: &CoercionPlan, value: syn::Expr, depth: usize)',
                      'pub(super) fn render(plan: &CoercionPlan, actual: &Type, value: syn::Expr, depth: usize)')
        self.mutation(owner, 'use super::wide_uint_type;',
                      'use super::wide_uint_type;\nfn redecide() { unsigned_maximum("7"); }')

    def test_cli_protects_every_transitive_old_owner_policy_and_new_plan(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (*coercion.OWNERS, coercion.POLICY):
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
