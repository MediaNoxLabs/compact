# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""A corrected negative test preserves immutable historical routing authority."""
import copy
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_routing_test as routing
import recording_admission_inventory_membership as membership
import recording_admission_inventory_send as send
import recording_admission_inventory_zswap as zswap
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_guarded as prior_tests


class RoutingTestExtensionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.GuardedDepositTransitionTests.setUp(self)
        self.routing_extension = json.loads(previous.ROUTING_TEST_EXTENSION.read_text())
        self.test_owner = self.root / routing.OWNER
        self.test_owner.write_bytes((inventory.ROOT / routing.OWNER).read_bytes())

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
            routing_test_extension=self.routing_extension)

    def test_current_authority_keeps_counts_and_reconstructs_three_historical_pins(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 4)
        self.assertEqual(len(report['retired_routes']), 16)
        self.assertEqual(report['reviewed_routing_test_extension']['new_retired_routes'], [])
        old = routing.predecessor_source(self.test_owner.read_text(), self.routing_extension)
        self.assertEqual(inventory.digest(old.encode()), membership.REVIEWED_OWNERS[routing.OWNER])
        self.assertEqual(inventory.digest(old.encode()), send.REVIEWED_OWNERS[routing.OWNER])
        original = previous.membership_checkpoint_sources(inventory.ROOT)[routing.OWNER]
        self.assertEqual(inventory.digest(original.encode()), zswap.TEST_SHA)

    def test_missing_extension_and_each_ancestor_fail_closed(self):
        for field in ['routing_extension', 'guarded_transition', 'merge_transition', 'transition',
                      'instrumentation', 'jubjub_transition', 'call_extension', 'terminal_transition',
                      'unit_transition', 'context_transition', 'kernel_transition', 'composite_transition',
                      'receive_transition', 'unit_domain_extension', 'voting_transition', 'phase_transition',
                      'immediate_transition', 'zswap_transition', 'membership_transition', 'send_transition', 'extension']:
            old = getattr(self, field)
            setattr(self, field, None)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_forged_ancestor_cannot_authorize_current_test(self):
        for field in ['zswap_transition', 'membership_transition', 'send_transition', 'guarded_transition']:
            old = getattr(self, field)
            forged = copy.deepcopy(old); forged['claim'] = 'unreviewed rewritten history'
            setattr(self, field, forged)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_unrelated_current_test_changes_and_partial_rollback_fail(self):
        original = self.test_owner.read_text()
        variants = [original + '\n// unrelated drift\n',
                    original.replace('assert!(selected.typed_result.is_none());', 'assert!(true);', 1)]
        for edit in self.routing_extension['test_adaptations']:
            variants.append(original.replace(edit['current'], edit['previous'], 1))
        for changed in variants:
            self.assertNotEqual(changed, original)
            self.test_owner.write_text(changed)
            self.assertFalse(self.report()['passed'])
        self.test_owner.write_text(original)

    def test_rehashing_manifest_and_current_owner_does_not_authorize(self):
        original = copy.deepcopy(self.routing_extension)
        self.test_owner.write_text(self.test_owner.read_text() + '\n// changed\n')
        self.routing_extension['reviewed_source_sha256'] = inventory.digest(self.test_owner.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            routing.predecessor_source(self.test_owner.read_text(), self.routing_extension)
        self.routing_extension = original

    def test_missing_or_altered_reconstruction_is_rejected(self):
        original = copy.deepcopy(self.routing_extension)
        for value in [[], original['test_adaptations'][:1],
                      [{'current': 'NotEqual', 'previous': 'NativeWitnessCall'}]]:
            self.routing_extension['test_adaptations'] = value
            self.assertFalse(self.report()['passed'])
        self.routing_extension = original

    def test_invalid_full_chain_never_becomes_success_from_test_snapshot(self):
        self.extension['claim'] += ' unreviewed observation history'
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertNotIn('reviewed_routing_test_extension', report)

    def test_snapshot_cannot_hide_unrelated_production_or_shared_mutations(self):
        for owner in [inventory.SELECTOR, membership.TYPED,
                      next(iter(self.guarded_transition['shared_sources_sha256']))]:
            path = self.root / owner; original = path.read_text()
            path.write_text(original + '\n// unrelated production change\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_earlier_checker_still_refuses_unadapted_current_owner(self):
        at_send = previous.merge_checkpoint_sources(inventory.ROOT)
        owners = {owner: (self.root / owner).read_text() for owner in send.REVIEWED_OWNERS}
        owners.update(at_send)
        owners[routing.OWNER] = self.test_owner.read_text()
        with self.assertRaises(ValueError):
            send.predecessor_source_snapshot(owners, self.send_transition)
        owners[routing.OWNER] = routing.predecessor_source(self.test_owner.read_text(), self.routing_extension)
        snapshot = send.predecessor_source_snapshot(owners, self.send_transition)
        self.assertEqual(inventory.digest(snapshot[routing.OWNER].encode()), routing.PRIOR_SHA)

    def test_cli_cannot_overwrite_adaptation_manifest(self):
        original = previous.ROUTING_TEST_EXTENSION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--routing-test-extension', str(previous.ROUTING_TEST_EXTENSION),
                            '--output', str(previous.ROUTING_TEST_EXTENSION)])
        self.assertEqual(previous.ROUTING_TEST_EXTENSION.read_bytes(), original)


if __name__ == '__main__':
    unittest.main()
