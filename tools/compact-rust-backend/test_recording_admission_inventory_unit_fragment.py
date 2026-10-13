# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Single checked Unit views, immutable families/oracles and output protection."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import recording_admission_inventory as inventory
import recording_admission_inventory_shared_leaf as shared_leaf
import recording_admission_inventory_unit_fragment as unit
from test_recording_admission_inventory_generic_qualification import _Overlay
from recording_admission_inventory_shared_leaf_test_support import historical_overlay
MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.unit-fragments-622c.json')


class UnitFragmentTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        latest = inventory.BASELINE.with_name('recording_admission_inventory.shared-leaf-622d1a.json')
        current_root = shared_leaf.latest_checkpoint(inventory.ROOT, inventory.BASELINE)
        self.root = (historical_overlay(current_root, json.loads(latest.read_text()), _Overlay)
                     if latest.exists() else _Overlay(current_root))
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = unit.predecessor_sources(self.root, self.extension)
    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'unit_fragment_extension': self.extension, **overrides})
    def args(self):
        args = ['--unit-fragment-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args
    def mutation(self, owner, before, after):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaises(ValueError):
            unit.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_exact4eaa_predecessor_preserves_reports_and_limits_the_new_scope(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(unit.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for k in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[k], prior[k])
        for k in prior:
            if k.startswith('reviewed_'):
                self.assertEqual(current[k], prior[k])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_unit_fragment_extension']
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (70, 49))
        self.assertEqual(stage['slice'], 'C1+C2')
        self.assertIn('pending622', stage['production_closure']['body_syntax_bridge'])
        self.assertIn('no identical visit-count', stage['production_closure']['static_work_correction'])
        self.assertEqual(len(stage['production_closure']['preserved_semantic_functions']), 7)
        self.assertEqual(len(stage['production_closure']['old_oracle_functions']), 9)

    def test_every_owner_new_source_historical_manifest_and_original_oracle_rejects_drift(self):
        paths = {unit.SELECTOR}
        for key in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            paths.update(self.extension[key])
        authority = self.extension['original_oracle_preservation']['authority']
        original = json.loads((inventory.ROOT / authority).read_text())['files']
        self.assertEqual(len(original), 49)
        paths.update(x['path'] for x in original)
        for owner in paths:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (inventory.ROOT / owner).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_missing_forged_or_non627_parent_cannot_qualify_a_full_body_fold(self):
        for overrides in ({'unit_fragment_extension': None}, {'shared_coercion_extension': None}):
            self.assertFalse(self.report(**overrides)['passed'])
        for kind in ('inverse', 'retirement', 'history', 'scope'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][unit.LIB]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            else:
                forged['claim'] = 'syntax-free complete body compiler and epic done'
            self.assertFalse(self.report(unit_fragment_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                unit.checkpoint(self.root, forged)
        prior = inventory.verify(unit.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        forged = {**prior, 'reviewed_shared_coercion_extension': {}}
        report = unit.verify_extension(self.root, self.baseline, self.extension,
                                        lambda _: forged, inventory.selector_signature)
        self.assertFalse(report['passed'])
        self.assertIn('actual627', report['errors'][0])

    def test_single_fragments_and_all_source_occurrences_precede_output_projection(self):
        self.mutation(unit.LIB, 'let (body, unit_body) = body.into_views();',
                      'let unit_body = unit_statements(&circuit.body, &parameters, &callable_circuits)?;')
        self.mutation(unit.BRIDGE, 'crate::native_fragment_with_calls(&source.body, &scope, pure)?;',
                      'crate::expression_with_calls(&source.body, &scope, pure)?;')
        self.mutation(unit.LIB, 'native_fragment_with_calls(otherwise, parameters, circuits)?;',
                      'skip_otherwise_occurrence(otherwise)?;')
        self.mutation(unit.LIB, 'statements.extend(fragment.into_views().1);',
                      'statements.extend(unit_statements(step, parameters, circuits)?);')

    def test_dispatch_cannot_add_shape_admission_or_drop_an_original_variant(self):
        owner = unit.BASE + '/pure_native/dispatch.rs'
        self.mutation(owner, '$crate::ir::Expr::Assert { .. }',
                      '_')
        self.mutation(owner, '$crate::ir::Expr::Boolean { .. }',
                      '$crate::ir::Expr::Unit')
        self.mutation(owner, '$value($expr, result)', '$value($expr, result) + $value($expr, result)')
        self.mutation(owner, 'let result = match $expr {', 'let result = match extra_entry($expr) {')
        self.mutation(unit.LIB, 'fn pure_crypto_expression(', 'fn changed_crypto_family(')
        owner = unit.BASE + '/pure_native/tests/old_oracle/control.rs'
        self.mutation(owner, 'fn unit_statements(', 'fn changed_old_unit_oracle(')

    def test_formatters_cannot_reacquire_source_type_checks_or_production_test_oracles(self):
        self.mutation(unit.BASE + '/pure_native/control.rs', 'pub(crate) struct IfFacts {',
                      'fn recheck(expr: &Expr) -> Result<(), RenderError> { todo!() }\npub(crate) struct IfFacts {')
        self.mutation(unit.BASE + '/pure_native/fragment.rs', 'pub(crate) enum NativeFragment {',
                      'fn repeat() { native_fragment_with_calls(); }\npub(crate) enum NativeFragment {')
        self.mutation(unit.BASE + '/pure_native.rs', '#[cfg(test)]\nmod tests;', 'mod tests;')

    def test_cli_protects_all_transitive_native_owners_formatters_and_frozen_oracles(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (unit.LIB, unit.BRIDGE, unit.BASE + '/pure_native/tests/old_oracle/control.rs'):
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
