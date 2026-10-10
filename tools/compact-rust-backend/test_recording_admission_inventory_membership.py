# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Membership retirement authenticates both owners and every earlier checkpoint."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_membership as membership
import recording_admission_inventory_zswap as zswap
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_zswap as prior_tests


class MembershipTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.ZswapTransitionTests.setUp(self)
        self.membership_transition = json.loads(previous.MEMBERSHIP_TRANSITION.read_text())
        predecessor = previous.send_checkpoint_sources(inventory.ROOT)
        for owner in [*membership.REVIEWED_OWNERS, *self.membership_transition['test_sources_sha256'], *self.membership_transition['shared_sources_sha256']]:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(predecessor[owner].encode() if owner in predecessor else (inventory.ROOT / owner).read_bytes())

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
            zswap_transition=self.zswap_transition, membership_transition=self.membership_transition)

    def test_exact_first_priority_cutover_preserves_shared_renderer_and_history(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([r['id'] for r in report['retired_routes']], membership.RETIRED)
        self.assertTrue(all(r['passed'] for r in report['retired_routes']))
        self.assertEqual(len(report['retired_routes']), 13)
        self.assertEqual(report['current_selector']['ordered_calls'], self.zswap_transition['reviewed_selector']['ordered_calls'][1:])
        self.assertEqual(len(report['current_selector']['ordered_calls']), 7)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertEqual(ids.count(membership.REPLACEMENT), 1)
        self.assertNotIn(membership.ROUTE, ids)
        self.assertIn('effectful.start-funding', ids)
        prior = membership.predecessor_sources(self.root, self.membership_transition)
        for owner, expected in membership.PRIOR_OWNERS.items():
            self.assertEqual(membership.digest(prior[owner].encode()), expected)

    def test_missing_or_invalid_transition_never_supplies_predecessors(self):
        for review in [None, {**self.membership_transition, 'issue': 999}]:
            self.membership_transition = review
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertTrue(any('Zswap retirement: predecessor reconstruction failed' in e for e in report['errors']))
            self.assertNotIn(membership.ROUTE, [r['id'] for r in report['retired_routes']])

    def test_predecessor_cannot_be_rewritten_even_with_rehashed_manifest(self):
        self.zswap_transition['claim'] += ' unreviewed'
        self.membership_transition['prior_zswap_canonical_sha256'] = zswap.canonical(self.zswap_transition)
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertNotIn(membership.ROUTE, [r['id'] for r in report['retired_routes']])

    def test_all_earlier_authorization_is_required(self):
        for name in ['zswap_transition', 'immediate_transition', 'phase_transition', 'voting_transition',
                     'unit_domain_extension', 'receive_transition', 'kernel_transition', 'call_extension']:
            original = getattr(self, name)
            setattr(self, name, None)
            report = self.report()
            self.assertFalse(report['passed'], name)
            self.assertNotIn(membership.ROUTE, [r['id'] for r in report['retired_routes']], name)
            setattr(self, name, original)

    def test_source_authentication_is_atomic_for_both_owners(self):
        for owner in membership.REVIEWED_OWNERS:
            path = self.root / owner
            before = path.read_bytes()
            path.write_bytes(before + b'\n// unreviewed owner change\n')
            with self.assertRaisesRegex(ValueError, 'current membership owner differs'):
                membership.predecessor_sources(self.root, self.membership_transition)
            self.assertFalse(self.report()['passed'])
            path.write_bytes(before)

    def test_manifest_cannot_rebaseline_reviewed_source_or_fragments(self):
        original = copy.deepcopy(self.membership_transition)
        for key in ['prior_source_fingerprints', 'reviewed_source_fingerprints', 'predecessor_reconstruction',
                    'oracle_functions_sha256', 'test_sources_sha256', 'shared_sources_sha256', 'retained_shared_production_symbols',
                    'reviewed_selector', 'instrumented_route_ids']:
            self.membership_transition = copy.deepcopy(original)
            self.membership_transition[key] = {}
            self.assertFalse(self.report()['passed'], key)
        self.membership_transition = original

    def test_each_oracle_guard_is_required_without_changing_body(self):
        path = self.root / membership.TYPED
        original = path.read_text()
        for marker in membership.GUARDS:
            guarded = '#[cfg(test)]\n' + marker
            self.assertIn(guarded, original)
            path.write_text(original.replace(guarded, marker, 1))
            self.assertFalse(self.report()['passed'], marker)
        path.write_text(original)

    def test_shared_renderer_cannot_be_hidden_or_deleted(self):
        path = self.root / membership.TYPED
        original = path.read_text()
        for marker in ['struct Plan', 'fn scalar_counter_hash(', 'fn optional_string(',
                       "pub(super) fn lower_start_funding<'a>(", 'mod start_funding;']:
            self.assertIn(marker, original)
            path.write_text(original.replace(marker, '#[cfg(test)]\n' + marker, 1))
            self.assertFalse(self.report()['passed'], marker)
        path.write_text(original)

    def test_original_oracle_body_cannot_be_changed_and_rehashed(self):
        path = self.root / membership.TYPED
        original = path.read_text()
        changed = original.replace('let mut assertions = 0;', 'let mut assertions = 900;', 1)
        self.assertNotEqual(original, changed)
        path.write_text(changed)
        self.membership_transition['reviewed_source_fingerprints'][membership.TYPED] = membership.digest(changed.encode())
        self.assertFalse(self.report()['passed'])

    def test_first_priority_helper_and_observation_are_pinned(self):
        path = self.root / membership.SELECTOR
        original = path.read_text()
        for before, after in [(membership.REPLACEMENT, membership.ROUTE),
                              ('checked_membership_domain', 'unreviewed_membership_domain'),
                              ('typed_plan::lower_funded_mint', 'typed_plan::lower_guarded_deposit')]:
            self.assertIn(before, original)
            path.write_text(original.replace(before, after, 1))
            self.assertFalse(self.report()['passed'], before)
        path.write_text(original)

    def test_production_call_or_alias_cannot_escape_into_other_tracked_owner(self):
        path = self.root / inventory.RECORDED
        original = path.read_text()
        for extra in ['\nfn accidental() { typed_plan :: lower (); }\n',
                      '\nuse self::typed_plan::lower as accidental;\n']:
            path.write_text(original + extra)
            report = self.report()
            self.assertFalse(report['passed'])
            # The full chain may stop earlier at #553's recorded.rs owner pin.
            # Check the new escape detector independently as well as requiring
            # the complete verifier to reject the mutation.
            sources = {owner: (self.root / owner).read_text()
                       for owner in [*membership.REVIEWED_OWNERS, inventory.RECORDED]}
            errors = membership.check_sources(self.root, sources, self.membership_transition)
            self.assertTrue(any('membership retirement: legacy root' in e for e in errors))
        path.write_text(original)

    def test_routing_test_source_is_pinned(self):
        for owner in self.membership_transition['test_sources_sha256']:
            path = self.root / owner
            before = path.read_bytes()
            path.write_bytes(before + b'\n// unreviewed test change\n')
            self.assertFalse(self.report()['passed'])
            path.write_bytes(before)

    def test_shared_dependency_bytes_cannot_drift_under_frozen_root(self):
        owner = next(iter(self.membership_transition['shared_sources_sha256']))
        path = self.root / owner
        path.write_bytes(path.read_bytes() + b'\n// unreviewed shared dependency\n')
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertTrue(any('membership retirement: retained shared lowering source changed' in e for e in report['errors']))

    def test_jubjub_refusal_dependency_cannot_expand_the_frozen_membership_oracle(self):
        owner = 'tools/compact-rust-backend/src/recorded/typed_plan/jubjub_values.rs'
        self.assertIn(owner, self.membership_transition['shared_sources_sha256'])
        path = self.root / owner
        before = path.read_text()
        guard = '    if plan.admission.composite_domain != CompositeDomain::JubjubValues {\n        return None;\n    }\n'
        self.assertEqual(before.count(guard), 1)
        path.write_text(before.replace(guard, '', 1))
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertIn('membership retirement: retained shared lowering source changed: ' + owner,
                      report['errors'])
        # The caller cannot authorize the expanded oracle by refreshing its own hash.
        self.membership_transition['shared_sources_sha256'][owner] = membership.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])

    def test_adapted_routing_tests_preserve_original_zswap_hash_authority(self):
        snapshot = membership.predecessor_sources(self.root, self.membership_transition)
        self.assertEqual(membership.digest(snapshot[membership.ZSWAP_TEST].encode()), zswap.TEST_SHA)
        sources = {owner: (self.root / owner).read_text() for owner in zswap.REVIEWED_OWNERS}
        sources.update(snapshot)
        self.assertFalse(zswap.check_sources(self.root, sources, self.zswap_transition))
        sources[membership.ZSWAP_TEST] += '\n// forged historical test\n'
        self.assertIn('Zswap retirement: reviewed routing test module changed',
                      zswap.check_sources(self.root, sources, self.zswap_transition))

    def test_cli_cannot_overwrite_new_manifest(self):
        before = previous.MEMBERSHIP_TRANSITION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--membership-transition', str(previous.MEMBERSHIP_TRANSITION),
                            '--output', str(previous.MEMBERSHIP_TRANSITION)])
        self.assertEqual(previous.MEMBERSHIP_TRANSITION.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
