#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Mutation controls for all three domains of the seventh retired family."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_composite as composite
import recording_admission_inventory_receive as receive
import test_recording_admission_inventory as previous


class CompositeTransitionTests(unittest.TestCase):
    setUp = previous.RetirementTransitionTests.setUp
    report = previous.RetirementTransitionTests.report

    def test_explicit_transition_preserves_denominator_prior_routes_and_shared_domain(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], receive.RETIRED)
        self.assertTrue(all(row['passed'] for row in report['retired_routes']))
        self.assertEqual(len(report['current_selector']['ordered_calls']), 12)
        self.assertNotIn('value.composite-domain', composite.RETIRED)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertIn(composite.REPLACEMENT, ids)
        self.assertNotIn(composite.ROUTE, ids)
        without = inventory.verify(self.root, self.baseline, transition=self.transition,
                                   baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
                                   jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
                                   terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition,
                                   context_query_transition=self.context_transition, kernel_transition=self.kernel_transition)
        self.assertFalse(without['passed'])
        self.assertEqual(len(without['retired_routes']), 6)

    def test_original_and_prior_evidence_cannot_be_rebound(self):
        for key in ('baseline_file_sha256', 'original_source_sha256',
                    'prior_kernel_transition_canonical_sha256'):
            with self.subTest(key=key):
                before = self.composite_transition[key]
                self.composite_transition[key] = '0' * 64
                self.assertFalse(self.report()['passed'])
                self.composite_transition[key] = before
        self.composite_transition['prior_source_fingerprints'][composite.SELECTOR] = '0' * 64
        self.assertIn('composite retirement: prior observation owner is not bound to frozen evidence', self.report()['errors'])

    def test_preceding_manifest_is_immutable(self):
        self.kernel_transition['claim'] += ' changed'
        self.assertIn('composite retirement: frozen kernel transition changed or unavailable', self.report()['errors'])

    def test_only_this_issue_route_and_oracle_are_authorized(self):
        for key, value in [('issue', 0), ('retired_route', 'selector.zswap'),
                           ('replacement_route', 'algebra.unreviewed'), ('oracle_source', '../other.rs'),
                           ('field_observation_source', composite.ORACLE)]:
            with self.subTest(key=key):
                before = self.composite_transition[key]
                self.composite_transition[key] = value
                self.assertFalse(self.report()['passed'])
                self.composite_transition[key] = before

    def test_reintroduced_calls_and_aliases_are_rejected(self):
        path = self.root / composite.SELECTOR
        before = path.read_text()
        for addition in ('fn accidental() { typed_plan :: lower_composite (); }',
                         'use super::typed_plan::lower_composite as old;',
                         'use super::typed_plan::composite_reference as old;'):
            path.write_text(before + '\n' + addition + '\n')
            self.assertIn('composite retirement: selector reintroduced old admission import or call', self.report()['errors'])

    def test_both_oracle_modules_and_wrapper_are_uniquely_test_only(self):
        path = self.root / composite.TYPED
        before = path.read_text()
        for declaration in ('mod composite_reference;', 'mod field_observations;',
                            "pub(super) fn lower_composite<'a>("):
            with self.subTest(declaration=declaration):
                changed = before.replace('#[cfg(test)]\n' + declaration, declaration)
                self.assertNotEqual(before, changed)
                path.write_text(changed)
                self.assertIn('composite retirement: both oracle modules and wrapper must be uniquely cfg(test)', self.report()['errors'])
        path.write_text(before + '\nmod composite_reference;\n')
        self.assertIn('composite retirement: both oracle modules and wrapper must be uniquely cfg(test)', self.report()['errors'])

    def test_admission_helpers_cannot_remain_in_production(self):
        path = self.root / composite.TYPED
        path.write_text(path.read_text() + '\nfn composite_type() {}\n')
        self.assertIn('composite retirement: admission helpers remain outside the test-only oracle', self.report()['errors'])

    def test_each_original_function_is_checked_independently(self):
        path = self.root / composite.ORACLE
        before = path.read_text()
        for marker in composite.ORIGINAL_FUNCTIONS:
            actual = composite.ORACLE_MARKER if marker == composite.MARKER else marker
            index = before.index(actual)
            brace = before.index('{\n', index)
            path.write_text(before[:brace + 2] + '    // changed original body\n' + before[brace + 2:])
            self.assertIn('composite retirement: original admission function changed: ' + marker, self.report()['errors'])
        path.unlink()
        self.assertTrue(any('original oracle unreadable' in error for error in self.report()['errors']))

    def test_forged_oracle_hashes_do_not_authorize_modified_functions_or_field_audit(self):
        for key in ('original_functions_sha256', 'oracle_normalized_functions_sha256'):
            before = copy.deepcopy(self.composite_transition[key])
            self.composite_transition[key][composite.MARKER] = '0' * 64
            self.assertIn('composite retirement: all four original admission functions must be preserved', self.report()['errors'])
            self.composite_transition[key] = before
        path = self.root / composite.FIELD
        path.write_bytes(path.read_bytes() + b'\n// modified audit\n')
        self.assertIn('composite retirement: whole field-observation oracle changed', self.report()['errors'])
        self.composite_transition['field_observation_source_sha256'] = inventory.digest(path.read_bytes())
        self.assertIn('composite retirement: whole field-observation oracle must preserve original source', self.report()['errors'])

    def test_unrelated_selector_order_is_not_authorized_by_refreshed_hashes(self):
        path = self.root / composite.SELECTOR
        before = path.read_text()
        changed = before.replace('typed_plan::lower_voting_commit(', '__swap__(')
        changed = changed.replace('typed_plan::lower_funded_mint(', 'typed_plan::lower_voting_commit(')
        changed = changed.replace('__swap__(', 'typed_plan::lower_funded_mint(')
        self.assertNotEqual(before, changed)
        path.write_text(changed)
        self.composite_transition['reviewed_source_fingerprints'][composite.SELECTOR] = inventory.digest(path.read_bytes())
        self.composite_transition['reviewed_selector'] = inventory.selector_signature(changed)
        self.assertIn('composite retirement: unrelated selector calls/order changed', self.report()['errors'])

    def test_other_owners_hooks_retirement_sets_and_shared_policy_remain_guarded(self):
        original = copy.deepcopy(self.composite_transition)
        mutations = [('reviewed_source_fingerprints', {composite.SELECTOR: '0' * 64, composite.TYPED: '0' * 64}),
                     ('instrumented_route_ids', original['instrumented_route_ids'] + ['algebra.unreviewed']),
                     ('retired_route_ids', composite.RETIRED + ['value.composite-domain']),
                     ('retired_route_ids', composite.RETIRED[1:]),
                     ('retained_shared_production_symbols', [])]
        for key, value in mutations:
            self.composite_transition = copy.deepcopy(original)
            self.composite_transition[key] = value
            self.assertFalse(self.report()['passed'])
        self.composite_transition = original
        path = self.root / composite.SELECTOR
        path.write_bytes(path.read_bytes() + b'\n// unreviewed source drift\n')
        self.assertTrue(any('observation owner changed' in error for error in self.report()['errors']))

    def test_cli_cannot_overwrite_manifest(self):
        manifest = self.root / 'composite.json'
        manifest.write_text(json.dumps(self.composite_transition))
        before = manifest.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--composite-transition', str(manifest), '--output', str(manifest)])
        self.assertEqual(before, manifest.read_bytes())


if __name__ == '__main__':
    unittest.main()
