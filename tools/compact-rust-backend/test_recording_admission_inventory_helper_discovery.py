# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Semantic helper source authority, typed boundary and no-write controls."""
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
import recording_admission_inventory_shared_conversion as shared_conversion
from recording_admission_inventory_shared_conversion_test_support import install_historical_reads
import recording_admission_inventory_shared_leaf as shared_leaf
import recording_admission_inventory_unit_fragment as unit_fragment
import recording_admission_inventory_shared_coercion as shared_coercion
import recording_admission_inventory_target_edition as target_edition
import recording_admission_inventory_shared_pure as shared_pure
import recording_admission_inventory_helper_discovery as helpers
import recording_admission_inventory_native_tuple as native_tuple
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.helper-discovery-620.json')


class HelperDiscoveryTests(unittest.TestCase):
    def setUp(self):
        self.transition = json.loads(MANIFEST.read_text())
        latest_root = unit_fragment.latest_checkpoint(
            shared_leaf.latest_checkpoint(
            shared_conversion.latest_checkpoint(
            shared_arithmetic.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE)
        latest_root = shared_coercion.latest_checkpoint(latest_root, inventory.BASELINE)
        latest_root = target_edition.latest_checkpoint(latest_root, inventory.BASELINE)
        latest_root = shared_pure.latest_checkpoint(latest_root, inventory.BASELINE)
        self.source_root = native_tuple.latest_checkpoint(latest_root, inventory.BASELINE)
        self.root = _Overlay(self.source_root)
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.baseline_bytes)
        self.previous = {
            key: json.loads((inventory.ROOT / path).read_text())
            for key, path in self.transition['predecessor_options'].items()
        }
        install_arithmetic_reads(self, inventory.ROOT, inventory.BASELINE)
        install_historical_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        arguments = dict(self.previous, helper_discovery_transition=self.transition)
        arguments.update(overrides)
        return inventory.verify(self.root, self.baseline,
                                baseline_bytes=self.baseline_bytes, **arguments)

    def cli_arguments(self):
        args = ['--helper-discovery-transition', str(MANIFEST)]
        for key, path in self.transition['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        return args

    def test_exact_signed_predecessor_and_actual_shape_retirement(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        prior = inventory.verify(helpers.checkpoint(self.root, self.transition),
                                 self.baseline, baseline_bytes=self.baseline_bytes,
                                 **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        old_ids = {row['id'] for row in prior['retired_routes']}
        new_ids = {row['id'] for row in report['retired_routes']}
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(old_ids), 66)
        self.assertEqual(new_ids - old_ids, set(helpers.ROUTES))
        self.assertNotIn('helper.transitive-admission', new_ids)
        self.assertEqual(report['reviewed_generic_retirement'], prior['reviewed_generic_retirement'])
        self.assertEqual(report['reviewed_semantic_helper_discovery']['historical_manifest_count'], 64)

    def test_each_complete_owner_new_source_and_historical_authority_rejects_drift(self):
        for label in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            for owner in self.transition[label]:
                with self.subTest(label=label, owner=owner):
                    self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n'
                    self.assertFalse(self.report()['passed'])
                    self.root.changes.clear()

    def test_missing_forged_and_previous_authorities_fail(self):
        for overrides in (dict(helper_discovery_transition=None),
                          dict(generic_retirement_transition=None),
                          dict(generic_qualification_extension=None),
                          dict(helper_discovery_transition={**self.transition, 'claim': 'unreviewed'})):
            self.assertFalse(self.report(**overrides)['passed'])

    def test_partial_inverse_false_count_and_forged_helper_reference_fail(self):
        for kind in ('inverse', 'retirement', 'helper', 'historical'):
            forged = copy.deepcopy(self.transition)
            if kind == 'inverse':
                owner = next(iter(forged['owners']))
                forged['owners'][owner]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'retirement':
                forged['retired_route_ids'].append('helper.transitive-admission')
            elif kind == 'helper':
                forged['helper_oracle']['previous_sha256'] = '0' * 64
            else:
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            self.assertFalse(self.report(helper_discovery_transition=forged)['passed'])

    def test_source_fences_reject_shape_gates_unbounded_or_confused_facts(self):
        mutations = [
            (helpers.HELPERS, 'pub(crate) fn plan_recorded_helpers(', 'fn collect_shared_callees('),
            (helpers.HELPERS, 'if !prospective[index] {', 'if circuit.result == Type::Unit {'),
            (helpers.FACTS, 'children(visit, &mut scan)?;', '// skipped operands'),
            (helpers.FACTS, 'check_frames(&scan.metrics, limits)?;', '// unchecked frames'),
            (helpers.FACTS, '(Limits::DEFAULT.declarations, Limits::DEFAULT.nodes)', '(usize::MAX, usize::MAX)'),
            (helpers.FACTS, 'Node::Expr(Expr::Call { name, .. })', 'Node::Expr(Expr::WitnessCall { name, .. })'),
            (helpers.ROOT, '#[cfg(test)]\nmod helpers_reference;', 'mod helpers_reference;'),
            (helpers.GRAPH, 'edges.sort_unstable();', '// unchecked order'),
            (helpers.GRAPH, 'bound("nodes", target_count, targets)?;', '// unchecked graph size'),
            (helpers.GRAPH, 'if !selected[index]', 'if true'),
        ]
        for owner, checks in self.transition['production_checks'].items():
            for anchor, count in checks['required_occurrences'].items():
                if count == 1:
                    mutations.append((owner, anchor, '// removed semantic boundary'))
        for owner, before, after in mutations:
            with self.subTest(owner=owner, anchor=before):
                source = (self.source_root / owner).read_text()
                self.assertEqual(source.count(before), 1)
                self.root.changes[owner] = source.replace(before, after, 1).encode()
                with self.assertRaises(ValueError):
                    helpers.source_closure(self.root, self.transition)
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_every_original_oracle_rejects_unreviewed_drift(self):
        authority = (inventory.ROOT / (helpers.BASE +
                     '/profile_selection/broad_unit_inclusion_tests/legacy-authority.json'))
        original = json.loads(authority.read_text())['files']
        self.assertEqual(len(original), 49)
        for row in original:
            with self.subTest(owner=row['path']):
                self.root.changes[row['path']] = (inventory.ROOT / row['path']).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_unrelated_compiler_drift_is_not_hidden(self):
        owner = helpers.SELECTOR
        self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n// drift\n'
        self.assertFalse(self.report()['passed'])

    def test_cli_protects_all_transitive_authorities_and_sources(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        self.assertIn(inventory.ROOT / self.transition['helper_oracle']['path'], paths)
        for owner in paths:
            before = owner.read_bytes()
            with self.subTest(owner=str(owner)), mock.patch.object(
                    Path, 'write_text', side_effect=AssertionError('authority write attempted')) as write:
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    inventory.main([*self.cli_arguments(), '--output', str(owner)])
                write.assert_not_called()
            self.assertEqual(owner.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
