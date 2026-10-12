# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Optional-write retirement authenticates old authority and actual eager siblings."""
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_optional_write as optional
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_unit_composition as unit_tests
import test_recording_admission_inventory_funded_mint as funded_tests

class OptionalWriteTransitionTests(unittest.TestCase):
    def setUp(self):
        unit_tests.UnitCompositionTransitionTests.setUp(self)
        self.optional_write_transition = json.loads(previous.OPTIONAL_WRITE_TRANSITION.read_text())
        for owner in [*self.optional_write_transition['owners'], *self.optional_write_transition['new_and_shared_sources']]:
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
            optional_write_transition=self.optional_write_transition)

    def test_live_final_retirement_and_exact_historical_authority(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(report['current_selector']['ordered_calls'], [])
        self.assertEqual(len(report['retired_routes']), 21)
        self.assertEqual(report['retired_routes'][-1]['id'], optional.ROUTE)
        self.assertEqual(len(report['historical_predecessor']['retired_routes']), 20)
        self.assertEqual(report['historical_predecessor']['current_selector']['ordered_calls'], [])
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertNotIn(optional.ROUTE, ids)
        self.assertEqual(ids.count(optional.REPLACEMENT), 1)
        for owner, source in optional.predecessor_sources(self.root, self.optional_write_transition).items():
            row = self.optional_write_transition['owners'][owner]
            self.assertEqual(inventory.digest(source.encode()), row.get('historical_sha256',row['previous_sha256']))

    def test_missing_forged_transition_and_prior_authority_fail_closed(self):
        for field in ['optional_write_transition','unit_composition_transition','value_extension','encoding_extension']:
            original = getattr(self, field)
            for value in [None, {**original,'claim':'forged'}]:
                setattr(self,field,value)
                self.assertFalse(self.report()['passed'], field)
            setattr(self,field,original)

    def test_every_actual_owner_and_new_dependency_is_authenticated(self):
        for owner in [*self.optional_write_transition['owners'], *self.optional_write_transition['new_and_shared_sources']]:
            path = self.root / owner
            original = path.read_text()
            path.write_text(original+'\n// drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_oracle_cfg_and_eager_sibling_drift_fail(self):
        path = self.root / optional.GUARDED
        original = path.read_text()
        marker = '#[cfg(test)]\npub(super) fn closed_authorized_optional_write_steps('
        path.write_text(original.replace(marker, marker.replace('#[cfg(test)]\n', ''), 1))
        self.assertFalse(self.report()['passed'])
        path.write_text(original)
        path = self.root / inventory.SELECTOR
        original = path.read_text()
        self.assertIn('.or(record_option!(', original)
        path.write_text(original.replace('.or(record_option!(', '.or_else(|| record_option!(', 1))
        self.assertFalse(self.report()['passed'])

    def test_rehashing_cannot_authorize_production_drift_or_an_overlay(self):
        owner = inventory.SELECTOR
        path = self.root / owner
        path.write_text(path.read_text()+'\n// drift\n')
        self.optional_write_transition['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            optional._Checkpoint(self.root,{owner:'forged'},self.optional_write_transition)

    def test_other_live_authority_is_not_hidden_by_predecessor_projection(self):
        path = self.root / 'tools/compact-rust-backend/src/recorded/admission_probe.rs'
        path.write_text(path.read_text()+'\n// unrelated production drift\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_transition(self):
        path = previous.OPTIONAL_WRITE_TRANSITION
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--optional-write-transition',str(path),'--output',str(path)])
        self.assertEqual(path.read_bytes(),before)

if __name__ == '__main__':
    unittest.main()
