# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Nonretiring source authority with in-memory drift controls; no tree copying."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_shared_pure as shared_pure
import recording_admission_inventory_generic_qualification as qualification
import recording_admission_inventory_generic_retirement as retirement
import recording_admission_inventory_helper_discovery as helpers
import recording_admission_inventory_native_tuple as native_tuple

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.generic-qualification-619.json')


class _OverlayFile:
    def __init__(self, root, owner):
        self.root, self.owner = root, owner

    def read_bytes(self):
        if self.owner in self.root.changes:
            return self.root.changes[self.owner]
        return (self.root.base / self.owner).read_bytes()

    def read_text(self):
        return self.read_bytes().decode()


class _Overlay:
    """Each mutation changes one read; the real checkout remains untouched."""
    def __init__(self, base):
        self.base, self.changes = base, {}

    def __truediv__(self, owner):
        return _OverlayFile(self, owner)


class GenericQualificationTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        newest = inventory.BASELINE.with_name('recording_admission_inventory.helper-discovery-620.json')
        current_root = shared_pure.latest_checkpoint(inventory.ROOT, inventory.BASELINE)
        current_root = native_tuple.latest_checkpoint(current_root, inventory.BASELINE)
        latest_root = (helpers.checkpoint(current_root, json.loads(newest.read_text()))
                       if newest.exists() else current_root)
        outer = inventory.BASELINE.with_name('recording_admission_inventory.generic-retirement-618.json')
        # Historical qualification laws explicitly consume the authenticated
        # signed #619 snapshot; the outer gate checks all live retirement edits.
        self.source_root = (retirement.checkpoint(latest_root, json.loads(outer.read_text()))
                            if outer.exists() else latest_root)
        self.root = _Overlay(self.source_root)
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.baseline_bytes)
        self.previous = {
            key: json.loads((inventory.ROOT / path).read_text())
            for key, path in self.extension['predecessor_options'].items()
        }

    def report(self, **overrides):
        arguments = dict(self.previous, generic_qualification_extension=self.extension)
        arguments.update(overrides)
        # Reviewed transitions authenticate complete changed owners; the original
        # baseline's global strict mode predates those intentional source changes.
        return inventory.verify(self.root, self.baseline,
                                baseline_bytes=self.baseline_bytes, **arguments)

    def test_exact_predecessor_preserves_all_route_accounting(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        prior = inventory.verify(qualification.checkpoint(self.root, self.extension),
                                 self.baseline,
                                 baseline_bytes=self.baseline_bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 37)
        self.assertEqual(report['retired_routes'], prior['retired_routes'])
        self.assertEqual(report['reviewed_instrumentation'], prior['reviewed_instrumentation'])
        self.assertEqual(report['reviewed_generic_qualification']['new_retirements'], 0)

    def test_every_current_owner_and_new_source_rejects_drift(self):
        for owner in [*self.extension['owners'], *self.extension['new_and_shared_sources']]:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n// drift\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_missing_forged_and_historical_authorities_fail(self):
        self.assertFalse(self.report(generic_qualification_extension=None)['passed'])
        forged = {**self.extension, 'claim': 'unreviewed'}
        self.assertFalse(self.report(generic_qualification_extension=forged)['passed'])
        self.assertFalse(self.report(start_funding_transition=None)['passed'])

    def test_partial_inverse_and_false_retirement_cannot_be_rehashed(self):
        for mutate in ['inverse', 'retirement']:
            forged = copy.deepcopy(self.extension)
            if mutate == 'inverse':
                owner = next(iter(forged['owners']))
                forged['owners'][owner]['predecessor_reconstruction'][0]['previous'] += '\n'
            else:
                forged['retired_route_ids'] = ['return.dispatch']
            self.assertFalse(self.report(generic_qualification_extension=forged)['passed'])

    def test_checker_role_and_test_only_registration_mutations_fail(self):
        mutations = [
            (qualification.BASE + '.rs', '#[cfg(test)]\nmod generic_reference;', 'mod generic_reference;'),
            (qualification.SELECTOR, 'match select_checked(', 'match effect_plan::lower_with_calls_diagnostic('),
            (qualification.BASE + '/effect_plan/lower.rs', 'if !root_interface_type(&p.ty, circuit.internal)', 'if !execution_result_type(&p.ty)'),
            (qualification.BASE + '/effect_plan/validate.rs', 'if !root_interface_type(&value.ty, internal)', 'if !execution_result_type(&value.ty)'),
        ]
        for owner, before, after in mutations:
            original = (self.source_root / owner).read_text()
            self.assertEqual(original.count(before), 1)
            self.root.changes[owner] = original.replace(before, after, 1).encode()
            self.assertFalse(self.report()['passed'])
            self.root.changes.clear()

    def test_unrelated_production_drift_is_not_hidden(self):
        owner = qualification.BASE + '/helpers.rs'
        self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n// drift\n'
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_the_transition(self):
        before = MANIFEST.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--generic-qualification-extension', str(MANIFEST),
                            '--output', str(MANIFEST)])
        self.assertEqual(MANIFEST.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
