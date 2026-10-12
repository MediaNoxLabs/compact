# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Retire exactly one early renderer while preserving the authenticated old domain."""
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_audited_local as interface
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_vector_map as prior_tests
import test_recording_admission_inventory_funded_mint as funded_tests

class AuditedLocalTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.VectorMapExtensionTests.setUp(self)
        self.audited_local_transition = json.loads(previous.AUDITED_LOCAL_TRANSITION.read_text())
        for owner in [*self.audited_local_transition['owners'], *self.audited_local_transition['new_and_shared_sources']]:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((inventory.ROOT / owner).read_bytes())

    def report(self):
        return funded_tests.FundedMintTransitionTests.report(
            self, keccak_test_extension=self.keccak_extension,
            aligned_test_extension=self.aligned_extension,
            encoding_extension=self.encoding_extension,
            negation_test_extension=self.negation_extension,
            value_test_extension=self.value_extension,
            unit_composition_transition=self.unit_composition_transition,
            optional_write_transition=self.optional_write_transition,
            enum_advance_transition=self.enum_advance_transition,
            unsigned_product_transition=self.unsigned_product_transition,
            organizer_transition=self.organizer_transition,
            witness_merkle_transition=self.witness_merkle_transition,
            opaque_set_transition=self.opaque_set_transition,
            unit_interface_extension=self.unit_interface_extension,
            map_value_extension=self.map_value_extension,
            guarded_map_transition=self.guarded_map_transition,
            opaque_map_transition=self.opaque_map_transition,
            resource_diagnostics_extension=self.resource_diagnostics_extension,
            empty_recording_extension=self.empty_recording_extension,
            guarded_pure_transition=self.guarded_pure_transition,
            reachable_helpers_extension=self.reachable_helpers_extension,
            key_only_map_removal_extension=self.key_only_map_removal_extension,
            asset_removal_transition=self.asset_removal_transition,
            opaque_boolean_transition=self.opaque_boolean_transition,
            opaque_field_transition=self.opaque_field_transition,
            proved_set_sequence_transition=self.proved_set_sequence_transition,
            ordinary_interface_extension=self.ordinary_interface_extension,
            storage_codec_extension=self.storage_codec_extension,
            list_effects_extension=self.list_effects_extension,
            map_effects_extension=self.map_effects_extension,
            persistent_commit_extension=self.persistent_commit_extension,
            vector_map_extension=self.vector_map_extension,
            audited_local_transition=self.audited_local_transition)


    def test_single_retirement_preserves_exact_predecessor(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 35)
        self.assertEqual(report['retired_routes'][-1]['id'], 'early.audited-local')
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertNotIn('early.audited-local', ids)
        self.assertEqual(ids.count('early.read-only-verification'), 1)
        self.assertEqual(ids.count('algebra.unit-composition'), 1)
        self.assertEqual(ids.count('algebra.ordinary'), 1)
        self.assertFalse(report['additional_current_routes'][0]['retirement_counted'])
        for owner, source in interface.predecessor_sources(self.root, self.audited_local_transition).items():
            self.assertEqual(inventory.digest(source.encode()), self.audited_local_transition['owners'][owner]['previous_sha256'])

    def test_missing_forged_transition_and_prior_authorities_fail(self):
        for field in ['audited_local_transition', 'vector_map_extension', 'persistent_commit_extension', 'proved_set_sequence_transition', 'unit_composition_transition']:
            original = getattr(self, field)
            for value in [None, {**original, 'claim': 'forged'}]:
                setattr(self, field, value)
                self.assertFalse(self.report()['passed'], field)
            setattr(self, field, original)

    def test_all_live_owners_and_dependencies_reject_drift(self):
        for owner in [*self.audited_local_transition['owners'], *self.audited_local_transition['new_and_shared_sources']]:
            path = self.root / owner
            original = path.read_text()
            path.write_text(original+'\n// unreviewed drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_partial_oracle_annotation_and_readonly_drift_fail(self):
        path = self.root / interface.AUDITED_LOCAL
        original = path.read_text()
        for marker in ['#[cfg(test)]\nfn uncoerced<', '#[cfg(test)]\npub(super) fn render(']:
            self.assertIn(marker, original)
            path.write_text(original.replace(marker, marker.removeprefix('#[cfg(test)]\n'), 1))
            self.assertFalse(self.report()['passed'])
        path.write_text(original)
        path = self.root / interface.OWNER
        original = path.read_text()
        path.write_text(original.replace('read_only_verification::render(', 'audited_local::render(', 1))
        self.assertFalse(self.report()['passed'])

    def test_rehash_and_forged_projection_cannot_authorize_drift(self):
        owner = interface.OWNER
        path = self.root / owner
        path.write_text(path.read_text()+'\n// drift\n')
        self.audited_local_transition['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            interface._Checkpoint(self.root, {owner: 'forged'}, self.audited_local_transition)

    def test_unrelated_production_is_not_hidden(self):
        path = self.root / 'tools/compact-rust-backend/src/recorded/admission_probe.rs'
        path.write_text(path.read_text()+'\n// unreviewed drift\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_transition(self):
        path = previous.AUDITED_LOCAL_TRANSITION
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--audited-local-transition', str(path), '--output', str(path)])
        self.assertEqual(path.read_bytes(), before)

if __name__ == '__main__':
    unittest.main()
