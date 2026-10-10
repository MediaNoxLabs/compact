# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Both payout domains retire without rewriting prior shared authority."""
import contextlib
import copy
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_shielded_payout as payout
import recording_admission_inventory_reset_payout as reset
import recording_admission_inventory_guarded as guarded
import recording_admission_inventory_immediate as immediate
import recording_admission_inventory_merge as merge
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_reset_payout as prior_tests


class ShieldedPayoutTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.ResetPayoutTransitionTests.setUp(self)
        self.payout_transition = json.loads(previous.SHIELDED_PAYOUT_TRANSITION.read_text())
        for owner in [*payout.REVIEWED_OWNERS, *self.payout_transition['shared_sources_sha256'],
                      *self.payout_transition['test_sources_sha256']]:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((inventory.ROOT / owner).read_bytes())

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
            shielded_payout_transition=self.payout_transition)

    def test_only_payout_route_retires_and_all_six_predecessors_are_exact(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 2)
        self.assertEqual([r['id'] for r in report['retired_routes']], payout.RETIRED)
        self.assertEqual(len(payout.RETIRED), 18)
        self.assertEqual(report['reviewed_routing_test_extension']['issue'], 568)
        old = payout.predecessor_sources(self.root, self.payout_transition)
        for owner, expected in payout.PRIOR_OWNERS.items():
            self.assertEqual(inventory.digest(old[owner].encode()), expected)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertNotIn(payout.ROUTE, ids)
        self.assertEqual(ids.count(payout.REPLACEMENT), 1)

    def test_missing_successor_or_each_predecessor_fails(self):
        for field in ['payout_transition', 'reset_transition', 'routing_extension', 'guarded_transition', 'merge_transition',
                      'transition', 'instrumentation', 'jubjub_transition', 'call_extension',
                      'terminal_transition', 'unit_transition', 'context_transition', 'kernel_transition',
                      'composite_transition', 'receive_transition', 'unit_domain_extension',
                      'voting_transition', 'phase_transition', 'immediate_transition', 'zswap_transition',
                      'membership_transition', 'send_transition', 'extension']:
            old = getattr(self, field); setattr(self, field, None)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_each_actual_owner_and_root_guard_remains_immutable(self):
        for owner in payout.REVIEWED_OWNERS:
            path = self.root / owner; old = path.read_text()
            path.write_text(old + '\n// unreviewed owner\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(old)
        for owner, marker in [(payout.TYPED, "pub(super) fn lower_shielded_payout<'a>("),
                              (payout.PAYOUT, "pub(super) fn lower<'a>("),
                              (payout.PAYOUT, "fn coin_result("), (payout.PAYOUT, "fn add_paths("), (payout.PAYOUT, "pub(super) fn actionful_call_paths(")]:
            path = self.root / owner; old = path.read_text()
            self.assertIn('#[cfg(test)]\n' + marker, old)
            path.write_text(old.replace('#[cfg(test)]\n' + marker, marker, 1))
            self.assertFalse(self.report()['passed'])
            path.write_text(old)

    def test_shared_reset_module_and_helpers_must_stay_live(self):
        for owner, marker in [(payout.TYPED, 'mod shielded_payout;'),
                              (payout.PAYOUT, 'pub(super) fn cell_type('),
                              (payout.PAYOUT, 'pub(super) fn actionful_cell_type('),
                              (payout.PAYOUT, 'pub(super) fn witness_type('),
                              (payout.PAYOUT, 'pub(super) fn action_shape(')]:
            path = self.root / owner; old = path.read_text(); self.assertIn(marker, old)
            path.write_text(old.replace(marker, '#[cfg(test)]\n' + marker, 1))
            self.assertFalse(self.report()['passed'], marker)
            path.write_text(old)

    def test_all_retained_shared_sources_and_reviewed_tests_are_pinned(self):
        self.assertEqual(self.payout_transition['prior_shared_sources_sha256'], self.reset_transition['shared_sources_sha256'])
        self.assertEqual(self.payout_transition['prior_shared_sources_sha256'][payout.PAYOUT], payout.PRIOR_OWNERS[payout.PAYOUT])
        self.assertEqual(len(self.payout_transition['shared_sources_sha256']), 23)
        for owner in {*self.payout_transition['shared_sources_sha256'],
                      *self.payout_transition['test_sources_sha256']}:
            path = self.root / owner; old = path.read_bytes()
            path.write_bytes(old + b'\n// dependency drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_bytes(old)

    def test_rehashed_manifest_and_empty_maps_cannot_authorize(self):
        original = copy.deepcopy(self.payout_transition)
        for key in ['predecessor_reconstruction', 'reviewed_source_fingerprints',
                    'prior_source_fingerprints', 'shared_sources_sha256', 'test_sources_sha256']:
            self.payout_transition[key] = {}
            self.assertFalse(self.report()['passed'], key)
            self.payout_transition = copy.deepcopy(original)
        self.payout_transition['claim'] += ' arbitrary authority'
        with self.assertRaises(ValueError):
            payout.predecessor_sources(self.root, self.payout_transition)

    def test_old_calls_aliases_and_existing_composite_changes_fail(self):
        path = self.root / payout.SELECTOR; old = path.read_text()
        for addition in ['typed_plan::lower_shielded_payout()',
                         'use super::typed_plan::lower_shielded_payout as legacy;',
                         'use super::typed_plan::{lower_shielded_payout as legacy};',
                         'shielded_payout::lower()']:
            path.write_text(old + '\n' + addition)
            self.assertFalse(self.report()['passed'])
        path.write_text(old.replace('fn checked_composite_attempt(', 'fn changed_interface('))
        self.assertFalse(self.report()['passed'])
        path.write_text(old)

    def test_historical_immediate_whole_payout_exception_keeps_exact_previous_bytes(self):
        sources = {owner: (self.root / owner).read_text() for owner in self.baseline['source_fingerprints']}
        at_immediate = {**sources, **previous.zswap_checkpoint_sources(inventory.ROOT)}
        at_immediate[immediate.ATTEMPT] = (self.root / immediate.ATTEMPT).read_text()
        prior = payout.predecessor_sources(self.root, self.payout_transition)
        at_immediate[payout.PAYOUT] = prior[payout.PAYOUT]
        old_merge = previous.merge_checkpoint_sources(inventory.ROOT)[merge.MERGE]
        old_guarded = previous.guarded_checkpoint_sources(inventory.ROOT)[guarded.GUARDED]
        old_reset = previous.reset_payout_checkpoint_sources(inventory.ROOT)[reset.RESET]
        self.assertEqual(immediate.check_sources(self.root, at_immediate, self.immediate_transition,
                                                old_merge, old_guarded, old_reset), [])
        at_immediate[payout.PAYOUT] += '\n'
        self.assertEqual(immediate.check_sources(self.root, at_immediate, self.immediate_transition,
                                                old_merge, old_guarded, old_reset),
                         ['immediate retirement: frozen payout test-owner source changed'])

    def test_historical_reset_shared_and_three_test_overlays_are_narrow(self):
        sources = {owner: (self.root / owner).read_text() for owner in self.baseline['source_fingerprints']}
        prior = payout.predecessor_sources(self.root, self.payout_transition)
        sources.update(prior)
        values = [prior[payout.PAYOUT], prior[payout.VOTING_TEST],
                  prior[payout.GUARDED_TEST], prior[payout.RESET_TEST]]
        self.assertEqual(reset.check_sources(self.root, sources, self.reset_transition, *values), [])
        for i, owner in enumerate([payout.PAYOUT, payout.VOTING_TEST, payout.GUARDED_TEST, payout.RESET_TEST]):
            changed = values.copy(); changed[i] += '\n'
            expected = 'shared admission dependency changed' if i == 0 else 'reviewed routing test changed'
            self.assertEqual(reset.check_sources(self.root, sources, self.reset_transition, *changed),
                             ['reset payout retirement: ' + expected + ': ' + owner])
        other = next(p for p in self.reset_transition['shared_sources_sha256'] if p != payout.PAYOUT)
        path = self.root / other; path.write_text(path.read_text() + '\n// unrelated\n')
        self.assertTrue(reset.check_sources(self.root, sources, self.reset_transition, *values))

    def test_invalid_routing568_and_rehashed_guarded566_cannot_authorize(self):
        for field in ['routing_extension', 'reset_transition']:
            old = getattr(self, field); forged = copy.deepcopy(old); forged['claim'] += ' forged'
            setattr(self, field, forged)
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertNotIn(payout.ROUTE, [r['id'] for r in report['retired_routes']])
            setattr(self, field, old)

    def test_corrupt_current_routing_test_cannot_hide_behind_historical_overlay(self):
        self.test_owner.write_text(self.test_owner.read_text() + '\n// changed live test\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_successor_manifest(self):
        original = previous.SHIELDED_PAYOUT_TRANSITION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--shielded-payout-transition', str(previous.SHIELDED_PAYOUT_TRANSITION),
                            '--output', str(previous.SHIELDED_PAYOUT_TRANSITION)])
        self.assertEqual(previous.SHIELDED_PAYOUT_TRANSITION.read_bytes(), original)


if __name__ == '__main__':
    unittest.main()
