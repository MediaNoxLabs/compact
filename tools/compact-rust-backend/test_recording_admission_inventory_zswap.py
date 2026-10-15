# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Zswap cutover mutations cannot rewrite the unchanged preceding checkpoints."""
import contextlib
import copy
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_immediate as immediate
import recording_admission_inventory_zswap as zswap
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_immediate as prior_tests


class ZswapTransitionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.ImmediateTransitionTests.setUp(self)
        for owner in [*zswap.REVIEWED_OWNERS, zswap.TEST_SOURCE]:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(previous.checkpoint_bytes(owner))
        for owner, source in previous.membership_checkpoint_sources(inventory.ROOT).items():
            (self.root / owner).write_text(source)
        self.zswap_transition = json.loads(previous.ZSWAP_TRANSITION.read_text())

    def report(self):
        return inventory.verify(
            self.root, self.baseline, transition=self.transition,
            baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
            jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
            terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition,
            context_query_transition=self.context_transition, kernel_transition=self.kernel_transition,
            composite_transition=self.composite_transition, receive_transition=self.receive_transition,
            unit_domain_extension=self.unit_domain_extension, voting_transition=self.voting_transition,
            phase_reset_transition=self.phase_transition, immediate_send_transition=self.immediate_transition,
            zswap_transition=self.zswap_transition)

    def test_exact_cutover_preserves_denominator_history_and_final_priority(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], zswap.RETIRED)
        self.assertTrue(all(row['passed'] for row in report['retired_routes']))
        calls = report['current_selector']['ordered_calls']
        self.assertEqual(len(calls), 8)
        self.assertEqual(calls, self.immediate_transition['reviewed_selector']['ordered_calls'][:-1])
        self.assertEqual(calls[-1], 'typed_plan::lower_unit_composition_checked')
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertEqual(ids.count(zswap.REPLACEMENT), 1)
        self.assertNotIn(zswap.ROUTE, ids)
        self.assertEqual(len(report['retired_routes']), 12)
        self.assertEqual(report['reviewed_unit_domain_extension']['new_retired_routes'], [])

    def test_missing_or_invalid_evidence_never_supplies_immediate_snapshot(self):
        for review in [None, {**self.zswap_transition, 'issue': 999}]:
            self.zswap_transition = review
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertTrue(any('immediate retirement: predecessor reconstruction failed' in e for e in report['errors']))
            self.assertNotIn(zswap.ROUTE, [row['id'] for row in report['retired_routes']])

    def test_all_exact_predecessors_required_and_cannot_be_rehashed(self):
        for name, key in [('immediate_transition', 'prior_immediate_canonical_sha256'),
                          ('phase_transition', 'prior_phase_canonical_sha256'),
                          ('voting_transition', 'prior_voting_canonical_sha256'),
                          ('unit_domain_extension', 'prior_unit_domain_canonical_sha256')]:
            original = copy.deepcopy(getattr(self, name))
            setattr(self, name, None)
            self.assertFalse(self.report()['passed'], name)
            changed = copy.deepcopy(original)
            changed['claim'] += ' unreviewed'
            setattr(self, name, changed)
            self.zswap_transition[key] = zswap.canonical(changed)
            self.assertFalse(self.report()['passed'], name)
            setattr(self, name, original)
            self.zswap_transition = json.loads(previous.ZSWAP_TRANSITION.read_text())

    def test_authenticated_source_cannot_authorize_a_broken_earlier_chain(self):
        for name in ['receive_transition', 'kernel_transition', 'call_extension']:
            original = getattr(self, name)
            setattr(self, name, None)
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertNotIn(zswap.ROUTE, [row['id'] for row in report['retired_routes']])
            setattr(self, name, original)

    def test_manifest_cannot_rebaseline_owner_or_reconstruction_identities(self):
        original = copy.deepcopy(self.zswap_transition)
        for key in ['reviewed_checkpoint', 'baseline_file_sha256', 'oracle_source_sha256', 'helper_sha256']:
            self.zswap_transition = copy.deepcopy(original)
            self.zswap_transition[key] = '0' * len(original[key])
            self.assertFalse(self.report()['passed'], key)
        for key in ['prior_source_fingerprints', 'reviewed_source_fingerprints', 'predecessor_reconstruction',
                    'module_guard', 'import_adaptation', 'test_module']:
            self.zswap_transition = copy.deepcopy(original)
            self.zswap_transition[key] = {}
            self.assertFalse(self.report()['passed'], key)

    def test_original_oracle_is_byte_identical_and_cannot_gain_shared_ownership(self):
        path = self.root / zswap.ORACLE
        before = path.read_bytes()
        for after in [before + b'\n// unreviewed drift\n', before.replace(b'plan.effects > 0', b'plan.effects > 99')]:
            path.write_bytes(after)
            self.zswap_transition['oracle_source_sha256'] = zswap.digest(after)
            self.assertFalse(self.report()['passed'])
            path.write_bytes(before)
            self.zswap_transition = json.loads(previous.ZSWAP_TRANSITION.read_text())
        self.zswap_transition['retained_shared_production_symbols'] = ['operand_type']
        self.assertFalse(self.report()['passed'])

    def test_whole_module_guard_is_unique_and_cannot_be_removed_or_relaxed(self):
        path = self.root / zswap.RECORDED
        original = path.read_text()
        for replacement in [zswap.MODULE, '#[cfg(any(test, feature = "oracle"))]\n' + zswap.MODULE,
                            zswap.GUARDED_MODULE + '\n' + zswap.GUARDED_MODULE, '']:
            path.write_text(original.replace(zswap.GUARDED_MODULE, replacement, 1))
            self.assertFalse(self.report()['passed'])
            path.write_text(original)

    def test_retired_lower_import_and_alias_cannot_escape_test_module(self):
        for owner, addition in [
            (zswap.SELECTOR, '\nuse super::zswap_plan::lower as hidden;\n'),
            (zswap.RECORDED, '\nfn hidden() { zswap_plan::lower(); }\n'),
            (inventory.TYPED_PLAN, '\nuse super::zswap_plan;\n'),
            (inventory.TYPED_PLAN, '\nfn hidden() { super::zswap_plan::lower(); }\n'),
        ]:
            path = self.root / owner
            original = path.read_text()
            path.write_text(original + addition)
            report = self.report()
            self.assertFalse(report['passed'])
            sources = {p: (self.root / p).read_text() for p in self.baseline['source_fingerprints']}
            source_errors = zswap.check_sources(self.root, sources, self.zswap_transition)
            self.assertTrue(any('legacy authority escaped test module' in e for e in source_errors))
            path.write_text(original)

    def test_exact_interface_helper_rejects_domain_shape_and_count_policy_changes(self):
        path = self.root / zswap.SELECTOR
        original = path.read_text()
        helper = zswap.function(original, zswap.HELPER)
        for before, after in [
            ('if circuit.internal', 'if false'),
            ('circuit.result != Type::Unit', 'circuit.result != Type::Boolean'),
            ('Type::Bytes { length: 32 }', 'Type::Bytes { length: 31 }'),
            ('max: u64::MAX.to_string()', 'max: u128::MAX.to_string()'),
            ('|| parameter.ty == qualified_coin_type()', ''),
            ('if circuit.internal', 'if circuit.actions.is_empty() || circuit.internal'),
            ('if circuit.internal', 'if circuit.parameters.len() != 3 || circuit.internal'),
            ('StateReturn::Unit | StateReturn::Expression { .. }', 'StateReturn::Unit'),
        ]:
            self.assertIn(before, helper)
            path.write_text(original.replace(helper, helper.replace(before, after, 1), 1))
            # Updating manifest hashes alone must never authorize a new policy.
            self.zswap_transition['helper_sha256'] = zswap.digest(zswap.function(path.read_text(), zswap.HELPER).encode())
            self.zswap_transition['reviewed_source_fingerprints'][zswap.SELECTOR] = zswap.digest(path.read_bytes())
            self.assertFalse(self.report()['passed'], before)
            path.write_text(original)
            self.zswap_transition = json.loads(previous.ZSWAP_TRANSITION.read_text())

    def test_selector_priority_and_checked_error_propagation_are_pinned(self):
        path = self.root / zswap.SELECTOR
        original = path.read_text()
        for before, after in [
            ('let through_zswap = match through_composition', 'let through_zswap = match through_unit'),
            ('checked_zswap_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?',
             'checked_zswap_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits).ok().flatten()'),
            ('"algebra.zswap"', '"selector.zswap"'),
            ('typed_plan::lower_unit_composition_checked(', 'typed_plan::lower_other('),
        ]:
            self.assertIn(before, original)
            path.write_text(original.replace(before, after, 1))
            self.assertFalse(self.report()['passed'], before)
            path.write_text(original)

    def test_extra_hook_call_and_retirement_are_not_authorized(self):
        original = copy.deepcopy(self.zswap_transition)
        for key, value in [('retired_route_ids', 'selector.shielded-send'),
                           ('new_retired_routes', 'selector.unit-composition'),
                           ('instrumented_route_ids', 'algebra.another')]:
            self.zswap_transition[key].append(value)
            self.assertFalse(self.report()['passed'], key)
            self.zswap_transition = copy.deepcopy(original)
        self.zswap_transition['reviewed_selector']['ordered_calls'].pop(0)
        self.assertFalse(self.report()['passed'])

    def test_predecessor_snapshot_is_exact_and_caller_supplied_forgery_fails(self):
        snapshot = zswap.predecessor_sources(self.root, self.zswap_transition)
        self.assertEqual(set(snapshot), set(zswap.PRIOR_OWNERS))
        for owner, expected in zswap.PRIOR_OWNERS.items():
            self.assertEqual(zswap.digest(snapshot[owner].encode()), expected)
        immediate_sources = {owner: (self.root / owner).read_text() for owner in immediate.REVIEWED_OWNERS}
        immediate_sources[zswap.SELECTOR] = snapshot[zswap.SELECTOR]
        historical = immediate.predecessor_source_snapshot(immediate_sources, self.immediate_transition)
        for owner, expected in immediate.PRIOR_OWNERS.items():
            self.assertEqual(zswap.digest(historical[owner].encode()), expected)
        for owner in zswap.REVIEWED_OWNERS:
            sources = {p: (self.root / p).read_text() for p in zswap.REVIEWED_OWNERS}
            sources[owner] += '\n// forged\n'
            with self.assertRaises(ValueError):
                zswap.predecessor_source_snapshot(sources, self.zswap_transition)
        immediate_sources[zswap.SELECTOR] += '\n// forged old selector\n'
        with self.assertRaises(ValueError):
            immediate.predecessor_source_snapshot(immediate_sources, self.immediate_transition)

    def test_missing_sources_fail_closed(self):
        for owner in [*zswap.REVIEWED_OWNERS, zswap.ORACLE]:
            path = self.root / owner
            original = path.read_bytes()
            path.unlink()
            self.assertFalse(self.report()['passed'], owner)
            path.write_bytes(original)

    def test_routing_test_registration_and_source_are_pinned(self):
        path = self.root / zswap.SELECTOR
        original = path.read_text()
        for replacement in ['', zswap.TEST_REGISTRATION.replace('#[cfg(test)]\n', ''),
                            zswap.TEST_REGISTRATION * 2]:
            path.write_text(original.replace(zswap.TEST_REGISTRATION, replacement, 1))
            self.assertFalse(self.report()['passed'])
            path.write_text(original)
        test_path = self.root / zswap.TEST_SOURCE
        original = test_path.read_bytes()
        test_path.write_bytes(original + b'\n// changed\n')
        self.assertFalse(self.report()['passed'])
        test_path.write_bytes(original)

    def test_overwrite_protection_includes_new_evidence(self):
        before = previous.ZSWAP_TRANSITION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            inventory.main(['--zswap-transition', str(previous.ZSWAP_TRANSITION),
                            '--output', str(previous.ZSWAP_TRANSITION)])
        self.assertEqual(error.exception.code, 2)
        self.assertEqual(previous.ZSWAP_TRANSITION.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
