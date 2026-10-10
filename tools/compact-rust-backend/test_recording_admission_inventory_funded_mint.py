# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Funded mint retires while all historical source authority remains authenticated."""
import contextlib
import copy
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_funded_mint as funded
import recording_admission_inventory_shielded_payout as payout
import recording_admission_inventory_reset_payout as reset
import recording_admission_inventory_guarded as guarded
import recording_admission_inventory_immediate as immediate
import recording_admission_inventory_merge as merge
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_inequality_test as prior_tests


class FundedMintTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.InequalityTestExtensionTests.setUp(self)
        self.funded_transition = json.loads(previous.FUNDED_MINT_TRANSITION.read_text())
        for owner in [*funded.REVIEWED_OWNERS, *self.funded_transition['shared_sources_sha256'],
                      *self.funded_transition['test_sources_sha256']]:
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
            shielded_payout_transition=self.payout_transition,
            inequality_test_extension=self.inequality_extension,
            funded_mint_transition=self.funded_transition)

    def test_only_funded_route_retires_and_all_seven_predecessors_are_exact(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 1)
        self.assertEqual([r['id'] for r in report['retired_routes']], funded.RETIRED)
        self.assertEqual(len(funded.RETIRED), 19)
        self.assertEqual(report['reviewed_inequality_test_extension']['issue'], 571)
        old = funded.predecessor_sources(self.root, self.funded_transition)
        for owner, expected in funded.PRIOR_OWNERS.items():
            self.assertEqual(inventory.digest(old[owner].encode()), expected)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertNotIn(funded.ROUTE, ids)
        self.assertEqual(ids.count(funded.REPLACEMENT), 1)

    def test_missing_successor_and_every_ancestor_fail_closed(self):
        for field in ['funded_transition', 'inequality_extension', 'payout_transition', 'reset_transition',
                      'routing_extension', 'guarded_transition', 'merge_transition', 'transition',
                      'instrumentation', 'jubjub_transition', 'call_extension', 'terminal_transition',
                      'unit_transition', 'context_transition', 'kernel_transition', 'composite_transition',
                      'receive_transition', 'unit_domain_extension', 'voting_transition', 'phase_transition',
                      'immediate_transition', 'zswap_transition', 'membership_transition', 'send_transition', 'extension']:
            old = getattr(self, field); setattr(self, field, None)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_current_owners_and_each_private_guard_are_pinned(self):
        for owner in funded.REVIEWED_OWNERS:
            path = self.root / owner; old = path.read_text()
            path.write_text(old + '\n// changed owner\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(old)
        for owner, marker in [(funded.TYPED, "pub(super) fn lower_funded_mint<'a>("),
                              (funded.FUNDED, "pub(super) fn lower<'a>("),
                              (funded.GUARDED, '#[derive(Clone, Copy, Default)]\nstruct Flow'),
                              (funded.GUARDED, 'struct Audit'),
                              (funded.GUARDED, "impl Audit<'_>"),
                              (funded.GUARDED, 'pub(super) fn prefix(')]:
            path = self.root / owner; old = path.read_text()
            self.assertIn('#[cfg(test)]\n' + marker, old)
            path.write_text(old.replace('#[cfg(test)]\n' + marker, marker, 1))
            self.assertFalse(self.report()['passed'], marker)
            path.write_text(old)

    def test_shared_forwarding_and_funded_helpers_stay_live(self):
        for owner, marker in [(funded.TYPED, 'mod funded_mint;'),
                              (funded.GUARDED, 'pub(super) fn forwards_received_coin('),
                              (funded.GUARDED, 'pub(super) fn read_type('),
                              (funded.FUNDED, 'pub(super) fn funded_mint(')]:
            path = self.root / owner; old = path.read_text(); self.assertIn(marker, old)
            path.write_text(old.replace(marker, '#[cfg(test)]\n' + marker, 1))
            self.assertFalse(self.report()['passed'], marker)
            path.write_text(old)

    def test_all_shared_dependencies_and_routing_tests_are_pinned(self):
        self.assertEqual(self.funded_transition['prior_shared_sources_sha256'], self.payout_transition['shared_sources_sha256'])
        self.assertEqual(len(self.funded_transition['shared_sources_sha256']), 23)
        for owner in {*self.funded_transition['shared_sources_sha256'], *self.funded_transition['test_sources_sha256']}:
            path = self.root / owner; old = path.read_bytes()
            path.write_bytes(old + b'\n// drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_bytes(old)

    def test_rehashed_manifest_empty_maps_and_bad_reconstruction_cannot_authorize(self):
        original = copy.deepcopy(self.funded_transition)
        for key in ['predecessor_reconstruction', 'reviewed_source_fingerprints',
                    'prior_source_fingerprints', 'shared_sources_sha256', 'test_sources_sha256']:
            self.funded_transition[key] = {}
            self.assertFalse(self.report()['passed'], key)
            self.funded_transition = copy.deepcopy(original)
        self.funded_transition['claim'] += ' forged'
        with self.assertRaises(ValueError): funded.predecessor_sources(self.root, self.funded_transition)

    def test_old_calls_aliases_and_existing_composite_changes_fail(self):
        path = self.root / funded.SELECTOR; old = path.read_text()
        for addition in ['typed_plan::lower_funded_mint()',
                         'use super::typed_plan::lower_funded_mint as legacy;',
                         'use super::typed_plan::{lower_funded_mint as legacy};',
                         'funded_mint::lower()']:
            path.write_text(old + '\n' + addition)
            self.assertFalse(self.report()['passed'])
        path.write_text(old.replace('fn checked_composite_attempt(', 'fn changed_interface('))
        self.assertFalse(self.report()['passed'])
        path.write_text(old)

    def test_shielded_guarded_and_test_overlays_keep_exact_previous_bytes(self):
        sources = {owner: (self.root / owner).read_text() for owner in {*self.baseline['source_fingerprints'], *payout.REVIEWED_OWNERS}}
        prior = funded.predecessor_sources(self.root, self.funded_transition)
        sources.update(prior)
        values = [prior[funded.GUARDED], prior[funded.VOTING_TEST], prior[funded.GUARDED_TEST]]
        self.assertEqual(payout.check_sources(self.root, sources, self.payout_transition, *values), [])
        for i, owner in enumerate([funded.GUARDED, funded.VOTING_TEST, funded.GUARDED_TEST]):
            changed = values.copy(); changed[i] += '\n'
            expected = 'shared admission dependency changed' if i == 0 else 'reviewed routing test changed'
            self.assertEqual(payout.check_sources(self.root, sources, self.payout_transition, *changed),
                             ['shielded payout retirement: ' + expected + ': ' + owner])
        other = next(p for p in self.payout_transition['shared_sources_sha256'] if p != funded.GUARDED)
        path = self.root / other; path.write_text(path.read_text() + '\n// unrelated\n')
        self.assertTrue(payout.check_sources(self.root, sources, self.payout_transition, *values))

    def test_reset_guarded_overlay_is_exact_and_cannot_hide_other_source(self):
        current = {owner: (self.root / owner).read_text() for owner in {*self.baseline['source_fingerprints'], *payout.REVIEWED_OWNERS}}
        prior = funded.predecessor_sources(self.root, self.funded_transition)
        old = payout.predecessor_source_snapshot({**current, **prior}, self.payout_transition)
        values = [old[payout.PAYOUT], old[payout.VOTING_TEST], old[payout.GUARDED_TEST],
                  old[payout.RESET_TEST], prior[funded.GUARDED]]
        self.assertEqual(reset.check_sources(self.root, {**current, **prior, **old}, self.reset_transition, *values), [])
        values[-1] += '\n'
        self.assertEqual(reset.check_sources(self.root, {**current, **prior, **old}, self.reset_transition, *values),
                         ['reset payout retirement: shared admission dependency changed: ' + funded.GUARDED])

    def test_invalid_571_or_570_predecessor_cannot_authorize(self):
        for field in ['inequality_extension', 'payout_transition', 'routing_extension', 'guarded_transition']:
            old = getattr(self, field); forged = copy.deepcopy(old); forged['claim'] += ' forged'
            setattr(self, field, forged)
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertNotIn(funded.ROUTE, [r['id'] for r in report['retired_routes']])
            setattr(self, field, old)

    def test_current_routing571_file_cannot_hide_behind_successor(self):
        self.test_owner.write_text(self.test_owner.read_text() + '\n// current test drift\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_successor_manifest(self):
        original = previous.FUNDED_MINT_TRANSITION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--funded-mint-transition', str(previous.FUNDED_MINT_TRANSITION),
                            '--output', str(previous.FUNDED_MINT_TRANSITION)])
        self.assertEqual(previous.FUNDED_MINT_TRANSITION.read_bytes(), original)


if __name__ == '__main__': unittest.main()
