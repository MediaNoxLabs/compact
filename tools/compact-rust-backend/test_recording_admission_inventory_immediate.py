# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Immediate retirement preserves checked send, shared helpers and old reviews."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_immediate as immediate
import recording_admission_inventory_phase_reset as phase
import recording_admission_inventory_receive as receive
import test_recording_admission_inventory as previous


class ImmediateTransitionTests(unittest.TestCase):
    def setUp(self):
        previous.RetirementTransitionTests.setUp(self)
        for owner in {*immediate.PRIOR_OWNERS, *immediate.SHARED_CALLERS,
                      *phase.REVIEWED_OWNERS, *phase.SHARED_CALLERS}:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(previous.checkpoint_bytes(owner))
        # Preserve the #550 checkpoint and every original mutation expectation.
        for owner, source in previous.zswap_checkpoint_sources(inventory.ROOT).items():
            (self.root / owner).write_text(source)
        self.voting_transition = json.loads(previous.VOTING_TRANSITION.read_text())
        self.phase_transition = json.loads(previous.PHASE_TRANSITION.read_text())
        self.immediate_transition = json.loads(previous.IMMEDIATE_TRANSITION.read_text())

    def report(self):
        return inventory.verify(
            self.root, self.baseline, transition=self.transition,
            baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
            jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
            terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition,
            context_query_transition=self.context_transition, kernel_transition=self.kernel_transition,
            composite_transition=self.composite_transition, receive_transition=self.receive_transition,
            unit_domain_extension=self.unit_domain_extension, voting_transition=self.voting_transition,
            phase_reset_transition=self.phase_transition, immediate_send_transition=self.immediate_transition)

    def test_exact_retirement_reuses_existing_composite_and_keeps_checked_send(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], immediate.RETIRED)
        self.assertTrue(all(row['passed'] for row in report['retired_routes']))
        calls = report['current_selector']['ordered_calls']
        self.assertEqual(len(calls), 9)
        self.assertIn('typed_plan::lower_shielded_send_checked', calls)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertEqual(ids.count('algebra.composite-values'), 1)
        self.assertNotIn(immediate.ROUTE, ids)
        self.assertIn('selector.shielded-send', ids)
        self.assertEqual(report['reviewed_unit_domain_extension']['new_retired_routes'], [])

    def test_missing_or_invalid_review_never_supplies_receive_or_phase_snapshots(self):
        for review in [None, {**self.immediate_transition, 'issue': 999}]:
            self.immediate_transition = review
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertIn('receive retirement: retained shared caller changed: ' + immediate.IMMEDIATE, report['errors'])
            self.assertTrue(any('phase retirement: predecessor reconstruction failed' in e for e in report['errors']))
            self.assertNotIn(immediate.ROUTE, [row['id'] for row in report['retired_routes']])

    def test_all_exact_predecessors_required_and_cannot_be_rehashed(self):
        for name, key in [('phase_transition', 'prior_phase_canonical_sha256'),
                          ('voting_transition', 'prior_voting_canonical_sha256'),
                          ('unit_domain_extension', 'prior_unit_domain_canonical_sha256')]:
            original = copy.deepcopy(getattr(self, name))
            setattr(self, name, None)
            self.assertFalse(self.report()['passed'])
            changed = copy.deepcopy(original)
            changed['claim'] += ' unreviewed'
            setattr(self, name, changed)
            self.immediate_transition[key] = immediate.canonical(changed)
            self.assertFalse(self.report()['passed'])
            setattr(self, name, original)
            self.immediate_transition = json.loads(previous.IMMEDIATE_TRANSITION.read_text())

    def test_authenticated_snapshot_alone_does_not_authorize_broken_older_chain(self):
        self.receive_transition = None
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertNotIn(immediate.ROUTE, [row['id'] for row in report['retired_routes']])

    def test_manifest_fields_do_not_authorize_rebaselining(self):
        original = copy.deepcopy(self.immediate_transition)
        for key in ['reviewed_checkpoint', 'baseline_file_sha256', 'original_oracle_sha256',
                    'checked_send_sha256', 'existing_composite_domain_sha256']:
            self.immediate_transition = copy.deepcopy(original)
            self.immediate_transition[key] = '0' * len(original[key])
            self.assertFalse(self.report()['passed'])
        for key in ['prior_source_fingerprints', 'reviewed_source_fingerprints',
                    'shared_functions_sha256', 'shared_callers_sha256', 'guarded_declarations',
                    'wrapper_functions_sha256', 'variant_annotation', 'import_adaptation']:
            self.immediate_transition = copy.deepcopy(original)
            self.immediate_transition[key] = {}
            self.assertFalse(self.report()['passed'])

    def test_every_oracle_and_option_adapter_guard_is_required_unique_and_pinned(self):
        original_review = copy.deepcopy(self.immediate_transition)
        for owner, markers in immediate.GUARDS.items():
            path = self.root / owner
            source = path.read_text()
            for marker in markers:
                guard = immediate.guarded(marker)
                self.assertEqual(source.count(guard), 1)
                for replacement in (marker, '#[cfg(test)]\n' + guard):
                    path.write_text(source.replace(guard, replacement, 1))
                    self.immediate_transition = copy.deepcopy(original_review)
                    self.immediate_transition['reviewed_source_fingerprints'][owner] = inventory.digest(path.read_bytes())
                    self.assertFalse(self.report()['passed'])
                    with self.assertRaises(ValueError):
                        immediate.predecessor_sources(self.root, original_review)
                path.write_text(source)

    def test_module_shared_helpers_and_checked_send_must_remain_production(self):
        for owner, markers in [(immediate.TYPED, ['mod immediate_send;', immediate.CHECKED_SEND]),
                               (immediate.IMMEDIATE, list(immediate.SHARED_FUNCTIONS))]:
            path = self.root / owner
            source = path.read_text()
            for marker in markers:
                self.assertEqual(source.count(marker), 1)
                path.write_text(source.replace(marker, '#[cfg(test)]\n' + marker, 1))
                self.assertFalse(self.report()['passed'])
            path.write_text(source)
        path = self.root / immediate.SELECTOR
        source = path.read_text()
        path.write_text(source.replace('typed_plan::lower_shielded_send_checked(', 'typed_plan::deleted_send('))
        self.assertFalse(self.report()['passed'])

    def test_original_bodies_imports_and_variant_adaptation_are_exact(self):
        for owner, markers in immediate.GUARDS.items():
            path = self.root / owner
            source = path.read_text()
            for marker in markers:
                path.write_text(source.replace(marker, marker + '/* body drift */', 1))
                self.assertFalse(self.report()['passed'])
            path.write_text(source)
        for owner, before, after in [
            (immediate.TYPED, immediate.GUARDED_VARIANT, immediate.VARIANT),
            (immediate.TYPED, immediate.GUARDED_VARIANT, '    #[allow(dead_code)]\n' + immediate.VARIANT),
            (immediate.IMMEDIATE, immediate.NEW_IMPORT, immediate.OLD_IMPORT),
        ]:
            path = self.root / owner
            source = path.read_text()
            self.assertEqual(source.count(before), 1)
            path.write_text(source.replace(before, after, 1))
            self.assertFalse(self.report()['passed'])
            path.write_text(source)

    def test_shared_helpers_and_callers_are_not_rehashed_away(self):
        path = self.root / immediate.IMMEDIATE
        source = path.read_text()
        changed = source.replace('value == "0"', 'value == "1"', 1)
        self.assertNotEqual(changed, source)
        path.write_text(changed)
        self.assertFalse(self.report()['passed'])
        path.write_text(source)
        for owner in immediate.SHARED_CALLERS:
            path = self.root / owner
            source = path.read_text()
            path.write_text(source + '\n// shared caller drift\n')
            self.assertIn('immediate retirement: shared caller changed: ' + owner, self.report()['errors'])
            path.write_text(source)

    def test_fallback_aliases_cannot_escape_test_only_authority(self):
        for owner in [immediate.SELECTOR, inventory.RECORDED, immediate.TYPED]:
            path = self.root / owner
            source = path.read_text()
            for addition in ['fn accidental() { typed_plan :: lower_immediate_shielded_send (); }',
                             'use super::typed_plan::lower_shielded_send as old;',
                             'use super::typed_plan::immediate_send::{audit_bridge as old};',
                             'fn escaped() { immediate_send :: lower (); }']:
                path.write_text(source + '\n' + addition)
                self.assertFalse(self.report()['passed'])
            path.write_text(source)

    def test_existing_inline_payout_test_calls_do_not_authorize_production_calls(self):
        path = self.root / immediate.PAYOUT_TEST_OWNER
        original = path.read_text()
        for changed in [original.replace('#[cfg(test)]\nmod tests {', 'mod tests {', 1),
                        original + '\nfn escaped() { lower_shielded_send(); }\n']:
            self.assertNotEqual(changed, original)
            path.write_text(changed)
            self.assertIn('immediate retirement: frozen payout test-owner source changed', self.report()['errors'])

    def test_domain_priority_selector_order_and_hook_identity_are_pinned(self):
        path = self.root / immediate.SELECTOR
        source = path.read_text()
        body = immediate.function(source, immediate.COMPOSITE)
        widened = source.replace(body, body.replace('Type::Struct { .. }', 'Type::Struct { .. } | Type::Field', 1), 1)
        moved = source.replace('let through_composite = match through_context', 'let through_composite = match None', 1)
        swapped = source.replace('typed_plan::lower_funded_mint(', '__swap__(')
        swapped = swapped.replace('typed_plan::lower_guarded_deposit(', 'typed_plan::lower_funded_mint(')
        swapped = swapped.replace('__swap__(', 'typed_plan::lower_guarded_deposit(')
        original = copy.deepcopy(self.immediate_transition)
        for changed in [widened, moved, swapped]:
            self.assertNotEqual(changed, source)
            path.write_text(changed)
            self.immediate_transition = copy.deepcopy(original)
            self.immediate_transition['reviewed_source_fingerprints'][immediate.SELECTOR] = inventory.digest(path.read_bytes())
            self.immediate_transition['reviewed_selector'] = inventory.selector_signature(changed)
            self.assertFalse(self.report()['passed'])
        path.write_text(source)
        for extra in ['algebra.composite-values', 'algebra.immediate-send', immediate.ROUTE]:
            self.immediate_transition = copy.deepcopy(original)
            self.immediate_transition['instrumented_route_ids'].append(extra)
            self.assertFalse(self.report()['passed'])
        self.immediate_transition = copy.deepcopy(original)
        self.immediate_transition['retired_route_ids'].append('selector.shielded-send')
        self.assertFalse(self.report()['passed'])

    def test_exact_history_and_historical_receive_pin_reject_forged_snapshots(self):
        originals = immediate.predecessor_sources(self.root, self.immediate_transition)
        self.assertEqual({owner: inventory.digest(text.encode()) for owner, text in originals.items()}, immediate.PRIOR_OWNERS)
        phase_sources = {owner: (self.root / owner).read_text() for owner in phase.REVIEWED_OWNERS}
        phase_sources.update(originals)
        phase.predecessor_source_snapshot(phase_sources, self.phase_transition)
        phase_sources[phase.TYPED] += '\n'
        with self.assertRaises(ValueError):
            phase.predecessor_source_snapshot(phase_sources, self.phase_transition)
        sources = {owner: (self.root / owner).read_text() for owner in self.baseline['source_fingerprints']}
        expected = 'receive retirement: retained shared caller changed: ' + immediate.IMMEDIATE
        self.assertIn(expected, receive.check_sources(self.root, sources, self.receive_transition))
        sources[immediate.IMMEDIATE] = originals[immediate.IMMEDIATE]
        self.assertNotIn(expected, receive.check_sources(self.root, sources, self.receive_transition))
        sources[immediate.IMMEDIATE] += '\n'
        self.assertIn(expected, receive.check_sources(self.root, sources, self.receive_transition))
        self.immediate_transition['predecessor_reconstruction']['prior_selector_suffix'] += '\n'
        self.assertFalse(self.report()['passed'])

    def test_missing_source_fails_without_inventing_history(self):
        for owner in immediate.PRIOR_OWNERS:
            path = self.root / owner
            source = path.read_bytes()
            path.unlink()
            self.assertFalse(self.report()['passed'])
            path.write_bytes(source)

    def test_cli_protects_new_evidence(self):
        path = self.root / 'immediate.json'
        path.write_text(json.dumps(self.immediate_transition))
        before = path.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--immediate-send-transition', str(path), '--output', str(path)])
        self.assertEqual(before, path.read_bytes())


if __name__ == '__main__':
    unittest.main()
