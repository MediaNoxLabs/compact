#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Mutation controls for the sixth explicit production-family retirement."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_kernel as kernel
import test_recording_admission_inventory as previous


class KernelTransitionTests(unittest.TestCase):
    setUp = previous.RetirementTransitionTests.setUp
    report = previous.RetirementTransitionTests.report

    def test_explicit_transition_preserves_denominator_prior_retirements_and_hooks(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], kernel.RETIRED)
        self.assertTrue(all(row['passed'] for row in report['retired_routes']))
        self.assertEqual(len(report['current_selector']['ordered_calls']), 14)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertIn(kernel.REPLACEMENT, ids)
        self.assertNotIn(kernel.ROUTE, ids)
        without = inventory.verify(self.root, self.baseline, transition=self.transition,
                                   baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
                                   jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
                                   terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition,
                                   context_query_transition=self.context_transition)
        self.assertFalse(without['passed'])
        self.assertEqual(len(without['retired_routes']), 5)

    def test_baseline_and_both_prior_manifest_bindings_are_required(self):
        for key in ('baseline_file_sha256', 'original_source_sha256',
                    'prior_context_transition_canonical_sha256', 'prior_call_extension_canonical_sha256'):
            with self.subTest(key=key):
                before = self.kernel_transition[key]
                self.kernel_transition[key] = '0' * 64
                self.assertFalse(self.report()['passed'])
                self.kernel_transition[key] = before
        for owner in kernel.SOURCES:
            before = self.kernel_transition['prior_source_fingerprints'][owner]
            self.kernel_transition['prior_source_fingerprints'][owner] = '0' * 64
            self.assertIn('kernel retirement: prior observation owners are not bound to frozen evidence', self.report()['errors'])
            self.kernel_transition['prior_source_fingerprints'][owner] = before

    def test_preceding_manifest_cannot_be_rewritten(self):
        self.context_transition['claim'] += ' changed'
        self.assertIn('kernel retirement: frozen context transition changed or unavailable', self.report()['errors'])

    def test_unreviewed_issue_route_or_oracle_location_is_rejected(self):
        for key, value in [('issue', 0), ('retired_route', 'selector.zswap'),
                           ('replacement_route', 'algebra.unreviewed'), ('oracle_source', '../other.rs')]:
            with self.subTest(key=key):
                before = self.kernel_transition[key]
                self.kernel_transition[key] = value
                self.assertFalse(self.report()['passed'])
                self.kernel_transition[key] = before

    def test_old_planner_call_or_alias_import_is_detected_with_whitespace(self):
        path = self.root / inventory.SELECTOR
        before = path.read_text()
        for addition in ('fn accidental() { kernel_plan :: lower (); }',
                         'use super::kernel_plan as legacy_kernel;'):
            with self.subTest(addition=addition):
                path.write_text(before + '\n' + addition + '\n')
                self.assertIn('kernel retirement: selector reintroduced old planner import or call', self.report()['errors'])

    def test_whole_module_is_uniquely_test_only(self):
        path = self.root / inventory.RECORDED
        original = path.read_text()
        for changed in (original.replace('#[cfg(test)]\nmod kernel_plan;', 'mod kernel_plan;'),
                        original + '\nmod kernel_plan;\n',
                        original.replace('mod kernel_plan;', 'mod other_kernel;')):
            with self.subTest(changed=changed[-40:]):
                self.assertNotEqual(original, changed)
                path.write_text(changed)
                self.assertIn('kernel retirement: whole oracle module must be uniquely cfg(test)', self.report()['errors'])

    def test_entire_oracle_is_byte_identical_including_nonfunction_text(self):
        path = self.root / kernel.ORACLE
        before = path.read_bytes()
        for after in (before + b'\n// source changed\n',
                      before.replace(b'plan.effects > 0', b'plan.effects > 1')):
            with self.subTest(after=after[-30:]):
                self.assertNotEqual(before, after)
                path.write_bytes(after)
                self.assertIn('kernel retirement: whole oracle differs from frozen original source', self.report()['errors'])
        path.unlink()
        self.assertIn('kernel retirement: original oracle source unreadable', self.report()['errors'])

    def test_forged_oracle_digest_cannot_authorize_changed_body(self):
        path = self.root / kernel.ORACLE
        path.write_bytes(path.read_bytes() + b'\n// changed\n')
        self.kernel_transition['oracle_source_sha256'] = inventory.digest(path.read_bytes())
        self.assertIn('kernel retirement: whole oracle must preserve original source exactly', self.report()['errors'])

    def test_unrelated_selector_reordering_cannot_be_hidden_by_new_fingerprints(self):
        path = self.root / inventory.SELECTOR
        original = path.read_text()
        changed = original.replace('typed_plan::lower_composite(', '__swap__(')
        changed = changed.replace('typed_plan::lower_shielded_receive(', 'typed_plan::lower_composite(')
        changed = changed.replace('__swap__(', 'typed_plan::lower_shielded_receive(')
        self.assertNotEqual(original, changed)
        path.write_text(changed)
        self.kernel_transition['reviewed_source_fingerprints'][inventory.SELECTOR] = inventory.digest(path.read_bytes())
        self.kernel_transition['reviewed_selector'] = inventory.selector_signature(changed)
        self.assertIn('kernel retirement: unrelated selector calls/order changed', self.report()['errors'])

    def test_unreviewed_drift_owners_and_hook_ids_cannot_be_authorized(self):
        original = copy.deepcopy(self.kernel_transition)
        self.kernel_transition['reviewed_source_fingerprints'][inventory.HOOK_SOURCES[-1]] = '0' * 64
        self.assertFalse(self.report()['passed'])
        self.kernel_transition = original
        self.kernel_transition['instrumented_route_ids'].append('algebra.unreviewed')
        self.assertIn('kernel retirement: unrelated observation IDs changed', self.report()['errors'])
        self.kernel_transition = json.loads(previous.KERNEL_TRANSITION.read_text())
        for relative in (*kernel.SOURCES, inventory.HOOK_SOURCES[-1]):
            path = self.root / relative
            before = path.read_bytes()
            path.write_bytes(before + b'\n// unreviewed drift\n')
            self.assertTrue(any('observation owner changed' in error for error in self.report()['errors']))
            path.write_bytes(before)

    def test_retirement_set_cannot_forget_or_expand_families(self):
        original = copy.deepcopy(self.kernel_transition)
        for ids in (kernel.RETIRED[:-1], kernel.RETIRED[1:], kernel.RETIRED + ['selector.zswap']):
            self.kernel_transition = copy.deepcopy(original)
            self.kernel_transition['retired_route_ids'] = ids
            self.assertIn('kernel retirement: unreviewed retirement set', self.report()['errors'])
        self.kernel_transition = original
        self.kernel_transition['retained_shared_production_symbols'] = ['kernel_plan::lower']
        self.assertIn('kernel retirement: this oracle has no shared production users', self.report()['errors'])

    def test_cli_cannot_overwrite_kernel_transition(self):
        manifest = self.root / 'kernel.json'
        manifest.write_text(json.dumps(self.kernel_transition))
        before = manifest.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--kernel-transition', str(manifest), '--output', str(manifest)])
        self.assertEqual(before, manifest.read_bytes())


if __name__ == '__main__':
    unittest.main()
