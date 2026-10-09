#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Mutation controls for complete receive retirement and live shared dispatch."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_receive as receive
import test_recording_admission_inventory as previous


class ReceiveTransitionTests(unittest.TestCase):
    setUp = previous.RetirementTransitionTests.setUp
    report = previous.RetirementTransitionTests.report

    def test_explicit_transition_keeps_denominator_and_only_retires_receive(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([r['id'] for r in report['retired_routes']], receive.RETIRED)
        self.assertTrue(all(r['passed'] for r in report['retired_routes']))
        self.assertEqual(len(report['current_selector']['ordered_calls']), 12)
        self.assertIn(receive.REPLACEMENT, report['reviewed_instrumentation']['instrumented_route_ids'])
        self.assertNotIn(receive.ROUTE, report['reviewed_instrumentation']['instrumented_route_ids'])
        self.receive_transition = None
        before = self.report()
        self.assertFalse(before['passed'])
        self.assertEqual(len(before['retired_routes']), 7)

    def test_original_predecessor_and_checkpoint_cannot_be_rebound(self):
        for key in ['baseline_file_sha256', 'original_source_sha256',
                    'prior_composite_transition_canonical_sha256', 'reviewed_checkpoint']:
            with self.subTest(key=key):
                original = self.receive_transition[key]
                self.receive_transition[key] = '0' * len(original)
                self.assertFalse(self.report()['passed'])
                self.receive_transition[key] = original
        self.receive_transition['prior_source_fingerprints'][receive.SELECTOR] = '0' * 64
        self.assertIn('receive retirement: prior observation owner is not bound to frozen evidence', self.report()['errors'])

    def test_previous_manifest_remains_immutable(self):
        self.composite_transition['claim'] += ' changed'
        self.assertIn('receive retirement: frozen composite transition changed or unavailable', self.report()['errors'])

    def test_original_bodies_cannot_be_replaced_or_rehashed(self):
        path = self.root / receive.ORACLE
        original = path.read_text()
        for marker in receive.ORIGINAL_FUNCTIONS:
            actual = receive.ORACLE_MARKER if marker == receive.MARKER else marker
            brace = original.index('{\n', original.index(actual))
            path.write_text(original[:brace + 2] + '    // changed oracle\n' + original[brace + 2:])
            self.assertIn('receive retirement: original admission function changed: ' + marker, self.report()['errors'])
        path.write_text(original)
        self.receive_transition['oracle_normalized_functions_sha256'][receive.MARKER] = '0' * 64
        self.assertIn('receive retirement: original root admission function must be preserved', self.report()['errors'])

    def test_module_and_wrapper_cannot_lose_test_guards_or_be_duplicated(self):
        path = self.root / receive.TYPED
        original = path.read_text()
        for declaration in ['mod shielded_receive_reference;', receive.MARKER]:
            changed = original.replace('#[cfg(test)]\n' + declaration, declaration)
            self.assertNotEqual(original, changed)
            path.write_text(changed)
            self.assertIn('receive retirement: oracle module and wrapper must be uniquely cfg(test)', self.report()['errors'])
        path.write_text(original + '\nmod shielded_receive_reference;\n')
        self.assertIn('receive retirement: oracle module and wrapper must be uniquely cfg(test)', self.report()['errors'])

    def test_no_production_fallback_or_alias_in_any_observed_owner(self):
        for owner in [receive.SELECTOR, inventory.RECORDED]:
            path = self.root / owner
            original = path.read_text()
            for addition in ['fn accidental() { typed_plan :: lower_shielded_receive (); }',
                             'use super::typed_plan::lower_shielded_receive as old;',
                             'use super::typed_plan::shielded_receive_reference as old;']:
                path.write_text(original + '\n' + addition)
                self.assertIn('receive retirement: production owner reintroduced old admission import or call: ' + owner, self.report()['errors'])
            path.write_text(original)
        path = self.root / receive.TYPED
        path.write_text(path.read_text() + '\nuse shielded_receive_reference as old;\n')
        self.assertIn('receive retirement: reference authority escaped the test-only wrapper', self.report()['errors'])

    def test_reference_call_must_stay_inside_guarded_wrapper(self):
        path = self.root / receive.TYPED
        text = path.read_text().replace('shielded_receive_reference::lower(circuit, ledger, witnesses, pure, circuits)', 'None')
        path.write_text(text + '\nfn illegal() { shielded_receive_reference::lower(); }\n')
        self.assertIn('receive retirement: reference authority escaped the test-only wrapper', self.report()['errors'])

    def test_shared_helpers_cannot_be_duplicated_or_replaced(self):
        path = self.root / receive.TYPED
        path.write_text(path.read_text() + '\nfn shielded_receive_action() {}\n')
        self.assertIn('receive retirement: retained shared admission helper changed: fn shielded_receive_action(', self.report()['errors'])

    def test_shared_predicate_and_both_live_dispatch_users_are_pinned(self):
        path = self.root / receive.TYPED
        original = path.read_text()
        changed = original.replace(receive.SHARED_MARKER, receive.SHARED_MARKER + '/* changed */')
        path.write_text(changed)
        self.assertIn('receive retirement: retained shared predicate changed', self.report()['errors'])
        for prefix in ['singleton_bridge', 'intents']:
            changed = original.replace('composite_domain.' + prefix + '()', 'composite_domain.unreviewed()')
            path.write_text(changed)
            user = 'singleton_bridge' if prefix == 'singleton_bridge' else 'unit_actions_intents'
            self.assertIn('receive retirement: retained shared predicate user changed: ' + user, self.report()['errors'])
        path.write_text(original)
        self.receive_transition['retained_shared_predicate_sha256'] = '0' * 64
        self.assertIn('receive retirement: shared predicate and live users must remain bound', self.report()['errors'])

    def test_unrelated_selector_order_cannot_be_approved_with_refreshed_hashes(self):
        path = self.root / receive.SELECTOR
        original = path.read_text()
        changed = original.replace('typed_plan::lower_voting_commit(', '__swap__(')
        changed = changed.replace('typed_plan::lower_funded_mint(', 'typed_plan::lower_voting_commit(')
        changed = changed.replace('__swap__(', 'typed_plan::lower_funded_mint(')
        self.assertNotEqual(original, changed)
        path.write_text(changed)
        self.receive_transition['reviewed_source_fingerprints'][receive.SELECTOR] = inventory.digest(path.read_bytes())
        self.receive_transition['reviewed_selector'] = inventory.selector_signature(changed)
        self.assertIn('receive retirement: unrelated selector calls/order changed', self.report()['errors'])

    def test_no_extra_retirement_or_observation_owner_drift(self):
        original = copy.deepcopy(self.receive_transition)
        for key, value in [('retired_route_ids', receive.RETIRED + ['value.composite-domain']),
                           ('new_retired_routes', []), ('instrumented_route_ids', []),
                           ('reviewed_source_fingerprints', {receive.SELECTOR: '0' * 64, receive.TYPED: '0' * 64})]:
            self.receive_transition = copy.deepcopy(original)
            self.receive_transition[key] = value
            self.assertFalse(self.report()['passed'])
        self.receive_transition = original
        path = self.root / receive.SELECTOR
        path.write_bytes(path.read_bytes() + b'\n// unreviewed drift\n')
        self.assertTrue(any('observation owner changed' in e for e in self.report()['errors']))

    def test_shared_callers_and_exact_unit_policy_relocation_are_guarded(self):
        for owner in receive.SHARED_CALLERS:
            path = self.root / owner
            original = path.read_bytes()
            path.write_bytes(original + b'\n// modified live helper caller\n')
            self.assertIn('receive retirement: retained shared caller changed: ' + owner, self.report()['errors'])
            path.write_bytes(original)
        original = copy.deepcopy(self.receive_transition['relocated_shared_anchors'])
        for key, value in [('target', receive.TYPED), ('text', 'unit_actions: false,'),
                           ('source_occurrences', 1), ('target_occurrences', 0)]:
            self.receive_transition['relocated_shared_anchors'] = copy.deepcopy(original)
            self.receive_transition['relocated_shared_anchors'][0][key] = value
            self.assertIn('receive retirement: shared predicates/callers or exact policy relocation changed', self.report()['errors'])
        self.receive_transition['relocated_shared_anchors'] = original
        path = self.root / receive.ORACLE
        path.write_text(path.read_text().replace('unit_actions: true,', 'unit_actions: false,'))
        self.assertIn('unit retirement: retained shared policy or caller changed: unit_actions: true,', self.report()['errors'])

    def test_cli_cannot_overwrite_new_evidence(self):
        path = self.root / 'receive.json'
        path.write_text(json.dumps(self.receive_transition))
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--receive-transition', str(path), '--output', str(path)])
        self.assertEqual(before, path.read_bytes())


if __name__ == '__main__':
    unittest.main()
