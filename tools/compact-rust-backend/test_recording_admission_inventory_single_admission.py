# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Retire exactly one early renderer while preserving the authenticated old domain."""
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_single_admission as interface
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_read_only as prior_tests
import test_recording_admission_inventory_funded_mint as funded_tests

class SingleAdmissionTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.ReadOnlyTransitionTests.setUp(self)
        self.single_admission_transition = json.loads(previous.SINGLE_ADMISSION_TRANSITION.read_text())
        for owner in [*self.single_admission_transition['owners'], *self.single_admission_transition['new_and_shared_sources']]:
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
            audited_local_transition=self.audited_local_transition,
            read_only_transition=self.read_only_transition,
            single_admission_transition=self.single_admission_transition)


    def test_consolidation_preserves_exact_predecessor_and_counts(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 36)
        self.assertEqual(report['retired_routes'][-1]['id'], 'early.read-only-verification')
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertNotIn('early.read-only-verification', ids)
        self.assertNotIn('early.audited-local', ids)
        self.assertNotIn('algebra.unit-composition', ids)
        self.assertEqual(len(report['reviewed_single_admission']['consolidated_checked_route_ids']), 12)
        self.assertEqual(ids.count('algebra.ordinary'), 1)
        self.assertFalse(report['additional_current_routes'][0]['retirement_counted'])
        for owner, source in interface.predecessor_sources(self.root, self.single_admission_transition).items():
            self.assertEqual(inventory.digest(source.encode()), self.single_admission_transition['owners'][owner]['previous_sha256'])

    def test_missing_forged_transition_and_prior_authorities_fail(self):
        for field in ['single_admission_transition', 'read_only_transition', 'audited_local_transition', 'vector_map_extension', 'persistent_commit_extension', 'proved_set_sequence_transition', 'unit_composition_transition']:
            original = getattr(self, field)
            for value in [None, {**original, 'claim': 'forged'}]:
                setattr(self, field, value)
                self.assertFalse(self.report()['passed'], field)
            setattr(self, field, original)

    def test_all_live_owners_and_dependencies_reject_drift(self):
        for owner in [*self.single_admission_transition['owners'], *self.single_admission_transition['new_and_shared_sources']]:
            path = self.root / owner
            original = path.read_text()
            path.write_text(original+'\n// unreviewed drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_production_selector_and_projection_mutations_fail(self):
        path = self.root / interface.SELECTOR
        original = path.read_text()
        for old, new in [
            ('#[cfg(test)]\npub(super) fn legacy_select(', 'pub(super) fn legacy_select('),
            ('unit_composition_rejection = Some(rejection)', 'unit_composition_rejection = None'),
            ('Err(RenderError::RecordingPlanInvariant {', 'Ok(compatibility_rejection(circuit, rejection)); Err(RenderError::RecordingPlanInvariant {'),
            ('organizer_steps: Some(plan.steps)', 'organizer_steps: None'),
        ]:
            self.assertIn(old, original)
            path.write_text(original.replace(old, new, 1))
            self.assertFalse(self.report()['passed'])
        path.write_text(original)

    def test_rehash_and_forged_projection_cannot_authorize_drift(self):
        owner = interface.OWNER
        path = self.root / owner
        path.write_text(path.read_text()+'\n// drift\n')
        self.single_admission_transition['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            interface._Checkpoint(self.root, {owner: 'forged'}, self.single_admission_transition)

    def test_unrelated_production_is_not_hidden(self):
        path = self.root / 'tools/compact-rust-backend/src/recorded/admission_probe.rs'
        path.write_text(path.read_text()+'\n// unreviewed drift\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_transition(self):
        path = previous.SINGLE_ADMISSION_TRANSITION
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--single-admission-transition', str(path), '--output', str(path)])
        self.assertEqual(path.read_bytes(), before)

if __name__ == '__main__':
    unittest.main()
