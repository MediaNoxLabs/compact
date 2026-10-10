# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Phase-only retirement must preserve the mixed module's production helpers."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_phase_reset as phase
import test_recording_admission_inventory as previous


class PhaseResetTransitionTests(unittest.TestCase):
    def setUp(self):
        previous.RetirementTransitionTests.setUp(self)
        for owner in {*phase.PRIOR_OWNERS, *phase.SHARED_CALLERS}:
            destination = self.root / owner
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((inventory.ROOT / owner).read_bytes())
        self.voting_transition = json.loads(previous.VOTING_TRANSITION.read_text())
        self.phase_transition = json.loads(previous.PHASE_TRANSITION.read_text())

    def report(self):
        return inventory.verify(
            self.root, self.baseline, transition=self.transition,
            baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
            jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
            terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition,
            context_query_transition=self.context_transition, kernel_transition=self.kernel_transition,
            composite_transition=self.composite_transition, receive_transition=self.receive_transition,
            unit_domain_extension=self.unit_domain_extension, voting_transition=self.voting_transition,
            phase_reset_transition=self.phase_transition)

    def test_removes_only_phase_authority_and_reuses_existing_kernel(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], phase.RETIRED)
        self.assertTrue(all(row['passed'] for row in report['retired_routes']))
        self.assertEqual(len(report['current_selector']['ordered_calls']), 10)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertEqual(ids.count('algebra.kernel'), 1)
        self.assertNotIn(phase.ROUTE, ids)
        self.assertNotIn('algebra.phase-reset', ids)
        self.assertEqual(report['reviewed_unit_domain_extension']['new_retired_routes'], [])

    def test_missing_or_invalid_phase_evidence_cannot_skip_voting_review(self):
        for review in [None, {**self.phase_transition, 'issue': 999}]:
            self.phase_transition = review
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertTrue(any('voting retirement: predecessor reconstruction failed' in e for e in report['errors']))
            self.assertNotIn(phase.ROUTE, [row['id'] for row in report['retired_routes']])

    def test_prior_voting_and_unit_evidence_cannot_be_missing_or_rehashed(self):
        for name, key in [('voting_transition', 'prior_voting_canonical_sha256'),
                          ('unit_domain_extension', 'prior_unit_domain_canonical_sha256')]:
            original = copy.deepcopy(getattr(self, name))
            setattr(self, name, None)
            self.assertFalse(self.report()['passed'])
            changed = copy.deepcopy(original)
            changed['claim'] += ' unauthorized'
            setattr(self, name, changed)
            self.phase_transition[key] = phase.canonical(changed)
            self.assertFalse(self.report()['passed'])
            setattr(self, name, original)
            self.phase_transition = json.loads(previous.PHASE_TRANSITION.read_text())

    def test_authenticated_snapshot_does_not_authorize_a_broken_older_chain(self):
        self.receive_transition = None
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertNotIn(phase.ROUTE, [row['id'] for row in report['retired_routes']])
        self.assertTrue(any('frozen receive predecessor' in error for error in report['errors']))

    def test_frozen_identity_manifest_fields_cannot_be_rebound(self):
        original = copy.deepcopy(self.phase_transition)
        for key in ['reviewed_checkpoint', 'baseline_file_sha256', 'original_oracle_sha256',
                    'wrapper_sha256', 'existing_kernel_domain_sha256']:
            self.phase_transition = copy.deepcopy(original)
            self.phase_transition[key] = '0' * len(original[key])
            self.assertFalse(self.report()['passed'])
        for key in ['prior_source_fingerprints', 'reviewed_source_fingerprints',
                    'shared_functions_sha256', 'shared_callers_sha256', 'guarded_declarations']:
            self.phase_transition = copy.deepcopy(original)
            self.phase_transition[key] = {}
            self.assertFalse(self.report()['passed'])

    def test_each_guard_is_required_unique_and_cannot_be_rehashed(self):
        original_review = copy.deepcopy(self.phase_transition)
        for owner, markers in phase.GUARDS.items():
            path = self.root / owner
            source = path.read_text()
            for marker in markers:
                guarded = '#[cfg(test)]\n' + marker
                self.assertEqual(source.count(guarded), 1)
                for replacement in (marker, '#[cfg(test)]\n' + guarded):
                    path.write_text(source.replace(guarded, replacement, 1))
                    self.phase_transition = copy.deepcopy(original_review)
                    self.phase_transition['reviewed_source_fingerprints'][owner] = inventory.digest(path.read_bytes())
                    self.assertFalse(self.report()['passed'])
                    with self.assertRaises(ValueError):
                        phase.predecessor_sources(self.root, original_review)
                path.write_text(source)

    def test_shared_module_helpers_and_constants_must_remain_live(self):
        for owner, markers in [(phase.TYPED, ['mod phase_reset;']),
                               (phase.PHASE, [*phase.SHARED_FUNCTIONS, 'pub(super) const WIDENED:', 'pub(super) const SUM:'])]:
            path = self.root / owner
            source = path.read_text()
            for marker in markers:
                self.assertEqual(source.count(marker), 1)
                path.write_text(source.replace(marker, '#[cfg(test)]\n' + marker, 1))
                self.assertFalse(self.report()['passed'])
            path.write_text(source)

    def test_original_audit_lower_and_wrapper_bodies_stay_frozen(self):
        for owner, marker in [(phase.PHASE, "struct Audit<'a>"),
                              (phase.PHASE, "impl Audit<'_>"),
                              (phase.PHASE, "pub(super) fn lower<'a>("),
                              (phase.TYPED, phase.MARKER)]:
            path = self.root / owner
            source = path.read_text()
            self.assertEqual(source.count(marker), 1)
            path.write_text(source.replace(marker, marker + '/* changed body */', 1))
            self.assertFalse(self.report()['passed'])
            path.write_text(source)

    def test_live_shared_callers_cannot_change_even_outside_observation_owners(self):
        for owner in phase.SHARED_CALLERS:
            path = self.root / owner
            source = path.read_text()
            path.write_text(source + '\n// changed shared caller\n')
            self.assertIn('phase retirement: shared caller changed: ' + owner, self.report()['errors'])
            path.write_text(source)

    def test_old_fallback_and_import_aliases_cannot_return(self):
        for owner in [phase.SELECTOR, inventory.RECORDED, phase.TYPED]:
            path = self.root / owner
            source = path.read_text()
            for addition in ['fn accidental() { typed_plan :: lower_phase_reset (); }',
                             'use super::typed_plan::lower_phase_reset as old;',
                             'use super::typed_plan::phase_reset::{lower as old};']:
                path.write_text(source + '\n' + addition)
                self.assertFalse(self.report()['passed'])
            path.write_text(source)

    def test_kernel_domain_and_route_position_stay_unchanged(self):
        path = self.root / phase.SELECTOR
        source = path.read_text()
        kernel = phase.function(source, phase.KERNEL)
        widened = source.replace(kernel, kernel.replace('if circuit.internal', 'if false', 1), 1)
        self.assertNotEqual(widened, source)
        moved = source.replace('let before_context = match typed_prefix', 'let before_context = match None', 1)
        self.assertNotEqual(moved, source)
        for text in (widened, moved):
            path.write_text(text)
            self.phase_transition['reviewed_source_fingerprints'][phase.SELECTOR] = inventory.digest(path.read_bytes())
            self.phase_transition['reviewed_selector'] = inventory.selector_signature(text)
            self.assertFalse(self.report()['passed'])

    def test_selector_order_and_existing_hook_uniqueness_are_pinned(self):
        path = self.root / phase.SELECTOR
        source = path.read_text()
        changed = source.replace('typed_plan::lower_funded_mint(', '__swap__(')
        changed = changed.replace('typed_plan::lower_guarded_deposit(', 'typed_plan::lower_funded_mint(')
        changed = changed.replace('__swap__(', 'typed_plan::lower_guarded_deposit(')
        path.write_text(changed)
        self.phase_transition['reviewed_source_fingerprints'][phase.SELECTOR] = inventory.digest(path.read_bytes())
        self.phase_transition['reviewed_selector'] = inventory.selector_signature(changed)
        self.assertFalse(self.report()['passed'])
        path.write_text(source)
        original = json.loads(previous.PHASE_TRANSITION.read_text())
        for extra in ['algebra.kernel', 'algebra.phase-reset', phase.ROUTE]:
            self.phase_transition = copy.deepcopy(original)
            self.phase_transition['instrumented_route_ids'].append(extra)
            self.assertFalse(self.report()['passed'])
        self.phase_transition = copy.deepcopy(original)
        self.phase_transition['retired_route_ids'].append('selector.funded-mint')
        self.assertFalse(self.report()['passed'])

    def test_reconstruction_fragments_and_sources_cannot_be_forged(self):
        original = copy.deepcopy(self.phase_transition)
        sources = phase.predecessor_sources(self.root, original)
        self.assertEqual({owner: inventory.digest(text.encode()) for owner, text in sources.items()}, phase.PRIOR_OWNERS)
        for key in original['predecessor_reconstruction']:
            self.phase_transition = copy.deepcopy(original)
            self.phase_transition['predecessor_reconstruction'][key] += '\n'
            self.assertFalse(self.report()['passed'])
        self.phase_transition = original
        for owner in phase.PRIOR_OWNERS:
            path = self.root / owner
            source = path.read_bytes()
            path.unlink()
            self.assertFalse(self.report()['passed'])
            path.write_bytes(source)

    def test_cli_protects_new_transition(self):
        path = self.root / 'phase.json'
        path.write_text(json.dumps(self.phase_transition))
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--phase-reset-transition', str(path), '--output', str(path)])
        self.assertEqual(before, path.read_bytes())


if __name__ == '__main__':
    unittest.main()
