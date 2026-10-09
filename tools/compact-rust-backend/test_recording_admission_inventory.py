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
        baseline_bytes = inventory.BASELINE.read_bytes()
        baseline = json.loads(baseline_bytes)
        transition = json.loads(inventory.BASELINE.with_name(
            'recording_admission_inventory.transition-511.json').read_text())
        report = inventory.verify(inventory.ROOT, baseline,
                                  transition=transition, baseline_bytes=baseline_bytes)
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(len(baseline['main_selector']['ordered_calls']), 19)
        self.assertEqual(sum(row['family'] == 'eager-organizer-selection' for row in baseline['routes']), 4)
        self.assertEqual(sum(row['family'] == 'early-complete-renderer' for row in baseline['routes']), 2)


class RetirementTransitionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.baseline_bytes)
        self.transition = json.loads(inventory.BASELINE.with_name(
            'recording_admission_inventory.transition-511.json').read_text())
        for relative in [*self.baseline['source_fingerprints'], inventory.ORACLE]:
            destination = self.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((inventory.ROOT / relative).read_bytes())

    def report(self):
        return inventory.verify(self.root, self.baseline, transition=self.transition,
                                baseline_bytes=self.baseline_bytes)

    def test_reviewed_retirement_keeps_frozen_denominator(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], ['effectful.effectful'])
        self.assertTrue(report['retired_routes'][0]['passed'])
        # Retirement is never implicit: the untransitioned baseline must flag it.
        self.assertFalse(inventory.verify(self.root, self.baseline)['passed'])

    def test_call_reintroduction_fails_even_with_whitespace(self):
        path = self.root / inventory.RECORDED
        path.write_text(path.read_text() + '\nfn accidental() { typed_plan :: lower_effectful (); }\n')
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertIn('retirement: recorded.rs reintroduced a lower_effectful call', report['errors'])

    def test_stripped_test_cfg_fails_for_module_and_reexport(self):
        path = self.root / inventory.TYPED_PLAN
        original = path.read_text()
        for declaration in ('mod effectful_reference;', 'pub(super) use effectful_reference::lower_effectful;'):
            with self.subTest(declaration=declaration):
                path.write_text(original.replace('#[cfg(test)]\n' + declaration, declaration))
                self.assertFalse(self.report()['passed'])
        path.write_text(original)

    def test_production_lowerer_definition_cannot_return(self):
        path = self.root / inventory.TYPED_PLAN
        path.write_text(path.read_text() + '\nfn lower_effectful() {}\n')
        self.assertFalse(self.report()['passed'])

    def test_altered_test_oracle_fails(self):
        path = self.root / inventory.ORACLE
        path.write_text(path.read_text().replace('branches == 0', 'branches == 99'))
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertIn('retirement: test oracle differs from frozen original body', report['errors'])

    def test_changed_baseline_bytes_or_decoded_content_fail(self):
        self.baseline_bytes += b'\n'
        self.assertFalse(self.report()['passed'])
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline['claim'] = 'edited baseline'
        self.assertFalse(self.report()['passed'])

    def test_original_source_fingerprint_binding_is_required(self):
        self.transition['original_source_sha256'] = '0' * 64
        self.assertFalse(self.report()['passed'])

    def test_unrelated_anchor_and_selector_still_fail(self):
        path = self.root / inventory.RECORDED
        path.write_text(path.read_text().replace('closed_opaque_asset_removal', 'removed_asset_gate'))
        self.assertFalse(self.report()['passed'])
        path.write_bytes((inventory.ROOT / inventory.RECORDED).read_bytes())
        selector = self.root / inventory.SELECTOR
        selector.write_text(selector.read_text() + '\nfn new_unreviewed_gate() {}\n')
        self.assertFalse(self.report()['passed'])

    def test_transition_cannot_retire_another_route(self):
        self.transition['retired_route'] = 'selector.zswap'
        self.assertFalse(self.report()['passed'])


if __name__ == '__main__':
    unittest.main()
