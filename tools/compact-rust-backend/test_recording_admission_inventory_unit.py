#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Mutation controls for the fourth explicit production-family retirement."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_unit as unit
import recording_admission_inventory_context as context
import recording_admission_inventory_kernel as kernel
import test_recording_admission_inventory as previous


class UnitActionsTransitionTests(unittest.TestCase):
    setUp = previous.RetirementTransitionTests.setUp
    report = previous.RetirementTransitionTests.report

    def test_explicit_transition_preserves_original_denominator_and_other_retirements(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], kernel.RETIRED)
        self.assertTrue(all(row['passed'] for row in report['retired_routes']))
        self.assertEqual(len(report['current_selector']['ordered_calls']), 14)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertIn(unit.REPLACEMENT, ids)
        self.assertNotIn(unit.ROUTE, ids)
        without = inventory.verify(self.root, self.baseline, transition=self.transition,
                                   baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
                                   jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
                                   terminal_transition=self.terminal_transition)
        self.assertFalse(without['passed'])
        self.assertEqual(len(without['retired_routes']), 3)

    def test_original_source_prior_hash_and_baseline_bindings_are_required(self):
        for key in ('baseline_file_sha256', 'original_source_sha256',
                    'prior_terminal_transition_canonical_sha256'):
            with self.subTest(key=key):
                before = self.unit_transition[key]
                self.unit_transition[key] = '0' * 64
                self.assertFalse(self.report()['passed'])
                self.unit_transition[key] = before
        self.unit_transition['prior_source_fingerprints'][inventory.SELECTOR] = '0' * 64
        self.assertFalse(self.report()['passed'])

    def test_preceding_manifest_cannot_be_rewritten(self):
        self.terminal_transition['claim'] += ' changed'
        self.assertIn('unit retirement: frozen terminal transition changed or unavailable', self.report()['errors'])

    def test_old_admission_call_reintroduction_is_detected_with_whitespace(self):
        path = self.root / inventory.SELECTOR
        path.write_text(path.read_text() + '\nfn accidental() { typed_plan :: lower_unit_actions (); }\n')
        self.assertIn('unit retirement: selector reintroduced old admission', self.report()['errors'])

    def test_oracle_module_and_wrapper_are_uniquely_test_only(self):
        path = self.root / inventory.TYPED_PLAN
        original = path.read_text()
        for declaration in ('mod unit_actions_reference;', "pub(super) fn lower_unit_actions<'a>("):
            with self.subTest(declaration=declaration):
                changed = original.replace('#[cfg(test)]\n' + declaration, declaration)
                self.assertNotEqual(original, changed)
                path.write_text(changed)
                self.assertIn('unit retirement: oracle module and wrapper must each be uniquely cfg(test)', self.report()['errors'])
        path.write_text(original)

    def test_moved_oracle_and_retained_predicates_remain_exact(self):
        for relative, find, replace, error in (
            (unit.ORACLE, 'audit.helpers == 0', 'audit.helpers == 99', 'oracle no longer equals original admission body'),
            (unit.ORIGINAL, 'c.parameters.len() == 1', 'c.parameters.len() == 2', 'shared predicates differ from original'),
        ):
            with self.subTest(relative=relative):
                path = self.root / relative
                original = path.read_text()
                self.assertIn(find, original)
                path.write_text(original.replace(find, replace, 1))
                self.assertIn('unit retirement: ' + error, self.report()['errors'])
                path.write_text(original)
        self.unit_transition['oracle_body_sha256'] = '0' * 64
        self.assertIn('unit retirement: oracle must preserve original admission body exactly', self.report()['errors'])

    def test_shared_policy_users_are_not_claimed_retired(self):
        for relative, anchors in unit.SHARED_ANCHORS.items():
            for anchor in anchors:
                with self.subTest(anchor=anchor):
                    path = self.root / relative
                    original = path.read_text()
                    self.assertIn(anchor, original)
                    path.write_text(original.replace(anchor, '// removed shared policy', 1))
                    self.assertIn('unit retirement: retained shared policy or caller changed: ' + anchor, self.report()['errors'])
                    path.write_text(original)
        self.unit_transition['retained_shared_production_symbols'] = []
        self.assertIn('unit retirement: shared production ownership claim changed', self.report()['errors'])

    def test_shape_admission_cannot_return_to_production_predicate_owner(self):
        path = self.root / unit.ORIGINAL
        path.write_text(path.read_text() + '\nstruct Audit;\n')
        self.assertIn('unit retirement: shape admission remains in production predicate owner', self.report()['errors'])

    def test_unrelated_selector_reordering_cannot_be_hidden_by_refreshed_fingerprints(self):
        path = self.root / inventory.SELECTOR
        original = path.read_text()
        changed = original.replace('typed_plan::lower_composite(', '__swap__(')
        changed = changed.replace('typed_plan::lower_shielded_receive(', 'typed_plan::lower_composite(')
        changed = changed.replace('__swap__(', 'typed_plan::lower_shielded_receive(')
        self.assertNotEqual(original, changed)
        path.write_text(changed)
        self.unit_transition['reviewed_source_fingerprints'][inventory.SELECTOR] = inventory.digest(path.read_bytes())
        self.unit_transition['reviewed_selector'] = inventory.selector_signature(changed)
        self.assertIn('unit retirement: unrelated selector calls/order changed', self.report()['errors'])

    def test_unreviewed_source_drift_owner_or_hook_is_not_authorized(self):
        original = copy.deepcopy(self.unit_transition)
        self.unit_transition['reviewed_source_fingerprints'][inventory.RECORDED] = '0' * 64
        self.assertFalse(self.report()['passed'])
        self.unit_transition = original
        self.unit_transition['instrumented_route_ids'].append('algebra.unreviewed')
        self.assertIn('unit retirement: unrelated observation IDs changed', self.report()['errors'])
        self.unit_transition = json.loads(previous.UNIT_TRANSITION.read_text())
        for relative in (inventory.SELECTOR, *inventory.HOOK_SOURCES[-2:]):
            path = self.root / relative
            before = path.read_bytes()
            path.write_bytes(before + b'\n// unreviewed drift\n')
            self.assertTrue(any('observation owner changed' in error for error in self.report()['errors']))
            path.write_bytes(before)

    def test_retirement_set_cannot_forget_or_expand_families(self):
        original = copy.deepcopy(self.unit_transition)
        for ids in (unit.RETIRED[:-1], unit.RETIRED[1:], unit.RETIRED + ['selector.kernel']):
            self.unit_transition = copy.deepcopy(original)
            self.unit_transition['retired_route_ids'] = ids
            self.assertIn('unit retirement: unreviewed retirement set', self.report()['errors'])

    def test_cli_cannot_overwrite_unit_transition(self):
        manifest = self.root / 'unit.json'
        manifest.write_text(json.dumps(self.unit_transition))
        before = manifest.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--unit-actions-transition', str(manifest), '--output', str(manifest)])
        self.assertEqual(before, manifest.read_bytes())


if __name__ == '__main__':
    unittest.main()
