# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Prospective merge retirement must preserve both historical and shared authority."""
import copy
import json
import unittest
import contextlib
import io
import recording_admission_inventory as inventory
import recording_admission_inventory_merge as merge
import recording_admission_inventory_send as send
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_observation_test as prior_tests


class MergeTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.ObservationTestExtensionTests.setUp(self)
        self.merge_transition = json.loads(previous.MERGE_TRANSITION.read_text())
        predecessor = previous.guarded_checkpoint_sources(inventory.ROOT)
        for owner in [*merge.REVIEWED_OWNERS, *self.merge_transition['test_sources_sha256'],
                      *self.merge_transition['shared_sources_sha256']]:
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
            merge_transition=self.merge_transition)

    def test_retirement_changes_only_one_route_with_authenticated_predecessor(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 5)
        self.assertEqual([row['id'] for row in report['retired_routes']], merge.RETIRED)
        self.assertEqual(len(report['retired_routes']), 15)
        old = merge.predecessor_sources(self.root, self.merge_transition)
        for owner in [merge.SELECTOR, merge.TYPED]:
            self.assertEqual(inventory.digest(old[owner].encode()), send.REVIEWED_OWNERS[owner])
        self.assertEqual(report['reviewed_observation_test_extension']['issue'], 564)

    def test_missing_transition_does_not_hide_removed_route(self):
        self.merge_transition = None
        self.assertFalse(self.report()['passed'])

    def test_every_predecessor_including_observation_extension_remains_required(self):
        for field in ['transition', 'instrumentation', 'jubjub_transition', 'call_extension',
                      'terminal_transition', 'unit_transition', 'context_transition', 'kernel_transition',
                      'composite_transition', 'receive_transition', 'unit_domain_extension',
                      'voting_transition', 'phase_transition', 'immediate_transition',
                      'zswap_transition', 'membership_transition', 'send_transition', 'extension']:
            old = getattr(self, field)
            setattr(self, field, None)
            self.assertFalse(self.report()['passed'], field)
            setattr(self, field, old)

    def test_current_owners_and_cfg_guards_cannot_be_rewritten(self):
        for owner in merge.REVIEWED_OWNERS:
            path = self.root / owner
            old = path.read_text()
            path.write_text(old + '\n// unrelated mutation\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(old)
        for owner, marker in [(merge.TYPED, "pub(super) fn lower_shielded_merge<'a>("),
                              (merge.MERGE, "pub(super) fn lower<'a>(")]:
            path = self.root / owner
            old = path.read_text()
            path.write_text(old.replace('#[cfg(test)]\n' + marker, marker))
            self.assertFalse(self.report()['passed'])
            path.write_text(old)

    def test_shared_module_constants_and_received_bridge_must_stay_live(self):
        for owner, marker in [(merge.TYPED, 'mod shielded_merge;'),
                              (merge.MERGE, 'pub(super) fn received_helper('),
                              (merge.MERGE, 'fn received_bridge('),
                              (merge.MERGE, 'pub(super) const INPUT:')]:
            path = self.root / owner
            old = path.read_text()
            self.assertIn(marker, old)
            path.write_text(old.replace(marker, '#[cfg(test)]\n' + marker, 1))
            self.assertFalse(self.report()['passed'])
            path.write_text(old)

    def test_caller_rehash_and_reconstruction_edits_cannot_authorize(self):
        forged = copy.deepcopy(self.merge_transition)
        forged['claim'] += ' unreviewed'
        with self.assertRaises(ValueError):
            merge.predecessor_sources(self.root, forged)
        for key in ['prior_source_fingerprints', 'reviewed_source_fingerprints', 'predecessor_reconstruction',
                    'shared_sources_sha256', 'test_sources_sha256']:
            old = self.merge_transition[key]
            self.merge_transition[key] = {}
            self.assertFalse(self.report()['passed'], key)
            self.merge_transition[key] = old

    def test_probe_extension_is_still_authenticated_not_an_owner_hash_override(self):
        self.probe.write_text(self.probe.read_text() + '\nfn changed_probe() {}\n')
        self.extension['reviewed_source_sha256'] = inventory.digest(self.probe.read_bytes())
        self.assertFalse(self.report()['passed'])

    def test_shared_dependencies_and_routing_tests_remain_pinned(self):
        for key in ['shared_sources_sha256', 'test_sources_sha256']:
            for owner in self.merge_transition[key]:
                path = self.root / owner
                old = path.read_bytes()
                path.write_bytes(old + b'\n// changed\n')
                self.assertFalse(self.report()['passed'], owner)
                path.write_bytes(old)

    def test_legacy_call_import_alias_and_earlier_test_tampering_fail(self):
        path = self.root / merge.SELECTOR
        old = path.read_text()
        for extra in ['\nfn accidental() { typed_plan :: lower_shielded_merge (); }\n',
                      '\nuse super::typed_plan::lower_shielded_merge as old_merge;\n']:
            path.write_text(old + extra)
            self.assertFalse(self.report()['passed'])
        path.write_text(old)
        path = self.root / 'tools/compact-rust-backend/src/recorded/profile_selection/send_migration_tests.rs'
        path.write_text(path.read_text() + '\n// historical proof changed\n')
        self.assertFalse(self.report()['passed'])

    def test_receive_and_immediate_shared_caller_pins_cannot_be_rewritten(self):
        # These older manifests still attest the original whole merge module.
        # The new source reconstruction must satisfy them, not refresh them.
        for manifest, field in [(self.receive_transition, 'retained_shared_callers_sha256'),
                                (self.immediate_transition, 'shared_callers_sha256')]:
            original = manifest[field][merge.MERGE]
            manifest[field][merge.MERGE] = '0' * 64
            self.assertFalse(self.report()['passed'])
            manifest[field][merge.MERGE] = original
        # The narrow #550 helper also rejects a tampered historical snapshot.
        old = merge.predecessor_sources(self.root, self.merge_transition)
        import recording_admission_inventory_immediate as immediate
        import recording_admission_inventory_receive as receive
        sources = {owner: (self.root / owner).read_text()
                   for owner in self.baseline['source_fingerprints']}
        errors = receive.check_sources(self.root, {**sources, merge.MERGE: old[merge.MERGE] + '\n'},
                                       self.receive_transition)
        self.assertIn('receive retirement: retained shared caller changed: ' + merge.MERGE, errors)
        at_550 = {**sources, **previous.zswap_checkpoint_sources(inventory.ROOT)}
        at_550[immediate.ATTEMPT] = (self.root / immediate.ATTEMPT).read_text()
        self.assertEqual(immediate.check_sources(self.root, at_550, self.immediate_transition,
                                                 old[merge.MERGE]), [])
        errors = immediate.check_sources(self.root, at_550, self.immediate_transition,
                                         old[merge.MERGE] + '\n')
        self.assertEqual(errors, ['immediate retirement: shared caller changed: ' + merge.MERGE])

    def test_cli_protects_new_manifest(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as failure:
            inventory.main(['--merge-transition', str(previous.MERGE_TRANSITION),
                            '--output', str(previous.MERGE_TRANSITION)])
        self.assertEqual(failure.exception.code, 2)
