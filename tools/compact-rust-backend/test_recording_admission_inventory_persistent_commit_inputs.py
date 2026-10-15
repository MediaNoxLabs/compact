# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Fixed-aligned PersistentCommit inputs authenticate exact predecessor and operation authority."""
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_persistent_commit_inputs as interface
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_map_effects as interface_tests
import test_recording_admission_inventory_funded_mint as funded_tests


class PersistentCommitInputExtensionTests(unittest.TestCase):
    def setUp(self):
        interface_tests.MapEffectsExtensionTests.setUp(self)
        self.persistent_commit_extension = json.loads(previous.PERSISTENT_COMMIT_EXTENSION.read_text())
        for owner in [*self.persistent_commit_extension['owners'], *self.persistent_commit_extension['new_and_shared_sources']]:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((previous.checkpoint_611(inventory.ROOT) / owner).read_bytes())

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
            persistent_commit_extension=self.persistent_commit_extension)

    def test_effect_extension_preserves_retirement_and_exact_predecessor(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 34)
        self.assertEqual(report['current_selector']['ordered_calls'], [])
        self.assertEqual(report['reviewed_instrumentation']['instrumented_route_ids'].count('algebra.ordinary'), 1)
        self.assertEqual(report['additional_current_routes'][0]['id'], 'algebra.ordinary')
        self.assertFalse(report['additional_current_routes'][0]['retirement_counted'])
        for owner, source in interface.predecessor_sources(self.root, self.persistent_commit_extension).items():
            self.assertEqual(inventory.digest(source.encode()), self.persistent_commit_extension['owners'][owner]['previous_sha256'])
        self.assertEqual(report['reviewed_instrumentation']['instrumented_route_ids'].count('algebra.unit-composition'), 1)

    def test_missing_forged_extension_and_previous_authorities_fail_closed(self):
        for field in ['persistent_commit_extension','map_effects_extension','list_effects_extension','storage_codec_extension','ordinary_interface_extension','proved_set_sequence_transition','opaque_field_transition','opaque_boolean_transition','asset_removal_transition','key_only_map_removal_extension','reachable_helpers_extension','guarded_pure_transition','empty_recording_extension','resource_diagnostics_extension','opaque_map_transition','guarded_map_transition','map_value_extension','unit_interface_extension','unit_composition_transition']:
            original = getattr(self, field)
            for value in [None, {**original, 'claim':'forged'}]:
                setattr(self, field, value)
                self.assertFalse(self.report()['passed'], field)
            setattr(self, field, original)
    def test_all_current_owners_and_checked_dependencies_remain_live(self):
        for owner in [*self.persistent_commit_extension['owners'], *self.persistent_commit_extension['new_and_shared_sources']]:
            path = self.root / owner
            original = path.read_text()
            path.write_text(original+'\n// unreviewed drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_effect_and_checked_authority_stay_live(self):
        path = self.root / 'tools/compact-rust-backend/src/recorded/effect_plan/validate/fab.rs'
        original = path.read_text()
        marker = 'supports(ty, Capability::FixedAligned)'
        self.assertIn(marker, original)
        path.write_text(original.replace(marker, 'supports(ty, Capability::Aligned)', 1))
        self.assertFalse(self.report()['passed'])
        path.write_text(original)
        path = self.root / 'tools/compact-rust-backend/src/recorded/legacy/actions/assertions.rs'
        original = path.read_text()
        path.write_text(original+'\n// unreviewed witness action authority change\n')
        self.assertFalse(self.report()['passed'])

    def test_rehash_or_forged_projection_cannot_authorize_drift(self):
        owner = interface.OWNER
        path = self.root / owner
        path.write_text(path.read_text()+'\n// drift\n')
        self.persistent_commit_extension['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            interface._Checkpoint(self.root, {owner:'forged'}, self.persistent_commit_extension)

    def test_unrelated_live_production_is_not_hidden(self):
        path = self.root / 'tools/compact-rust-backend/src/recorded/admission_probe.rs'
        path.write_text(path.read_text()+'\n// drift outside reconstructed owners\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_extension(self):
        path = previous.PERSISTENT_COMMIT_EXTENSION
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--persistent-commit-extension',str(path),'--output',str(path)])
        self.assertEqual(path.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
