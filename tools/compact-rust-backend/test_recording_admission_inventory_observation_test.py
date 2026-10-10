# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""One authenticated test replacement cannot authorize instrumentation drift."""
import copy
import json
import unittest
import contextlib
import io

import recording_admission_inventory as inventory
import recording_admission_inventory_observation_test as observation
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_send as prior_tests


class ObservationTestExtensionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.SendTransitionTests.setUp(self)
        self.extension = json.loads(previous.OBSERVATION_TEST_EXTENSION.read_text())
        self.probe = self.root / observation.OWNER
        self.probe.write_bytes((inventory.ROOT / observation.OWNER).read_bytes())

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
            send_transition=self.send_transition, observation_test_extension=self.extension)

    def test_exact_test_adaptation_preserves_old_bytes_counts_and_route_ids(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 6)
        self.assertEqual(len(report['retired_routes']), 14)
        self.assertEqual(report['reviewed_instrumentation']['instrumented_route_ids'],
                         self.send_transition['instrumented_route_ids'])
        old = observation.predecessor_source(self.probe.read_text(), self.extension)
        self.assertEqual(inventory.digest(old.encode()), self.call_extension['reviewed_source_fingerprints'][observation.OWNER])
        self.assertEqual(self.extension['new_retired_routes'], [])

    def test_missing_extension_keeps_original_instrumentation_failure(self):
        self.extension = None
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertIn('instrumentation: reviewed observation owner changed: ' + observation.OWNER, report['errors'])

    def test_each_prior_manifest_is_still_required(self):
        for field in ['transition', 'instrumentation', 'jubjub_transition', 'call_extension',
                      'terminal_transition', 'unit_transition', 'context_transition', 'kernel_transition',
                      'composite_transition', 'receive_transition', 'unit_domain_extension',
                      'voting_transition', 'phase_transition', 'immediate_transition',
                      'zswap_transition', 'membership_transition', 'send_transition']:
            with self.subTest(field=field):
                old = getattr(self, field)
                setattr(self, field, None)
                self.assertFalse(self.report()['passed'])
                setattr(self, field, old)

    def test_invalid_predecessor_and_caller_rehash_cannot_authorize(self):
        self.send_transition['claim'] += ' unreviewed'
        self.extension['prior_manifests']['send'] = observation.canonical(self.send_transition)
        self.assertFalse(self.report()['passed'])

    def test_exact_manifest_rejects_all_caller_controlled_metadata_changes(self):
        for key, value in [('issue', 565), ('owner', 'other.rs'), ('prior_source_sha256', '0'*64),
                           ('reviewed_source_sha256', '0'*64), ('new_retired_routes', ['fake'])]:
            with self.subTest(key=key):
                old = self.extension[key]
                self.extension[key] = value
                self.assertFalse(self.report()['passed'])
                self.extension[key] = old

    def test_test_body_edit_or_removed_attribute_cannot_be_rehashed(self):
        original = self.probe.read_text()
        for mutation in [original.replace('mandatory complete legacy start-funding control', 'weaker control'),
                         original.replace('#[test]\nfn retained_start_funding', 'fn retained_start_funding'),
                         original + '\nfn unrelated_probe_change() {}\n']:
            with self.subTest(mutation=mutation[-80:]):
                self.probe.write_text(mutation)
                self.assertFalse(self.report()['passed'])
                forged = copy.deepcopy(self.extension)
                forged['reviewed_source_sha256'] = inventory.digest(mutation.encode())
                with self.assertRaises(ValueError):
                    observation.predecessor_source(mutation, forged)
        self.probe.write_text(original)

    def test_other_observation_owner_remains_live_not_overlaid(self):
        trace = self.root / 'tools/compact-rust-backend/src/recorded/admission_probe/trace.rs'
        trace.write_text(trace.read_text() + '\nfn unrelated_trace_change() {}\n')
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertTrue(any('reviewed observation owner changed' in e for e in report['errors']))

    def test_invalid_historical_test_pins_remain_enforced(self):
        path = self.root / 'tools/compact-rust-backend/src/recorded/profile_selection/send_migration_tests.rs'
        path.write_text(path.read_text() + '\n// changed historical evidence\n')
        self.assertFalse(self.report()['passed'])

    def test_missing_probe_is_an_explicit_failure(self):
        self.probe.unlink()
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_adaptation_evidence(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as failure:
            inventory.main(['--observation-test-extension', str(previous.OBSERVATION_TEST_EXTENSION),
                            '--output', str(previous.OBSERVATION_TEST_EXTENSION)])
        self.assertEqual(failure.exception.code, 2)
