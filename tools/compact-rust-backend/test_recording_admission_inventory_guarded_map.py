# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Guarded Map-read retirement authenticates whole helper authority and retained siblings."""
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_guarded_map as guarded_map
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_map_values as interface_tests
import test_recording_admission_inventory_funded_mint as funded_tests

class GuardedMapTransitionTests(unittest.TestCase):
    def setUp(self):
        interface_tests.MapValueExtensionTests.setUp(self)
        self.guarded_map_transition = json.loads(previous.GUARDED_MAP_TRANSITION.read_text())
        for owner in [*self.guarded_map_transition['owners'], *self.guarded_map_transition['new_and_shared_sources']]:
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
            guarded_map_transition=self.guarded_map_transition)

    def test_live_final_retirement_and_exact_historical_authority(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([x['route'] for x in report['reviewed_anchor_adaptations']], ['final.shielded-gap-precedence'])
        self.assertEqual(report['current_selector']['ordered_calls'], [])
        self.assertEqual(len(report['retired_routes']), 27)
        self.assertEqual(report['retired_routes'][-1]['id'], guarded_map.ROUTE)
        self.assertEqual(len(report['historical_predecessor']['retired_routes']), 26)
        self.assertEqual(report['historical_predecessor']['current_selector']['ordered_calls'], [])
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertNotIn(guarded_map.ROUTE, ids)
        for route in guarded_map.REPLACEMENTS:
            self.assertEqual(ids.count(route), 1)
        for owner, source in guarded_map.predecessor_sources(self.root, self.guarded_map_transition).items():
            row = self.guarded_map_transition['owners'][owner]
            self.assertEqual(inventory.digest(source.encode()), row.get('historical_sha256',row['previous_sha256']))

    def test_missing_forged_transition_and_prior_authority_fail_closed(self):
        for field in ['guarded_map_transition','map_value_extension','unit_interface_extension','opaque_set_transition','unsigned_product_transition','organizer_transition','witness_merkle_transition','enum_advance_transition','optional_write_transition','unit_composition_transition','encoding_extension']:
            original = getattr(self, field)
            for value in [None, {**original,'claim':'forged'}]:
                setattr(self,field,value)
                self.assertFalse(self.report()['passed'], field)
            setattr(self,field,original)

    def test_every_actual_owner_and_new_dependency_is_authenticated(self):
        for owner in [*self.guarded_map_transition['owners'], *self.guarded_map_transition['new_and_shared_sources']]:
            path = self.root / owner
            original = path.read_text()
            path.write_text(original+'\n// drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_historical_module_cfg_and_sibling_drift_fail(self):
        path = self.root / guarded_map.LEGACY
        original = path.read_text()
        marker = '#[cfg(test)]\npub(super) mod guarded_map;'
        self.assertIn(marker, original)
        path.write_text(original.replace(marker, 'pub(super) mod guarded_map;', 1))
        self.assertFalse(self.report()['passed'])
        path.write_text(original)
        path = self.root / inventory.RECORDED
        original = path.read_text()
        self.assertIn('"guard.opaque-map-write"', original)
        path.write_text(original.replace('"guard.opaque-map-write"', '"guard.changed-map-write"', 1))
        self.assertFalse(self.report()['passed'])

    def test_rehashing_cannot_authorize_production_drift_or_an_overlay(self):
        owner = inventory.RECORDED
        path = self.root / owner
        path.write_text(path.read_text()+'\n// drift\n')
        self.guarded_map_transition['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            guarded_map._Checkpoint(self.root,{owner:'forged'},self.guarded_map_transition)

    def test_other_live_authority_is_not_hidden_by_predecessor_projection(self):
        path = self.root / 'tools/compact-rust-backend/src/recorded/admission_probe.rs'
        path.write_text(path.read_text()+'\n// unrelated production drift\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_transition(self):
        path = previous.GUARDED_MAP_TRANSITION
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--guarded-map-transition',str(path),'--output',str(path)])
        self.assertEqual(path.read_bytes(),before)

if __name__ == '__main__':
    unittest.main()
