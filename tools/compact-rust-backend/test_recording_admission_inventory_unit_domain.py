# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Ordinary Unit domain extension must not silently rebaseline an owner."""
import contextlib
import copy
import io
import json
import unittest
import recording_admission_inventory as inventory
import recording_admission_inventory_unit_domain as domain
import test_recording_admission_inventory as previous


class UnitDomainExtensionTests(unittest.TestCase):
    setUp = previous.RetirementTransitionTests.setUp
    report = previous.RetirementTransitionTests.report

    def test_explicit_extension_preserves_all_routes_and_selector(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 8)
        self.assertEqual(report['current_selector'], self.receive_transition['reviewed_selector'])
        self.assertEqual(report['reviewed_unit_domain_extension']['new_retired_routes'], [])
        self.unit_domain_extension = None
        self.assertFalse(self.report()['passed'])

    def test_predecessor_cannot_be_rehashed_or_omitted(self):
        self.receive_transition['claim'] += ' changed'
        self.unit_domain_extension['prior_receive_transition_canonical_sha256'] = domain.canonical(self.receive_transition)
        self.assertIn('Unit domain: frozen receive predecessor changed or unavailable', self.report()['errors'])
        self.receive_transition = None
        self.assertFalse(self.report()['passed'])

    def test_manifest_identity_and_bound_helpers_cannot_change(self):
        original = copy.deepcopy(self.unit_domain_extension)
        for key, value in [('issue', 999), ('schema_version', 2), ('classification', 'new-retirement'),
                           ('helper', 'fn other('), ('prior_helper_sha256', '0' * 64),
                           ('reviewed_helper_sha256', '0' * 64), ('prior_source_fingerprints', {})]:
            with self.subTest(key=key):
                self.unit_domain_extension = {**original, key: value}
                self.assertFalse(self.report()['passed'])

    def test_no_new_route_order_hook_or_retirement_is_authorized(self):
        original = copy.deepcopy(self.unit_domain_extension)
        for key, value in [('instrumented_route_ids', []), ('retired_route_ids', []),
                           ('new_retired_routes', ['selector.phase-reset']),
                           ('prior_selector', {}), ('reviewed_selector', {})]:
            with self.subTest(key=key):
                self.unit_domain_extension = {**original, key: value}
                self.assertFalse(self.report()['passed'])

    def test_exact_owner_scope_and_digest_are_required(self):
        original = copy.deepcopy(self.unit_domain_extension)
        for value in [{}, {domain.SELECTOR: 'bad'}, {domain.SELECTOR: None},
                      {**original['reviewed_source_fingerprints'], inventory.RECORDED: '0' * 64}]:
            self.unit_domain_extension = {**original, 'reviewed_source_fingerprints': value}
            self.assertFalse(self.report()['passed'])

    def test_widened_helper_cannot_be_approved_by_rehash(self):
        path = self.root / domain.SELECTOR
        text = path.read_text().replace('StateReturn::Unit | StateReturn::Expression { .. }',
                                        'StateReturn::Unit | StateReturn::Expression { .. } | StateReturn::Effectful { .. }', 1)
        path.write_text(text)
        self.unit_domain_extension['reviewed_source_fingerprints'][domain.SELECTOR] = inventory.digest(path.read_bytes())
        self.unit_domain_extension['reviewed_helper_sha256'] = inventory.digest(domain.function(text, domain.MARKER).encode())
        self.assertIn('Unit domain: current helper differs from reviewed body', self.report()['errors'])

    def test_unrelated_body_drift_cannot_be_approved_by_owner_rehash(self):
        path = self.root / domain.SELECTOR
        path.write_text(path.read_text() + '\nfn unreviewed() {}\n')
        self.unit_domain_extension['reviewed_source_fingerprints'][domain.SELECTOR] = inventory.digest(path.read_bytes())
        self.assertIn('Unit domain: unrelated selector owner content changed', self.report()['errors'])

    def test_selector_reorder_cannot_be_approved_by_rehash(self):
        path = self.root / domain.SELECTOR
        text = path.read_text().replace('typed_plan::lower_voting_commit(', 'typed_plan::lower_unreviewed(')
        path.write_text(text)
        self.unit_domain_extension['reviewed_source_fingerprints'][domain.SELECTOR] = inventory.digest(path.read_bytes())
        self.unit_domain_extension['reviewed_selector'] = inventory.selector_signature(text)
        self.assertIn('Unit domain: unchanged selector/routes required: reviewed_selector', self.report()['errors'])

    def test_duplicate_helper_and_missing_source_fail(self):
        path = self.root / domain.SELECTOR
        path.write_text(path.read_text() + '\nfn checked_unit_domain() {}\n')
        self.assertTrue(any('function anchor is not unique' in e for e in self.report()['errors']))
        path.unlink()
        self.assertTrue(any('Unit domain: unreadable helper:' in e for e in self.report()['errors']))

    def test_cli_cannot_overwrite_extension_evidence(self):
        path = self.root / 'unit-domain.json'
        path.write_text(json.dumps(self.unit_domain_extension))
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--unit-domain-extension', str(path), '--output', str(path)])
        self.assertEqual(before, path.read_bytes())


if __name__ == '__main__':
    unittest.main()
