# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Current voting cutover mutations; historical tests keep their prior checkpoint."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_voting as voting
import recording_admission_inventory_phase_reset as phase_reset
import test_recording_admission_inventory as previous


class VotingTransitionTests(unittest.TestCase):
    def setUp(self):
        previous.RetirementTransitionTests.setUp(self)
        # Keep this suite on its exact reviewed #547 checkpoint, independently
        # of later retirements; original assertions still own their old failures.
        phase_review = json.loads(previous.PHASE_TRANSITION.read_text())
        phase_sources = {owner: (inventory.ROOT / owner).read_text() for owner in phase_reset.REVIEWED_OWNERS}
        phase_sources.update(previous.immediate_checkpoint_sources(inventory.ROOT))
        for owner, source in phase_reset.predecessor_source_snapshot(phase_sources, phase_review).items():
            (self.root / owner).write_text(source)
        self.voting_transition = json.loads(previous.VOTING_TRANSITION.read_text())

    def report(self):
        return inventory.verify(
            self.root, self.baseline, transition=self.transition,
            baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
            jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
            terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition,
            context_query_transition=self.context_transition, kernel_transition=self.kernel_transition,
            composite_transition=self.composite_transition, receive_transition=self.receive_transition,
            unit_domain_extension=self.unit_domain_extension, voting_transition=self.voting_transition)

    def test_exact_cutover_preserves_denominator_and_previous_unit_review(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], voting.RETIRED)
        self.assertTrue(all(row['passed'] for row in report['retired_routes']))
        self.assertEqual(len(report['current_selector']['ordered_calls']), 11)
        self.assertEqual(report['reviewed_unit_domain_extension']['new_retired_routes'], [])
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertIn(voting.REPLACEMENT, ids)
        self.assertNotIn(voting.ROUTE, ids)

    def test_missing_or_invalid_voting_evidence_cannot_supply_historical_snapshot(self):
        for transition in [None, {**self.voting_transition, 'issue': 999}]:
            with self.subTest(transition=transition is None):
                self.voting_transition = transition
                report = self.report()
                self.assertFalse(report['passed'])
                self.assertIn('Unit domain: reviewed observation owner changed', report['errors'])
                self.assertEqual(len(report['retired_routes']), 8)

    def test_both_predecessors_required_and_cannot_be_rehashed(self):
        for name, key in [('receive_transition', 'prior_receive_transition_canonical_sha256'),
                          ('unit_domain_extension', 'prior_unit_domain_canonical_sha256')]:
            original = copy.deepcopy(getattr(self, name))
            setattr(self, name, None)
            self.assertFalse(self.report()['passed'])
            altered = copy.deepcopy(original)
            altered['claim'] += ' unreviewed'
            setattr(self, name, altered)
            self.voting_transition[key] = voting.canonical(altered)
            self.assertFalse(self.report()['passed'])
            setattr(self, name, original)
            self.voting_transition = json.loads(previous.VOTING_TRANSITION.read_text())

    def test_manifest_identity_fields_are_not_self_authorizing(self):
        original = copy.deepcopy(self.voting_transition)
        for key in ['baseline_file_sha256', 'reviewed_checkpoint', 'oracle_source_sha256',
                    'wrapper_sha256', 'helper_sha256', 'prior_unit_domain_canonical_sha256',
                    'prior_receive_transition_canonical_sha256']:
            with self.subTest(key=key):
                self.voting_transition = copy.deepcopy(original)
                self.voting_transition[key] = '0' * len(original[key])
                self.assertFalse(self.report()['passed'])
        for key in ['prior_source_fingerprints', 'reviewed_source_fingerprints']:
            self.voting_transition = copy.deepcopy(original)
            self.voting_transition[key][voting.SELECTOR] = '0' * 64
            self.assertFalse(self.report()['passed'])

    def test_original_oracle_and_wrapper_cannot_be_edited_or_rehashed(self):
        oracle = self.root / voting.ORACLE
        oracle.write_text(oracle.read_text() + '\n// changed oracle\n')
        self.voting_transition['oracle_source_sha256'] = inventory.digest(oracle.read_bytes())
        self.assertFalse(self.report()['passed'])
        self.voting_transition = json.loads(previous.VOTING_TRANSITION.read_text())
        self.assertIn('voting retirement: frozen oracle source changed', self.report()['errors'])
        oracle.write_bytes((inventory.ROOT / voting.ORACLE).read_bytes())
        typed = self.root / voting.TYPED
        typed.write_text(typed.read_text().replace('voting_commit::lower(circuit,', 'voting_commit::lower(/* drift */ circuit,'))
        self.assertIn('voting retirement: frozen wrapper body changed', self.report()['errors'])

    def test_module_and_wrapper_guards_are_unique_and_required(self):
        path = self.root / voting.TYPED
        original = path.read_text()
        for declaration in (voting.MODULE, voting.MARKER):
            guarded = '#[cfg(test)]\n' + declaration
            for replacement in (declaration, '#[cfg(test)]\n' + guarded):
                with self.subTest(declaration=declaration, replacement=replacement):
                    self.assertEqual(original.count(guarded), 1)
                    path.write_text(original.replace(guarded, replacement, 1))
                    self.assertFalse(self.report()['passed'])
        path.write_text(original + '\nmod voting_commit;\n')
        self.assertIn('voting retirement: oracle module and wrapper must be uniquely cfg(test)', self.report()['errors'])

    def test_production_fallback_aliases_and_escaped_wrapper_are_refused(self):
        for owner in (voting.SELECTOR, inventory.RECORDED):
            path = self.root / owner
            original = path.read_text()
            for addition in ['fn accidental() { typed_plan :: lower_voting_commit (); }',
                             'use super::typed_plan::lower_voting_commit as old;',
                             'use super::typed_plan::voting_commit as old;']:
                path.write_text(original + '\n' + addition)
                self.assertFalse(self.report()['passed'])
            path.write_text(original)
        path = self.root / voting.TYPED
        original = path.read_text()
        path.write_text(original.replace('voting_commit::lower(circuit, ledger, witnesses, pure, circuits)', 'None')
                        + '\nfn escaped() { voting_commit :: lower (); }\n')
        self.assertIn('voting retirement: reference authority escaped the test-only wrapper', self.report()['errors'])

    def test_domain_and_unrelated_owner_drift_cannot_be_authorized_by_rehash(self):
        path = self.root / voting.SELECTOR
        original = path.read_text()
        for changed in [original.replace('ballot.ty == Type::Boolean',
                                         'matches!(ballot.ty, Type::Boolean | Type::Field)', 1),
                        original + '\nfn unrelated() {}\n']:
            path.write_text(changed)
            self.voting_transition['reviewed_source_fingerprints'][voting.SELECTOR] = inventory.digest(path.read_bytes())
            self.voting_transition['helper_sha256'] = inventory.digest(voting.function(changed, voting.HELPER).encode())
            self.assertFalse(self.report()['passed'])

    def test_selector_order_attempt_position_and_hook_ids_are_pinned(self):
        path = self.root / voting.SELECTOR
        original = path.read_text()
        swapped = original.replace('typed_plan::lower_funded_mint(', '__swap__(')
        swapped = swapped.replace('typed_plan::lower_guarded_deposit(', 'typed_plan::lower_funded_mint(')
        swapped = swapped.replace('__swap__(', 'typed_plan::lower_guarded_deposit(')
        changed_hook = original.replace('"algebra.voting-commit"', '"selector.voting-commit"')
        # Swap the two complete attempts while retaining both and every legacy
        # call. A call-name-only inventory would miss this priority mutation.
        start = original.index('        let through_receive =')
        middle = original.index('        let through_voting =', start)
        end = original.index('        let before_terminal =', middle)
        receive_attempt = original[start:middle].replace('match through_composite', 'match through_voting')
        voting_attempt = original[middle:end].replace('match through_receive', 'match through_composite')
        moved = (original[:start] + voting_attempt + receive_attempt
                 + original[end:].replace('let before_terminal = through_voting',
                                          'let before_terminal = through_receive', 1))
        self.assertEqual(inventory.selector_signature(moved)['ordered_calls'],
                         inventory.selector_signature(original)['ordered_calls'])
        for changed in [swapped, changed_hook, moved]:
            self.assertNotEqual(changed, original)
            path.write_text(changed)
            self.voting_transition['reviewed_selector'] = inventory.selector_signature(changed)
            self.voting_transition['reviewed_source_fingerprints'][voting.SELECTOR] = inventory.digest(path.read_bytes())
            self.assertFalse(self.report()['passed'])
        path.write_text(original)
        self.voting_transition = json.loads(previous.VOTING_TRANSITION.read_text())
        self.voting_transition['instrumented_route_ids'].append('algebra.unreviewed')
        self.assertFalse(self.report()['passed'])

    def test_no_extra_retirement_or_forged_reconstruction_fragments(self):
        original = copy.deepcopy(self.voting_transition)
        for key in ('retired_route_ids', 'new_retired_routes'):
            self.voting_transition = copy.deepcopy(original)
            self.voting_transition[key].append('selector.funded-mint')
            self.assertFalse(self.report()['passed'])
        for key in original['predecessor_reconstruction']:
            self.voting_transition = copy.deepcopy(original)
            self.voting_transition['predecessor_reconstruction'][key] += '\n'
            self.assertFalse(self.report()['passed'])
            with self.assertRaises(ValueError):
                voting.predecessor_selector((self.root / voting.SELECTOR).read_text(), self.voting_transition)

    def test_duplicate_helper_and_missing_sources_fail(self):
        path = self.root / voting.SELECTOR
        original = path.read_text()
        path.write_text(original + '\nfn checked_voting_domain() {}\n')
        self.assertFalse(self.report()['passed'])
        path.write_text(original)
        for owner in (voting.SELECTOR, voting.TYPED, voting.ORACLE):
            path = self.root / owner
            original = path.read_bytes()
            path.unlink()
            self.assertFalse(self.report()['passed'])
            path.write_bytes(original)

    def test_exact_historical_reconstruction_rejects_source_drift(self):
        selector = (self.root / voting.SELECTOR).read_text()
        typed = (self.root / voting.TYPED).read_text()
        self.assertEqual(inventory.digest(voting.predecessor_selector(selector, self.voting_transition).encode()),
                         voting.PRIOR_OWNERS[voting.SELECTOR])
        self.assertEqual(inventory.digest(voting.predecessor_typed(typed).encode()), voting.PRIOR_OWNERS[voting.TYPED])
        for text in (typed + '\n', typed.replace('#[cfg(test)]\nmod voting_commit;', 'mod voting_commit;')):
            with self.assertRaises(ValueError):
                voting.predecessor_typed(text)
        with self.assertRaises(ValueError):
            voting.predecessor_selector(selector + '\n', self.voting_transition)

    def test_cli_cannot_overwrite_voting_evidence(self):
        path = self.root / 'voting.json'
        path.write_text(json.dumps(self.voting_transition))
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--voting-transition', str(path), '--output', str(path)])
        self.assertEqual(before, path.read_bytes())


if __name__ == '__main__':
    unittest.main()
