# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Reset payout subsumption authenticates previous shared and test-only sources."""
import contextlib
import copy
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_reset_payout as reset
import recording_admission_inventory_guarded as guarded
import recording_admission_inventory_phase_reset as phase
import recording_admission_inventory_immediate as immediate
import recording_admission_inventory_merge as merge
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_routing_test as prior_tests


class ResetPayoutTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.RoutingTestExtensionTests.setUp(self)
        self.reset_transition = json.loads(previous.RESET_PAYOUT_TRANSITION.read_text())
        predecessor = previous.shielded_payout_checkpoint_sources(inventory.ROOT)
        for owner in [*reset.REVIEWED_OWNERS, *self.reset_transition['shared_sources_sha256'],
                      *self.reset_transition['test_sources_sha256']]:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(predecessor[owner].encode() if owner in predecessor
                             else (inventory.ROOT / owner).read_bytes())

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
            reset_payout_transition=self.reset_transition)

    def test_only_reset_route_retires_and_all_five_predecessors_are_exact(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 3)
        self.assertEqual([r['id'] for r in report['retired_routes']], reset.RETIRED)
        self.assertEqual(len(reset.RETIRED), 17)
        self.assertEqual(report['reviewed_routing_test_extension']['issue'], 568)
        old = reset.predecessor_sources(self.root, self.reset_transition)
        for owner, expected in reset.PRIOR_OWNERS.items():
            self.assertEqual(inventory.digest(old[owner].encode()), expected)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertNotIn(reset.ROUTE, ids)
        self.assertEqual(ids.count(reset.REPLACEMENT), 1)

    def test_missing_successor_or_each_predecessor_fails(self):
        for field in ['reset_transition', 'routing_extension', 'guarded_transition', 'merge_transition',
                      'transition', 'instrumentation', 'jubjub_transition', 'call_extension',
                      'terminal_transition', 'unit_transition', 'context_transition', 'kernel_transition',
                      'composite_transition', 'receive_transition', 'unit_domain_extension',
                      'voting_transition', 'phase_transition', 'immediate_transition', 'zswap_transition',
                      'membership_transition', 'send_transition', 'extension']:
            old = getattr(self, field); setattr(self, field, None)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_each_actual_owner_and_root_guard_remains_immutable(self):
        for owner in reset.REVIEWED_OWNERS:
            path = self.root / owner; old = path.read_text()
            path.write_text(old + '\n// unreviewed owner\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(old)
        for owner, marker in [(reset.TYPED, "pub(super) fn lower_reset_payout<'a>("),
                              (reset.RESET, "pub(super) fn lower<'a>(")]:
            path = self.root / owner; old = path.read_text()
            self.assertIn('#[cfg(test)]\n' + marker, old)
            path.write_text(old.replace('#[cfg(test)]\n' + marker, marker, 1))
            self.assertFalse(self.report()['passed'])
            path.write_text(old)

    def test_shared_reset_module_and_helpers_must_stay_live(self):
        for owner, marker in [(reset.TYPED, 'mod reset_payout;'),
                              (reset.RESET, 'pub(super) fn read_type('),
                              (reset.RESET, 'pub(super) fn write_type('),
                              (reset.RESET, 'pub(super) fn true_arguments('),
                              (reset.RESET, 'pub(super) fn helper_shape(')]:
            path = self.root / owner; old = path.read_text(); self.assertIn(marker, old)
            path.write_text(old.replace(marker, '#[cfg(test)]\n' + marker, 1))
            self.assertFalse(self.report()['passed'], marker)
            path.write_text(old)

    def test_all_retained_shared_sources_and_reviewed_tests_are_pinned(self):
        self.assertEqual(len(self.reset_transition['shared_sources_sha256']), 23)
        for owner in {*self.reset_transition['shared_sources_sha256'],
                      *self.reset_transition['test_sources_sha256']}:
            path = self.root / owner; old = path.read_bytes()
            path.write_bytes(old + b'\n// dependency drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_bytes(old)

    def test_rehashed_manifest_and_empty_maps_cannot_authorize(self):
        original = copy.deepcopy(self.reset_transition)
        for key in ['predecessor_reconstruction', 'reviewed_source_fingerprints',
                    'prior_source_fingerprints', 'shared_sources_sha256', 'test_sources_sha256']:
            self.reset_transition[key] = {}
            self.assertFalse(self.report()['passed'], key)
            self.reset_transition = copy.deepcopy(original)
        self.reset_transition['claim'] += ' arbitrary authority'
        with self.assertRaises(ValueError):
            reset.predecessor_sources(self.root, self.reset_transition)

    def test_old_calls_aliases_and_existing_composite_changes_fail(self):
        path = self.root / reset.SELECTOR; old = path.read_text()
        for addition in ['typed_plan::lower_reset_payout()',
                         'use super::typed_plan::lower_reset_payout as legacy;',
                         'use super::typed_plan::{lower_reset_payout as legacy};',
                         'reset_payout::lower()']:
            path.write_text(old + '\n' + addition)
            self.assertFalse(self.report()['passed'])
        path.write_text(old.replace('fn checked_composite_attempt(', 'fn changed_interface('))
        self.assertFalse(self.report()['passed'])
        path.write_text(old)

    def test_historical_phase_and_immediate_reset_overlays_are_exact(self):
        sources = {owner: (self.root / owner).read_text() for owner in self.baseline['source_fingerprints']}
        old_reset = reset.predecessor_sources(self.root, self.reset_transition)[reset.RESET]
        at_phase = {**sources, **previous.immediate_checkpoint_sources(inventory.ROOT)}
        self.assertEqual(phase.check_sources(self.root, at_phase, self.phase_transition, old_reset), [])
        self.assertEqual(phase.check_sources(self.root, at_phase, self.phase_transition, old_reset + '\n'),
                         ['phase retirement: shared caller changed: ' + reset.RESET])
        at_immediate = {**sources, **previous.zswap_checkpoint_sources(inventory.ROOT)}
        at_immediate[immediate.ATTEMPT] = (self.root / immediate.ATTEMPT).read_text()
        old_merge = previous.merge_checkpoint_sources(inventory.ROOT)[merge.MERGE]
        old_guarded = previous.guarded_checkpoint_sources(inventory.ROOT)[guarded.GUARDED]
        self.assertEqual(immediate.check_sources(self.root, at_immediate, self.immediate_transition,
                                                old_merge, old_guarded, old_reset), [])
        self.assertEqual(immediate.check_sources(self.root, at_immediate, self.immediate_transition,
                                                old_merge, old_guarded, old_reset + '\n'),
                         ['immediate retirement: shared caller changed: ' + reset.RESET])

    def test_historical_guarded_test_overlays_are_narrow(self):
        sources = {owner: (self.root / owner).read_text() for owner in self.baseline['source_fingerprints']}
        prior = reset.predecessor_sources(self.root, self.reset_transition)
        sources.update(prior)
        vote, route = prior[reset.VOTING_TEST], prior[reset.GUARDED_TEST]
        self.assertEqual(guarded.check_sources(self.root, sources, self.guarded_transition, vote, route), [])
        for bad_vote, bad_route, owner in [(vote + '\n', route, reset.VOTING_TEST),
                                          (vote, route + '\n', reset.GUARDED_TEST)]:
            self.assertEqual(guarded.check_sources(self.root, sources, self.guarded_transition,
                                                 bad_vote, bad_route),
                             ['guarded deposit retirement: reviewed routing test changed: ' + owner])
        other = next(p for p in self.guarded_transition['test_sources_sha256']
                     if p not in (reset.VOTING_TEST, reset.GUARDED_TEST))
        path = self.root / other; path.write_text(path.read_text() + '\n// unrelated\n')
        self.assertTrue(guarded.check_sources(self.root, sources, self.guarded_transition, vote, route))

    def test_invalid_routing568_and_rehashed_guarded566_cannot_authorize(self):
        for field in ['routing_extension', 'guarded_transition']:
            old = getattr(self, field); forged = copy.deepcopy(old); forged['claim'] += ' forged'
            setattr(self, field, forged)
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertNotIn(reset.ROUTE, [r['id'] for r in report['retired_routes']])
            setattr(self, field, old)

    def test_corrupt_current_routing_test_cannot_hide_behind_historical_overlay(self):
        self.test_owner.write_text(self.test_owner.read_text() + '\n// changed live test\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_successor_manifest(self):
        original = previous.RESET_PAYOUT_TRANSITION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--reset-payout-transition', str(previous.RESET_PAYOUT_TRANSITION),
                            '--output', str(previous.RESET_PAYOUT_TRANSITION)])
        self.assertEqual(previous.RESET_PAYOUT_TRANSITION.read_bytes(), original)


if __name__ == '__main__':
    unittest.main()
