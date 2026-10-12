# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Final Unit retirement preserves historical authority and checks live routes."""
import contextlib
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_unit_composition as unit
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_value_test as value_tests
import test_recording_admission_inventory_funded_mint as funded_tests

class UnitCompositionTransitionTests(unittest.TestCase):
    def setUp(self):
        value_tests.ValueTestExtensionTests.setUp(self)
        self.unit_composition_transition = json.loads(previous.UNIT_COMPOSITION_TRANSITION.read_text())
        for owner in [*self.unit_composition_transition['owners'], *self.unit_composition_transition['new_and_shared_sources']]:
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
            unit_composition_transition=self.unit_composition_transition)

    def test_live_final_retirement_and_exact_historical_authority(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(report['current_selector']['ordered_calls'], [])
        self.assertEqual(len(report['retired_routes']), 20)
        self.assertEqual(report['retired_routes'][-1]['id'], unit.ROUTE)
        self.assertEqual(len(report['historical_predecessor']['retired_routes']), 19)
        self.assertEqual(report['historical_predecessor']['current_selector']['ordered_calls'], ['typed_plan::lower_unit_composition_checked'])
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertNotIn(unit.ROUTE, ids)
        self.assertEqual(ids.count(unit.REPLACEMENT), 1)
        for owner, source in unit.predecessor_sources(self.root, self.unit_composition_transition).items():
            row = self.unit_composition_transition['owners'][owner]
            self.assertEqual(inventory.digest(source.encode()), row.get('historical_sha256',row['previous_sha256']))

    def test_missing_forged_transition_and_prior_authority_fail_closed(self):
        for field in ['unit_composition_transition','value_extension','funded_transition','encoding_extension']:
            original = getattr(self, field)
            for value in [None, {**original,'claim':'forged'}]:
                setattr(self,field,value)
                self.assertFalse(self.report()['passed'], field)
            setattr(self,field,original)

    def test_every_actual_owner_and_new_dependency_is_authenticated(self):
        for owner in [*self.unit_composition_transition['owners'], *self.unit_composition_transition['new_and_shared_sources']]:
            path = self.root / owner
            original = path.read_text()
            path.write_text(original+'\n// drift\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_oracle_cfg_removal_and_partial_rollback_fail(self):
        for owner, markers in self.unit_composition_transition['test_only_markers'].items():
            path = self.root / owner
            original = path.read_text()
            for marker in markers:
                path.write_text(original.replace('#[cfg(test)]\n'+marker,marker,1))
                self.assertFalse(self.report()['passed'], marker)
                path.write_text(original)
        owner = inventory.SELECTOR
        row = self.unit_composition_transition['owners'][owner]
        edit = row['predecessor_reconstruction'][0]
        path = self.root / owner
        path.write_text(path.read_text().replace(edit['current'],edit['previous'],1))
        self.assertFalse(self.report()['passed'])

    def test_rehashing_cannot_authorize_production_drift_or_an_overlay(self):
        owner = inventory.SELECTOR
        path = self.root / owner
        path.write_text(path.read_text()+'\n// drift\n')
        self.unit_composition_transition['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            unit._Checkpoint(self.root,{owner:'forged'},self.unit_composition_transition)

    def test_other_live_authority_is_not_hidden_by_predecessor_projection(self):
        path = self.root / 'tools/compact-rust-backend/src/recorded/admission_probe.rs'
        path.write_text(path.read_text()+'\n// unrelated production drift\n')
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_transition(self):
        path = previous.UNIT_COMPOSITION_TRANSITION
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--unit-composition-transition',str(path),'--output',str(path)])
        self.assertEqual(path.read_bytes(),before)

if __name__ == '__main__':
    unittest.main()
