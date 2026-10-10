# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Send subsumption freezes production, diagnostics, and the complete prior chain."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_send as send
import recording_admission_inventory_membership as membership
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_membership as prior_tests


class SendTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.MembershipTransitionTests.setUp(self)
        self.send_transition = json.loads(previous.SEND_TRANSITION.read_text())
        predecessor = previous.merge_checkpoint_sources(inventory.ROOT)
        for owner in [*send.REVIEWED_OWNERS, *self.send_transition['test_sources_sha256'],
                      *self.send_transition['shared_sources_sha256']]:
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
            zswap_transition=self.zswap_transition, membership_transition=self.membership_transition,
            send_transition=self.send_transition)

    def test_exact_subsumption_retains_composite_priority_and_full_history(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([r['id'] for r in report['retired_routes']], send.RETIRED)
        self.assertEqual(len(report['retired_routes']), 14)
        self.assertTrue(all(r['passed'] for r in report['retired_routes']))
        self.assertEqual(report['current_selector']['ordered_calls'],
                         [c for c in self.membership_transition['reviewed_selector']['ordered_calls']
                          if c != 'typed_plan::lower_shielded_send_checked'])
        self.assertEqual(len(report['current_selector']['ordered_calls']), 6)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertEqual(ids.count(send.REPLACEMENT), 1)
        self.assertNotIn(send.ROUTE, ids)
        self.assertEqual(len(ids), len(self.membership_transition['instrumented_route_ids']) - 1)
        prior = send.predecessor_sources(self.root, self.send_transition)
        for owner, expected in send.PRIOR_OWNERS.items():
            self.assertEqual(send.digest(prior[owner].encode()), expected)
        membership.predecessor_source_snapshot(prior, self.membership_transition)

    def test_missing_or_invalid_transition_cannot_expose_predecessors(self):
        original = self.send_transition
        for transition in [None, {**original, 'issue': 999}]:
            self.send_transition = transition
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertNotIn(send.ROUTE, [r['id'] for r in report['retired_routes']])

    def test_all_historical_authorization_is_required(self):
        for name in ['membership_transition', 'zswap_transition', 'immediate_transition',
                     'phase_transition', 'voting_transition', 'unit_domain_extension',
                     'receive_transition', 'kernel_transition', 'call_extension']:
            original = getattr(self, name)
            setattr(self, name, None)
            report = self.report()
            self.assertFalse(report['passed'], name)
            self.assertNotIn(send.ROUTE, [r['id'] for r in report['retired_routes']], name)
            setattr(self, name, original)

    def test_historical_manifest_cannot_be_rehashed_into_new_authority(self):
        self.membership_transition['claim'] += ' unauthorized'
        self.send_transition['prior_membership_canonical_sha256'] = membership.canonical(self.membership_transition)
        self.assertFalse(self.report()['passed'])

    def test_each_current_owner_is_authenticated_before_reconstruction(self):
        for owner in send.REVIEWED_OWNERS:
            path = self.root / owner
            before = path.read_bytes()
            path.write_bytes(before + b'\n// unreviewed owner\n')
            with self.assertRaisesRegex(ValueError, 'current send owner differs'):
                send.predecessor_sources(self.root, self.send_transition)
            self.assertFalse(self.report()['passed'])
            path.write_bytes(before)

    def test_caller_cannot_refresh_any_identity_or_reconstruction(self):
        original = copy.deepcopy(self.send_transition)
        for key in ['prior_source_fingerprints', 'reviewed_source_fingerprints', 'predecessor_reconstruction',
                    'oracle_functions_sha256', 'test_sources_sha256', 'shared_sources_sha256',
                    'reviewed_selector', 'instrumented_route_ids', 'retired_route_ids', 'helper_sha256']:
            self.send_transition = copy.deepcopy(original)
            self.send_transition[key] = {}
            self.assertFalse(self.report()['passed'], key)
        self.send_transition = original

    def test_each_original_oracle_guard_and_body_is_required(self):
        path = self.root / send.TYPED
        before = path.read_text()
        for marker in send.GUARDS:
            guarded = '#[cfg(test)]\n' + marker
            self.assertEqual(before.count(guarded), 1)
            path.write_text(before.replace(guarded, marker, 1))
            self.assertFalse(self.report()['passed'], marker)
        path.write_text(before.replace('plan.evidence.zswap_inputs > 0', 'plan.evidence.zswap_inputs > 99', 1))
        self.assertFalse(self.report()['passed'])
        path.write_text(before)

    def test_live_shared_helpers_cannot_be_hidden(self):
        path = self.root / send.TYPED
        before = path.read_text()
        for marker in ['pub(super) fn shielded_send_result(', 'fn shielded_value_at(', 'fn shielded_plan<',
                       'struct Plan', "pub(super) fn lower_start_funding<'a>("]:
            self.assertIn(marker, before)
            path.write_text(before.replace(marker, '#[cfg(test)]\n' + marker, 1))
            self.assertFalse(self.report()['passed'], marker)
        path.write_text(before)

    def test_composite_attempt_and_diagnostic_authority_are_pinned(self):
        path = self.root / send.SELECTOR
        before = path.read_text()
        for old, new in [('checked_composite_attempt', 'unchecked_composite_attempt'),
                         ('shielded_send_rejection', 'discarded_send_rejection'),
                         ('typed_plan::lower_funded_mint', 'typed_plan::lower_guarded_deposit')]:
            self.assertIn(old, before)
            path.write_text(before.replace(old, new, 1))
            self.assertFalse(self.report()['passed'], old)
        path.write_text(before)

    def test_retired_production_call_and_alias_are_detected(self):
        path = self.root / send.RECORDED
        before = path.read_text()
        for extra in ['\nfn accidental() { typed_plan :: lower_shielded_send_checked (); }\n',
                      '\nuse self::typed_plan::lower_shielded_send_checked as accidental;\n']:
            path.write_text(before + extra)
            sources = {owner: (self.root / owner).read_text() for owner in send.REVIEWED_OWNERS}
            errors = send.check_sources(self.root, sources, self.send_transition)
            self.assertTrue(any('legacy root' in e for e in errors))
            self.assertFalse(self.report()['passed'])
        path.write_text(before)

    def test_negative_domain_dependency_and_rehash_are_rejected(self):
        owner = 'tools/compact-rust-backend/src/recorded/typed_plan/jubjub_values.rs'
        self.assertIn(owner, self.send_transition['shared_sources_sha256'])
        path = self.root / owner
        before = path.read_text()
        guard = '    if plan.admission.composite_domain != CompositeDomain::JubjubValues {\n        return None;\n    }\n'
        self.assertEqual(before.count(guard), 1)
        path.write_text(before.replace(guard, '', 1))
        self.assertFalse(self.report()['passed'])
        self.send_transition['shared_sources_sha256'][owner] = send.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])

    def test_routing_and_diagnostic_test_sources_are_pinned(self):
        for owner in self.send_transition['test_sources_sha256']:
            path = self.root / owner
            before = path.read_bytes()
            path.write_bytes(before + b'\n// unreviewed diagnostic assertion\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_bytes(before)

    def test_reconstructed_membership_checkpoint_still_enforces_old_rules(self):
        snapshot = send.predecessor_sources(self.root, self.send_transition)
        self.assertFalse(membership.check_sources(self.root, snapshot, self.membership_transition))
        snapshot[send.TYPED] += '\n// forged historical owner\n'
        self.assertTrue(membership.check_sources(self.root, snapshot, self.membership_transition))

    def test_cli_cannot_overwrite_send_manifest(self):
        before = previous.SEND_TRANSITION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--send-transition', str(previous.SEND_TRANSITION),
                            '--output', str(previous.SEND_TRANSITION)])
        self.assertEqual(previous.SEND_TRANSITION.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
