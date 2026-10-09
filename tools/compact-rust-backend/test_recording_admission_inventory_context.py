#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Mutation controls for the fifth explicit production-family retirement."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_context as context
import test_recording_admission_inventory as previous


class ContextQueryTransitionTests(unittest.TestCase):
    setUp = previous.RetirementTransitionTests.setUp
    report = previous.RetirementTransitionTests.report

    def test_explicit_transition_preserves_denominator_prior_retirements_and_hooks(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], context.RETIRED)
        self.assertTrue(all(row['passed'] for row in report['retired_routes']))
        self.assertEqual(len(report['current_selector']['ordered_calls']), 15)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertIn(context.REPLACEMENT, ids)
        self.assertNotIn(context.ROUTE, ids)
        without = inventory.verify(self.root, self.baseline, transition=self.transition,
                                   baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
                                   jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
                                   terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition)
        self.assertFalse(without['passed'])
        self.assertEqual(len(without['retired_routes']), 4)

    def test_original_prior_and_baseline_hash_bindings_are_required(self):
        for key in ('baseline_file_sha256', 'original_source_sha256',
                    'prior_unit_transition_canonical_sha256'):
            with self.subTest(key=key):
                before = self.context_transition[key]
                self.context_transition[key] = '0' * 64
                self.assertFalse(self.report()['passed'])
                self.context_transition[key] = before
        self.context_transition['prior_source_fingerprints'][inventory.SELECTOR] = '0' * 64
        self.assertFalse(self.report()['passed'])

    def test_preceding_manifest_cannot_be_rewritten(self):
        self.unit_transition['claim'] += ' changed'
        self.assertIn('context retirement: frozen unit transition changed or unavailable', self.report()['errors'])

    def test_old_admission_call_reintroduction_is_detected_with_whitespace(self):
        path = self.root / inventory.SELECTOR
        path.write_text(path.read_text() + '\nfn accidental() { typed_plan :: lower_context_query (); }\n')
        self.assertIn('context retirement: selector reintroduced old admission', self.report()['errors'])

    def test_oracle_module_and_wrapper_are_uniquely_test_only(self):
        path = self.root / inventory.TYPED_PLAN
        original = path.read_text()
        for declaration in ('mod context_query_reference;', context.MARKER):
            with self.subTest(declaration=declaration):
                changed = original.replace('#[cfg(test)]\n' + declaration, declaration)
                self.assertNotEqual(original, changed)
                path.write_text(changed)
                self.assertIn('context retirement: oracle module and wrapper must each be uniquely cfg(test)', self.report()['errors'])
        path.write_text(original + '\nfn lower_context_query() {}\n')
        self.assertIn('context retirement: oracle module and wrapper must each be uniquely cfg(test)', self.report()['errors'])

    def test_oracle_body_and_its_normalization_remain_exact(self):
        path = self.root / context.ORACLE
        path.write_text(path.read_text().replace('plan.evidence.kernel_self_reads > 0', 'plan.evidence.kernel_self_reads > 1'))
        self.assertIn('context retirement: oracle no longer equals original admission body', self.report()['errors'])
        self.context_transition['oracle_normalized_body_sha256'] = '0' * 64
        self.assertIn('context retirement: oracle must preserve original admission body after name-only normalization', self.report()['errors'])
        self.context_transition['oracle_body_start'] = 'unreviewed anchor'
        self.assertIn('context retirement: unreviewed oracle location or normalization', self.report()['errors'])

    def test_retained_shared_predicates_and_canonical_descriptor_are_pinned(self):
        for relative, find, replace, error in (
            (context.TYPED, 'allow_context && *ty == contract_address_type()', 'allow_context || *ty == contract_address_type()',
             'shared predicate differs from original: fn context_query_value('),
            (context.TYPED, '*ty == (Type::Bytes { length: 32 }) || *ty == contract_address_type()',
             '*ty == (Type::Bytes { length: 31 }) || *ty == contract_address_type()',
             'shared predicate differs from original: fn context_query_type('),
            (context.IR, 'name: "ContractAddress".into()', 'name: "OtherAddress".into()',
             'shared canonical descriptor differs from original'),
        ):
            with self.subTest(relative=relative, find=find):
                path = self.root / relative
                original = path.read_text()
                self.assertIn(find, original)
                path.write_text(original.replace(find, replace, 1))
                self.assertIn('context retirement: ' + error, self.report()['errors'])
                path.write_text(original)

    def test_shared_voting_commit_evaluator_dependencies_are_not_retired(self):
        for relative, anchors in context.SHARED_ANCHORS.items():
            for anchor in anchors:
                with self.subTest(anchor=anchor):
                    path = self.root / relative
                    original = path.read_text()
                    self.assertIn(anchor, original)
                    path.write_text(original.replace(anchor, '// removed shared dependency', 1))
                    self.assertIn('context retirement: retained shared policy or caller changed: ' + anchor, self.report()['errors'])
                    path.write_text(original)
        self.context_transition['retained_shared_production_symbols'] = []
        self.assertIn('context retirement: shared production ownership claim changed', self.report()['errors'])

    def test_unrelated_selector_reordering_cannot_be_hidden_by_new_fingerprints(self):
        path = self.root / inventory.SELECTOR
        original = path.read_text()
        changed = original.replace('typed_plan::lower_composite(', '__swap__(')
        changed = changed.replace('typed_plan::lower_shielded_receive(', 'typed_plan::lower_composite(')
        changed = changed.replace('__swap__(', 'typed_plan::lower_shielded_receive(')
        self.assertNotEqual(original, changed)
        path.write_text(changed)
        self.context_transition['reviewed_source_fingerprints'][inventory.SELECTOR] = inventory.digest(path.read_bytes())
        self.context_transition['reviewed_selector'] = inventory.selector_signature(changed)
        self.assertIn('context retirement: unrelated selector calls/order changed', self.report()['errors'])

    def test_unreviewed_drift_owners_and_hook_ids_cannot_be_authorized(self):
        original = copy.deepcopy(self.context_transition)
        self.context_transition['reviewed_source_fingerprints'][inventory.RECORDED] = '0' * 64
        self.assertFalse(self.report()['passed'])
        self.context_transition = original
        self.context_transition['instrumented_route_ids'].append('algebra.unreviewed')
        self.assertIn('context retirement: unrelated observation IDs changed', self.report()['errors'])
        self.context_transition = json.loads(previous.CONTEXT_TRANSITION.read_text())
        for relative in (inventory.SELECTOR, inventory.HOOK_SOURCES[-1]):
            path = self.root / relative
            before = path.read_bytes()
            path.write_bytes(before + b'\n// unreviewed drift\n')
            self.assertTrue(any('observation owner changed' in error for error in self.report()['errors']))
            path.write_bytes(before)

    def test_retirement_set_cannot_forget_or_expand_families(self):
        original = copy.deepcopy(self.context_transition)
        for ids in (context.RETIRED[:-1], context.RETIRED[1:], context.RETIRED + ['selector.kernel']):
            self.context_transition = copy.deepcopy(original)
            self.context_transition['retired_route_ids'] = ids
            self.assertIn('context retirement: unreviewed retirement set', self.report()['errors'])

    def test_cli_cannot_overwrite_context_transition(self):
        manifest = self.root / 'context.json'
        manifest.write_text(json.dumps(self.context_transition))
        before = manifest.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--context-query-transition', str(manifest), '--output', str(manifest)])
        self.assertEqual(before, manifest.read_bytes())


if __name__ == '__main__':
    unittest.main()
