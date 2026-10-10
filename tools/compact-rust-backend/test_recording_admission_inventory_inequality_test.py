# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""A supported inequality gets an exact historical test adaptation, not new admission."""
import contextlib
import copy
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_inequality_test as inequality
import recording_admission_inventory_routing_test as routing
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_shielded_payout as prior_tests


class InequalityTestExtensionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.ShieldedPayoutTransitionTests.setUp(self)
        self.inequality_extension = json.loads(previous.INEQUALITY_TEST_EXTENSION.read_text())
        self.test_owner = self.root / inequality.OWNER
        self.test_owner.write_bytes((inventory.ROOT / inequality.OWNER).read_bytes())

    def report(self):
        return inventory.verify(
            self.root, self.baseline, transition=self.transition,
            baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
            jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
            terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition,
            context_query_transition=self.context_transition, kernel_transition=self.kernel_transition,
            composite_transition=self.composite_transition, receive_transition=self.receive_transition,
            unit_domain_extension=self.unit_domain_extension, voting_transition=self.voting_transition,
            phase_reset_transition=self.phase_transition, immediate_send_transition=self.immediate_transition,
            zswap_transition=self.zswap_transition, membership_transition=self.membership_transition,
            send_transition=self.send_transition, observation_test_extension=self.extension,
            merge_transition=self.merge_transition, guarded_deposit_transition=self.guarded_transition,
            routing_test_extension=self.routing_extension,
            reset_payout_transition=self.reset_transition,
            shielded_payout_transition=self.payout_transition,
            inequality_test_extension=self.inequality_extension)

    def test_current_authority_keeps_counts_and_exact_routing568_predecessor(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 2)
        self.assertEqual(len(report['retired_routes']), 18)
        self.assertEqual(report['reviewed_inequality_test_extension']['new_retired_routes'], [])
        old = inequality.predecessor_source(self.test_owner.read_text(), self.inequality_extension)
        self.assertEqual(inventory.digest(old.encode()), routing.CURRENT_SHA)
        earlier = routing.predecessor_source(old, self.routing_extension)
        self.assertEqual(inventory.digest(earlier.encode()), routing.PRIOR_SHA)

    def test_missing_extension_and_every_ancestor_fail_closed(self):
        for field in ['inequality_extension', 'payout_transition', 'reset_transition', 'routing_extension',
                      'guarded_transition', 'merge_transition', 'transition', 'instrumentation',
                      'jubjub_transition', 'call_extension', 'terminal_transition', 'unit_transition',
                      'context_transition', 'kernel_transition', 'composite_transition', 'receive_transition',
                      'unit_domain_extension', 'voting_transition', 'phase_transition', 'immediate_transition',
                      'zswap_transition', 'membership_transition', 'send_transition', 'extension']:
            old = getattr(self, field); setattr(self, field, None)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_forged_immediate_predecessors_fail(self):
        for field in ['payout_transition', 'routing_extension']:
            old = getattr(self, field); changed = copy.deepcopy(old); changed['claim'] = 'unreviewed'
            setattr(self, field, changed)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_current_test_import_drift_and_partial_rollback_fail(self):
        original = self.test_owner.read_text()
        variants = [original + '\n// drift\n', original.replace('use super::*;', 'use super::{*};', 1)]
        for edit in self.inequality_extension['test_adaptations']:
            variants.append(original.replace(edit['current'], edit['previous'], 1))
        for value in variants:
            self.assertNotEqual(value, original)
            self.test_owner.write_text(value)
            self.assertFalse(self.report()['passed'])
        self.test_owner.write_text(original)

    def test_rehashed_new_manifest_does_not_authorize_drift(self):
        self.test_owner.write_text(self.test_owner.read_text() + '\n// changed\n')
        self.inequality_extension['reviewed_source_sha256'] = inventory.digest(self.test_owner.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            inequality.predecessor_source(self.test_owner.read_text(), self.inequality_extension)

    def test_changed_nonunique_or_missing_reconstruction_fails(self):
        original = copy.deepcopy(self.inequality_extension)
        for value in [[], [{'current': 'NotEqual', 'previous': 'Keccak256'}],
                      [dict(original['test_adaptations'][0], previous='forged')]]:
            self.inequality_extension['test_adaptations'] = value
            self.assertFalse(self.report()['passed'])
        self.inequality_extension = original

    def test_failed_full_chain_cannot_authorize_final_extension(self):
        self.extension['claim'] += ' forged observation history'
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertNotIn('reviewed_inequality_test_extension', report)

    def test_exact_one_owner_overlay_cannot_hide_production_or_shared_drift(self):
        for owner in [inventory.SELECTOR, next(iter(self.payout_transition['shared_sources_sha256']))]:
            path = self.root / owner; original = path.read_text()
            path.write_text(original + '\n// production changed\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_earlier_validator_rejects_current_or_arbitrary_overlay(self):
        arguments = [self.root, self.routing_extension, self.zswap_transition,
                     self.membership_transition, self.send_transition, self.guarded_transition]
        with self.assertRaises(ValueError): routing.validate(*arguments)
        with self.assertRaises(ValueError): routing.validate(*arguments, 'unrelated source')
        old = inequality.predecessor_source(self.test_owner.read_text(), self.inequality_extension)
        restored = routing.validate(*arguments, old)
        self.assertEqual(inventory.digest(restored.encode()), routing.PRIOR_SHA)

    def test_cli_cannot_overwrite_new_manifest(self):
        original = previous.INEQUALITY_TEST_EXTENSION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--inequality-test-extension', str(previous.INEQUALITY_TEST_EXTENSION),
                            '--output', str(previous.INEQUALITY_TEST_EXTENSION)])
        self.assertEqual(previous.INEQUALITY_TEST_EXTENSION.read_bytes(), original)


if __name__ == '__main__': unittest.main()
