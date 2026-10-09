#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Focused inventory gate tests; no Rust compilation or third-party modules."""
import contextlib
import copy
import io
import json
from pathlib import Path
import tempfile
import unittest

import recording_admission_inventory as inventory


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.selector = self.root / inventory.SELECTOR
        self.selector.parent.mkdir(parents=True)
        self.selector.write_text('pub(super) fn select() {\n typed_plan::lower();\n kernel_plan::lower();\n}\n')
        self.other = self.root / 'owner.rs'
        self.other.write_text('fn admission() {}\n')
        self.baseline = {
            'schema_version': 1, 'baseline_commit': 'a' * 40,
            'main_selector': inventory.selector_signature(self.selector.read_text()),
            'source_fingerprints': {
                inventory.SELECTOR: inventory.digest(self.selector.read_bytes()),
                'owner.rs': inventory.digest(self.other.read_bytes()),
            },
            'routes': [{
                'id': 'example.admission', 'baseline_status': 'production-active',
                'disposition': 'split-semantic-from-shape', 'planned_packages': ['EA-03'],
                'anchors': [{'source': 'owner.rs', 'text': 'fn admission(', 'baseline_occurrences': 1}],
            }],
        }

    def test_unchanged_baseline_passes_strictly(self):
        report = inventory.verify(self.root, self.baseline, True)
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['source_changes'], [])

    def test_anchor_removal_fails(self):
        self.other.write_text('fn replacement() {}\n')
        report = inventory.verify(self.root, self.baseline)
        self.assertFalse(report['passed'])
        self.assertEqual(report['route_checks'][0]['failures'][0]['actual'], 0)

    def test_anchor_duplication_fails(self):
        self.other.write_text(self.other.read_text() * 2)
        self.assertFalse(inventory.verify(self.root, self.baseline)['passed'])

    def test_new_selector_call_fails(self):
        self.selector.write_text(self.selector.read_text().replace(' kernel_plan::lower();', ' typed_plan::lower_new_family();\n kernel_plan::lower();'))
        report = inventory.verify(self.root, self.baseline)
        self.assertFalse(report['passed'])
        self.assertIn('typed_plan::lower_new_family', report['current_selector']['ordered_calls'])

    def test_selector_order_change_fails(self):
        self.selector.write_text('pub(super) fn select() {\n kernel_plan::lower();\n typed_plan::lower();\n}\n')
        self.assertFalse(inventory.verify(self.root, self.baseline)['passed'])

    def test_selector_unrecognized_gate_still_fails_digest(self):
        self.selector.write_text(self.selector.read_text().replace(' typed_plan::lower();', ' if new_gate() { return; }\n typed_plan::lower();'))
        report = inventory.verify(self.root, self.baseline)
        self.assertFalse(report['passed'])
        self.assertEqual(report['current_selector']['ordered_calls'], self.baseline['main_selector']['ordered_calls'])

    def test_other_source_drift_is_visible_and_optionally_fatal(self):
        self.other.write_text(self.other.read_text() + '#[cfg(test)] mod tests;\n')
        report = inventory.verify(self.root, self.baseline)
        self.assertTrue(report['passed'])
        self.assertEqual([r['source'] for r in report['source_changes']], ['owner.rs'])
        self.assertFalse(inventory.verify(self.root, self.baseline, True)['passed'])

    def test_missing_owner_fails(self):
        self.other.unlink()
        self.assertFalse(inventory.verify(self.root, self.baseline)['passed'])

    def test_invalid_metadata_fails_before_source_reads(self):
        mutations = [
            lambda b: b['routes'].append(copy.deepcopy(b['routes'][0])),
            lambda b: b['routes'][0].update(planned_packages=['EA-99']),
            lambda b: b['routes'][0].update(disposition='claim-parity'),
            lambda b: b['routes'][0].update(baseline_status='retired'),
            lambda b: b['source_fingerprints'].update({'../escape.rs': '0' * 64}),
            lambda b: b['source_fingerprints'].update({'owner.rs': 'bad-hash'}),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                baseline = copy.deepcopy(self.baseline)
                mutate(baseline)
                self.assertFalse(inventory.verify(self.root, baseline)['passed'])

    def test_cli_emits_current_report_without_modifying_baseline(self):
        baseline = self.root / 'baseline.json'
        baseline.write_text(json.dumps(self.baseline))
        before = baseline.read_bytes()
        output = self.root / 'current.json'
        self.assertEqual(inventory.main(['--root', str(self.root), '--baseline', str(baseline), '--output', str(output)]), 0)
        self.assertEqual(baseline.read_bytes(), before)
        self.assertTrue(json.loads(output.read_text())['passed'])
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--root', str(self.root), '--baseline', str(baseline), '--output', str(baseline)])
        self.assertEqual(baseline.read_bytes(), before)

    def test_checked_in_inventory_resolves_current_routes(self):
        baseline = json.loads(inventory.BASELINE.read_text())
        report = inventory.verify(inventory.ROOT, baseline)
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(len(baseline['main_selector']['ordered_calls']), 19)
        self.assertEqual(sum(row['family'] == 'eager-organizer-selection' for row in baseline['routes']), 4)
        self.assertEqual(sum(row['family'] == 'early-complete-renderer' for row in baseline['routes']), 2)


if __name__ == '__main__':
    unittest.main()
