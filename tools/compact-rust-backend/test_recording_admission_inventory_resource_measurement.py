# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Test-only probe authority, exact inverse and observable staged-view drift."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import recording_admission_inventory as inventory
import recording_admission_inventory_resource_measurement as measurement
from recording_admission_inventory_resource_measurement_test_support import historical_suite_reads
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.resource-measurement-622d5.json')


class ResourceMeasurementTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(inventory.ROOT)
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {key: json.loads((inventory.ROOT / path).read_text())
                         for key, path in self.extension['predecessor_options'].items()}

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'resource_measurement_extension': self.extension,
                                   **overrides})

    def test_actual_D4c_required_and_recording_accounting_is_unchanged(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(measurement.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in prior:
            self.assertEqual(current[key], prior[key])
        stage = current['reviewed_resource_measurement_extension']
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (79, 49))
        self.assertEqual(stage['new_retired_routes'], [])
        self.assertIn('production policy unchanged', stage['qualification'])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        self.assertFalse(self.report(body_checker_extension=None)['passed'])
        malformed = measurement.verify_extension(
            self.root, self.baseline, self.extension,
            lambda _: {**prior, 'reviewed_body_checker_extension': {}})
        self.assertFalse(malformed['passed'])
        self.assertIn('mandatory actual D4c', malformed['errors'][0])

    def test_forged_missing_or_partial_extension_never_projects(self):
        for change in (
                lambda e: e['owners'].clear(),
                lambda e: e['new_sources'].clear(),
                lambda e: e['owners'][measurement.CHECKER]['test_adaptations'].clear(),
                lambda e: e['historical_manifests'].pop(measurement.PARENT),
                lambda e: e.update(new_retired_route_ids=['forged'])):
            forged = copy.deepcopy(self.extension)
            change(forged)
            with self.subTest(extension=forged):
                with self.assertRaisesRegex(ValueError, 'unreviewed or incomplete'):
                    measurement.checkpoint(self.root, forged)
                self.assertFalse(self.report(resource_measurement_extension=forged)['passed'])

    def test_actual_checker_probe_and_all_prior_source_drift_refuse(self):
        paths = {measurement.CHECKER, *self.extension['new_sources'],
                 *self.extension['new_and_shared_sources'],
                 *self.extension['historical_manifests']}
        for owner in sorted(paths):
            with self.subTest(owner=owner):
                self.root.changes[owner] = (inventory.ROOT / owner).read_bytes() + b'\n'
                with self.assertRaises(ValueError):
                    measurement.checkpoint(self.root, self.extension)
                self.root.changes.clear()
        # Neither a missing registration nor a second registration is repaired.
        source = (inventory.ROOT / measurement.CHECKER).read_text()
        for changed in (source.replace(measurement.REGISTRATION, '', 1),
                        source + measurement.REGISTRATION):
            self.root.changes[measurement.CHECKER] = changed.encode()
            with self.assertRaisesRegex(ValueError, 'current test owner changed'):
                measurement.checkpoint(self.root, self.extension)
            self.root.changes.clear()

    def test_inverse_and_historical_read_view_are_exact_without_writing(self):
        actual = (inventory.ROOT / measurement.CHECKER).read_bytes()
        probe = (inventory.ROOT / measurement.PROBE).read_bytes()
        previous = measurement.predecessor_sources(self.root, self.extension)[measurement.CHECKER]
        self.assertEqual(previous, actual.decode().replace(measurement.REGISTRATION, '', 1))
        with historical_suite_reads(inventory.ROOT, self.extension):
            self.assertEqual((inventory.ROOT / measurement.CHECKER).read_text(), previous)
            self.assertEqual((inventory.ROOT / measurement.PROBE).read_bytes(), probe)
            mutated = _Overlay(inventory.ROOT)
            mutated.changes[measurement.CHECKER] = (previous + '\n').encode()
            with self.assertRaisesRegex(ValueError, 'new_and_shared_sources changed'):
                inventory.body_checker.predecessor_sources(
                    mutated, self.previous['body_checker_extension'])
        self.assertEqual((inventory.ROOT / measurement.CHECKER).read_bytes(), actual)
        self.assertEqual((inventory.ROOT / measurement.PROBE).read_bytes(), probe)

    def test_actual_drift_during_historical_suite_is_observable(self):
        original = Path.read_bytes
        changes = {}

        def actual_read(path):
            return changes.get(path.resolve(), original(path))

        with mock.patch.object(Path, 'read_bytes', actual_read):
            with historical_suite_reads(inventory.ROOT, self.extension):
                for owner, diagnostic in ((measurement.CHECKER, 'actual checker drift'),
                                          (measurement.PROBE, 'actual probe drift')):
                    target = (inventory.ROOT / owner).resolve()
                    changes[target] = original(target) + b'\n'
                    with self.assertRaisesRegex(ValueError, diagnostic):
                        (inventory.ROOT / measurement.CHECKER).read_bytes()
                    changes.clear()

    def test_cli_protects_current_probe_inverse_and_transitive_history_inputs(self):
        args = ['--resource-measurement-extension', str(MANIFEST)]
        for key, path in self.extension['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        parent = self.previous['body_checker_extension']
        targets = {str(MANIFEST), measurement.CHECKER, measurement.PROBE,
                   measurement.PARENT, inventory.body_checker.RETIRED,
                   next(iter(parent['oracle_authorities']))}
        for target in sorted(targets):
            path = Path(target) if Path(target).is_absolute() else inventory.ROOT / target
            with self.subTest(path=path), contextlib.redirect_stderr(io.StringIO()) as stderr:
                with self.assertRaises(SystemExit) as error:
                    inventory.main([*args, '--output', str(path)])
                self.assertEqual(error.exception.code, 2)
                self.assertIn('cannot overwrite', stderr.getvalue())


if __name__ == '__main__':
    unittest.main()
