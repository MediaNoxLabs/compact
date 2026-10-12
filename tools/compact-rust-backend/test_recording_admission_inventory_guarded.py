# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Guarded deposit retirement preserves frozen roots and live shared admission."""
import copy
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_guarded as guarded
import recording_admission_inventory_merge as merge
import recording_admission_inventory_immediate as immediate
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_merge as prior_tests


class GuardedDepositTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.MergeTransitionTests.setUp(self)
        self.guarded_transition = json.loads(previous.GUARDED_TRANSITION.read_text())
        predecessor = previous.reset_payout_checkpoint_sources(inventory.ROOT)
        for owner in [*guarded.REVIEWED_OWNERS, *self.guarded_transition['shared_sources_sha256'],
                      *self.guarded_transition['test_sources_sha256']]:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(predecessor[owner].encode() if owner in predecessor
                             else previous.checkpoint_bytes(owner))
        (self.root / previous.routing_test.OWNER).write_text(previous.routing_checkpoint_source(inventory.ROOT))

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
            merge_transition=self.merge_transition, guarded_deposit_transition=self.guarded_transition)

    def test_exact_four_legacy_sixteen_retired_and_prior_owners(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 4)
        self.assertEqual([r['id'] for r in report['retired_routes']], guarded.RETIRED)
        self.assertEqual(len(guarded.RETIRED), 16)
        snapshot = guarded.predecessor_sources(self.root, self.guarded_transition)
        for owner in [guarded.SELECTOR, guarded.TYPED, guarded.VOTING_TEST]:
            self.assertEqual(inventory.digest(snapshot[owner].encode()), merge.REVIEWED_OWNERS[owner])
        self.assertEqual(report['reviewed_observation_test_extension']['issue'], 564)

    def test_missing_transition_and_every_predecessor_fail_closed(self):
        for field in ['guarded_transition', 'merge_transition', 'transition', 'instrumentation',
                      'jubjub_transition', 'call_extension', 'terminal_transition', 'unit_transition',
                      'context_transition', 'kernel_transition', 'composite_transition', 'receive_transition',
                      'unit_domain_extension', 'voting_transition', 'phase_transition', 'immediate_transition',
                      'zswap_transition', 'membership_transition', 'send_transition', 'extension']:
            old = getattr(self, field)
            setattr(self, field, None)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_all_current_owners_and_root_guards_are_immutable(self):
        for owner in guarded.REVIEWED_OWNERS:
            path = self.root / owner; old = path.read_text()
            path.write_text(old + '\n// owner drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(old)
        for owner, marker in [(guarded.TYPED, "pub(super) fn lower_guarded_deposit<'a>("),
                              (guarded.GUARDED, "pub(super) fn lower<'a>(")]:
            path = self.root / owner; old = path.read_text()
            self.assertIn('#[cfg(test)]\n' + marker, old)
            path.write_text(old.replace('#[cfg(test)]\n' + marker, marker))
            self.assertFalse(self.report()['passed'])
            path.write_text(old)

    def test_shared_module_prefix_flow_audit_and_type_helpers_stay_live(self):
        for owner, marker in [(guarded.TYPED, 'mod guarded_deposit;'),
                              (guarded.GUARDED, 'pub(super) fn prefix('),
                              (guarded.GUARDED, 'pub(super) fn read_type('),
                              (guarded.GUARDED, 'pub(super) fn write_type('),
                              (guarded.GUARDED, 'pub(super) fn forwards_received_coin('),
                              (guarded.GUARDED, 'struct Flow {'), (guarded.GUARDED, "struct Audit<'a> {")]:
            path = self.root / owner; old = path.read_text(); self.assertIn(marker, old)
            path.write_text(old.replace(marker, '#[cfg(test)]\n' + marker, 1))
            self.assertFalse(self.report()['passed'], marker)
            path.write_text(old)

    def test_manifest_rehash_empty_maps_and_reconstruction_cannot_authorize(self):
        forged = copy.deepcopy(self.guarded_transition); forged['claim'] += ' changed'
        with self.assertRaises(ValueError):
            guarded.predecessor_sources(self.root, forged)
        for key in ['prior_source_fingerprints', 'reviewed_source_fingerprints',
                    'predecessor_reconstruction', 'shared_sources_sha256', 'test_sources_sha256']:
            old = self.guarded_transition[key]; self.guarded_transition[key] = {}
            self.assertFalse(self.report()['passed'], key)
            self.guarded_transition[key] = old

    def test_each_shared_and_routing_source_is_live_pinned(self):
        for owner in {*self.guarded_transition['shared_sources_sha256'],
                      *self.guarded_transition['test_sources_sha256']}:
            path = self.root / owner; old = path.read_bytes(); path.write_bytes(old + b'\n// drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_bytes(old)

    def test_old_calls_aliases_and_checked_helper_changes_fail(self):
        selector = self.root / guarded.SELECTOR; old = selector.read_text()
        for text in ['typed_plan::lower_guarded_deposit()',
                     'use super::typed_plan::lower_guarded_deposit as old_deposit;',
                     'use super::typed_plan::{lower_guarded_deposit as old_deposit};']:
            selector.write_text(old + '\n' + text)
            self.assertFalse(self.report()['passed'])
        selector.write_text(old.replace('fn checked_guarded_deposit_domain(', 'fn unauthorized_domain('))
        self.assertFalse(self.report()['passed'])
        selector.write_text(old)

    def test_observation_and_merge_manifests_cannot_be_rehashed(self):
        self.probe.write_text(self.probe.read_text() + '\nfn changed_probe() {}\n')
        self.extension['reviewed_source_sha256'] = inventory.digest(self.probe.read_bytes())
        self.assertFalse(self.report()['passed'])
        self.merge_transition['reviewed_source_fingerprints'][merge.VOTING_TEST] = \
            inventory.digest((self.root / merge.VOTING_TEST).read_bytes())
        self.assertFalse(self.report()['passed'])

    def test_historical_immediate_guarded_snapshot_is_exact_and_narrow(self):
        sources = {owner: (self.root / owner).read_text() for owner in self.baseline['source_fingerprints']}
        at_550 = {**sources, **previous.zswap_checkpoint_sources(inventory.ROOT)}
        at_550[immediate.ATTEMPT] = (self.root / immediate.ATTEMPT).read_text()
        old_merge = previous.merge_checkpoint_sources(inventory.ROOT)[merge.MERGE]
        old_guarded = guarded.predecessor_sources(self.root, self.guarded_transition)[guarded.GUARDED]
        self.assertEqual(immediate.check_sources(self.root, at_550, self.immediate_transition,
                                               old_merge, old_guarded), [])
        self.assertEqual(immediate.check_sources(self.root, at_550, self.immediate_transition,
                                                old_merge, old_guarded + '\n'),
                         ['immediate retirement: shared caller changed: ' + guarded.GUARDED])

    def test_historical_merge_voting_snapshot_is_exact_and_narrow(self):
        sources = {owner: (self.root / owner).read_text() for owner in self.baseline['source_fingerprints']}
        prior = guarded.predecessor_sources(self.root, self.guarded_transition)
        sources.update(prior)
        self.assertEqual(merge.check_sources(self.root, sources, self.merge_transition,
                                            prior[guarded.VOTING_TEST]), [])
        self.assertEqual(merge.check_sources(self.root, sources, self.merge_transition,
                                            prior[guarded.VOTING_TEST] + '\n'),
                         ['merge retirement: reviewed routing test changed: ' + guarded.VOTING_TEST])

    def test_cli_cannot_overwrite_new_evidence(self):
        original = previous.GUARDED_TRANSITION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--guarded-deposit-transition', str(previous.GUARDED_TRANSITION),
                            '--output', str(previous.GUARDED_TRANSITION)])
        self.assertEqual(previous.GUARDED_TRANSITION.read_bytes(), original)


if __name__ == '__main__':
    unittest.main()
