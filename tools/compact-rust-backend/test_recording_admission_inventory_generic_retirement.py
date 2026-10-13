# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Whole-fallback source authority and mutation controls; no checkout copies."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock

import recording_admission_inventory as inventory
import recording_admission_inventory_shared_conversion as shared_conversion
from recording_admission_inventory_shared_conversion_test_support import install_historical_reads
import recording_admission_inventory_shared_leaf as shared_leaf
import recording_admission_inventory_unit_fragment as unit_fragment
import recording_admission_inventory_shared_coercion as shared_coercion
import recording_admission_inventory_target_edition as target_edition
import recording_admission_inventory_shared_pure as shared_pure
import recording_admission_inventory_generic_retirement as retirement
import recording_admission_inventory_helper_discovery as helpers
import recording_admission_inventory_native_tuple as native_tuple
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.generic-retirement-618.json')


class GenericRetirementTests(unittest.TestCase):
    def setUp(self):
        self.transition = json.loads(MANIFEST.read_text())
        newest = inventory.BASELINE.with_name('recording_admission_inventory.helper-discovery-620.json')
        latest_root = unit_fragment.latest_checkpoint(
            shared_leaf.latest_checkpoint(
            shared_conversion.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE), inventory.BASELINE)
        latest_root = shared_coercion.latest_checkpoint(latest_root, inventory.BASELINE)
        latest_root = target_edition.latest_checkpoint(latest_root, inventory.BASELINE)
        latest_root = shared_pure.latest_checkpoint(latest_root, inventory.BASELINE)
        latest_root = native_tuple.latest_checkpoint(latest_root, inventory.BASELINE)
        self.source_root = (helpers.checkpoint(latest_root, json.loads(newest.read_text()))
                            if newest.exists() else latest_root)
        self.root = _Overlay(self.source_root)
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.baseline_bytes)
        self.previous = {
            key: json.loads((inventory.ROOT / path).read_text())
            for key, path in self.transition['predecessor_options'].items()
        }
        install_historical_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        arguments = dict(self.previous, generic_retirement_transition=self.transition)
        arguments.update(overrides)
        return inventory.verify(self.root, self.baseline,
                                baseline_bytes=self.baseline_bytes, **arguments)

    def test_exact_signed_predecessor_and_actual_whole_retirement(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        prior = inventory.verify(retirement.checkpoint(self.root, self.transition),
                                 self.baseline, baseline_bytes=self.baseline_bytes,
                                 **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        old_ids = {row['id'] for row in prior['retired_routes']}
        new_ids = {row['id'] for row in report['retired_routes']}
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(old_ids), 37)
        self.assertEqual(len(new_ids), 66)
        self.assertEqual(new_ids - old_ids, set(retirement.ROUTES))
        self.assertEqual(report['reviewed_generic_qualification'], prior['reviewed_generic_qualification'])
        self.assertEqual(report['reviewed_generic_retirement']['historical_manifest_count'], 62)
        closure = report['reviewed_generic_retirement']['production_closure']
        self.assertEqual(closure['ordinary_checker_call_sites'], 1)
        self.assertEqual(closure['diagnostic_policy'], 'checked compiler refusals')

    def test_every_reviewed_owner_new_source_and_historical_manifest_rejects_drift(self):
        for label in ['owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities']:
            for owner in self.transition[label]:
                with self.subTest(label=label, owner=owner):
                    self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n'
                    self.assertFalse(self.report()['passed'])
                    self.root.changes.clear()

    def test_each_original_oracle_rejects_unreviewed_drift(self):
        authority = self.transition['original_oracle_preservation']['authority']
        original = json.loads((inventory.ROOT / authority).read_text())['files']
        self.assertEqual(len(original), 49)
        for row in original:
            with self.subTest(owner=row['path']):
                self.root.changes[row['path']] = (self.source_root / row['path']).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_missing_forged_or_prior_authority_fails(self):
        for arguments in [dict(generic_retirement_transition=None),
                          dict(generic_qualification_extension=None),
                          dict(start_funding_transition=None),
                          dict(generic_retirement_transition={**self.transition, 'claim': 'unreviewed'})]:
            self.assertFalse(self.report(**arguments)['passed'])

    def test_false_retirement_partial_inverse_and_policy_cannot_be_rehashed(self):
        for kind in ['retirement', 'inverse', 'policy', 'history']:
            forged = copy.deepcopy(self.transition)
            if kind == 'retirement':
                forged['retired_route_ids'].append('guard.internal-public')
            elif kind == 'inverse':
                owner = next(iter(forged['owners']))
                forged['owners'][owner]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'policy':
                forged['production_closure']['diagnostic_policy'] = 'legacy projection'
            else:
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            self.assertFalse(self.report(generic_retirement_transition=forged)['passed'])

    def test_structural_fallback_checker_and_publication_guards(self):
        # Exercise semantic source fences separately from exact SHA guards.
        mutations = [
            (retirement.ROOT, '#[cfg(test)]\nmod legacy;', 'mod legacy;'),
            (retirement.ROOT, '#[cfg(test)]\nmod pure_calls;', 'mod pure_calls;'),
            (retirement.ROOT, 'profile_selection::select_checked(', 'profile_selection::select('),
            (retirement.ROOT, 'profile_selection::CheckedSelection::Refused(rejection) => {\n            return Ok(RecordingOutcome::Unsupported(rejection.into_checked_gap()));',
             'profile_selection::CheckedSelection::Refused(rejection) => {\n            return Ok(RecordingOutcome::Unsupported(rejection.into_gap()));'),
            (retirement.SELECTOR, '#[cfg(test)]\nfn compatibility_rejection(', 'fn compatibility_rejection('),
            (retirement.SELECTOR, 'effect_plan::lower_with_calls_diagnostic(', 'effect_plan::lower_with_calls('),
            (retirement.REJECTION, 'self.gap.ir_node = node.name;', 'self.gap.ir_node = "Return".into();'),
        ]
        for owner, before, after in mutations:
            with self.subTest(owner=owner, anchor=before):
                source = (self.source_root / owner).read_text()
                self.assertEqual(source.count(before), 1)
                self.root.changes[owner] = source.replace(before, after, 1).encode()
                with self.assertRaises(ValueError):
                    retirement._source_closure(self.root, self.baseline)
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_unrelated_production_drift_stays_live(self):
        owner = retirement.BASE + '/helpers.rs'
        self.root.changes[owner] = (self.source_root / owner).read_bytes() + b'\n// drift\n'
        self.assertFalse(self.report()['passed'])

    def test_cli_protects_transition_and_every_explicit_predecessor(self):
        originals = {MANIFEST: MANIFEST.read_bytes()}
        for path in self.transition['predecessor_options'].values():
            owner = inventory.ROOT / path
            originals[owner] = owner.read_bytes()
        args = ['--generic-retirement-transition', str(MANIFEST)]
        for key, path in self.transition['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        for owner, before in originals.items():
            with self.subTest(owner=str(owner)), contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                inventory.main([*args, '--output', str(owner)])
            self.assertEqual(owner.read_bytes(), before)

    def test_cli_never_writes_referenced_oracle_or_pinned_sources(self):
        args = ['--generic-retirement-transition', str(MANIFEST)]
        for key, path in self.transition['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        # Includes the new inner authority absent from explicit CLI flags, all
        # changed/new sources, historical authorities, and oracle stage inputs.
        paths = inventory.referenced_authority_paths(inventory.ROOT, self.transition)
        authority = inventory.ROOT / self.transition['original_oracle_preservation']['authority']
        originals = json.loads(authority.read_text())['files']
        paths.extend(inventory.ROOT / row['path'] for row in originals)
        self.assertIn(inventory.ROOT / self.transition['original_oracle_preservation']['stages'][0], paths)
        self.assertIn(inventory.ROOT / retirement.ROOT, paths)
        for owner in paths:
            before = owner.read_bytes()
            with self.subTest(owner=str(owner)), mock.patch.object(
                    Path, 'write_text', side_effect=AssertionError('report must not write an authority')) as write:
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    inventory.main([*args, '--output', str(owner)])
                write.assert_not_called()
            self.assertEqual(owner.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
