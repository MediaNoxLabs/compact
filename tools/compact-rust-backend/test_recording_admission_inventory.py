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
import recording_admission_inventory_jubjub as jubjub
import recording_admission_inventory_calls as calls
import recording_admission_inventory_terminal as terminal
import recording_admission_inventory_unit as unit
import recording_admission_inventory_context as context
import recording_admission_inventory_kernel as kernel
import recording_admission_inventory_composite as composite
import recording_admission_inventory_receive as receive
import recording_admission_inventory_voting as voting
import recording_admission_inventory_phase_reset as phase_reset
import recording_admission_inventory_immediate as immediate
import recording_admission_inventory_zswap as zswap
import recording_admission_inventory_membership as membership
import recording_admission_inventory_send as send
import recording_admission_inventory_observation_test as observation_test
import recording_admission_inventory_merge as merge
import recording_admission_inventory_guarded as guarded
import recording_admission_inventory_routing_test as routing_test
import recording_admission_inventory_inequality_test as inequality_test
import recording_admission_inventory_reset_payout as reset_payout
import recording_admission_inventory_shielded_payout as shielded_payout
import recording_admission_inventory_funded_mint as funded_mint

JUBJUB_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-517.json")
CALL_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.calls-519.json")


TERMINAL_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-521.json")


UNIT_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-525.json")


CONTEXT_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-526.json")


KERNEL_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-527.json")
COMPOSITE_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-529.json")
RECEIVE_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-536.json")


UNIT_DOMAIN_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.unit-domain-540.json")
VOTING_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-547.json")
PHASE_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-549.json")
IMMEDIATE_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-550.json")


ZSWAP_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-553.json")


MEMBERSHIP_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-560.json")


SEND_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-561.json")


OBSERVATION_TEST_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.observation-test-564.json")


def observation_checkpoint_source(root):
    return observation_test.predecessor_source(
        (root / observation_test.OWNER).read_text(), json.loads(OBSERVATION_TEST_EXTENSION.read_text()))


MERGE_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-565.json")


GUARDED_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-566.json")


ROUTING_TEST_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.routing-test-568.json")


INEQUALITY_TEST_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.inequality-test-571.json")


KECCAK_TEST_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.keccak-test-581.json")


ALIGNED_TEST_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.aligned-test-582.json")


ENCODING_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.encoding-583.json")


NEGATION_TEST_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.negation-test-585.json")


VALUE_TEST_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.value-test-586.json")


UNIT_COMPOSITION_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-580.json")


OPTIONAL_WRITE_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-587.json")


ENUM_ADVANCE_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-588.json")


WITNESS_MERKLE_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-589.json")


ORGANIZER_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-590.json")


UNSIGNED_PRODUCT_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-591.json")


OPAQUE_SET_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-592.json")


UNIT_INTERFACE_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.unit-interface-593.json")


MAP_VALUE_EXTENSION = inventory.BASELINE.with_name("recording_admission_inventory.map-values-595.json")


GUARDED_MAP_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-594.json")


OPAQUE_MAP_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-596.json")


def checkpoint_594(root):
    """Authenticate signed Map-read retirement before historical fixture setup."""
    transition = json.loads(OPAQUE_MAP_TRANSITION.read_text())
    try:
        source = (root / inventory.RECORDED).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.RECORDED]['current_sha256']:
        return inventory.opaque_map.checkpoint(root, transition)
    return root


def checkpoint_595(root):
    """Authenticate final Map-value checkpoint before historical fixture projection."""
    root = checkpoint_594(root)
    transition = json.loads(GUARDED_MAP_TRANSITION.read_text())
    try:
        source = (root / inventory.RECORDED).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.RECORDED]['current_sha256']:
        return inventory.guarded_map.checkpoint(root, transition)
    return root


def checkpoint_593(root):
    """Historical fixtures retain the authenticated signed Unit-interface checkpoint."""
    root = checkpoint_595(root)
    extension = json.loads(MAP_VALUE_EXTENSION.read_text())
    try:
        source = (root / inventory.map_values.OWNER).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == extension['owners'][inventory.map_values.OWNER]['current_sha256']:
        return inventory.map_values.checkpoint(root, extension)
    return root


def checkpoint_592(root):
    """Historical fixtures use the exact signed opaque Set retirement checkpoint."""
    root = checkpoint_593(root)
    extension = json.loads(UNIT_INTERFACE_EXTENSION.read_text())
    try:
        source = (root / inventory.unit_interface.OWNER).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == extension['owners'][inventory.unit_interface.OWNER]['current_sha256']:
        return inventory.unit_interface.checkpoint(root, extension)
    return root


def checkpoint_591(root):
    """Historical fixtures use the authenticated signed guarded unsigned-product checkpoint."""
    root = checkpoint_592(root)
    transition = json.loads(OPAQUE_SET_TRANSITION.read_text())
    try:
        source = (root / inventory.RECORDED).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.RECORDED]['current_sha256']:
        return inventory.opaque_set.checkpoint(root, transition)
    return root


def checkpoint_590(root):
    """Historical fixtures use the authenticated signed final-organizer checkpoint."""
    root = checkpoint_591(root)
    transition = json.loads(UNSIGNED_PRODUCT_TRANSITION.read_text())
    try:
        source = (root / inventory.RECORDED).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.RECORDED]['current_sha256']:
        return inventory.unsigned_product.checkpoint(root, transition)
    return root


def checkpoint_589(root):
    """Historical fixtures use the authenticated signed witness-Merkle checkpoint."""
    root = checkpoint_590(root)
    transition = json.loads(ORGANIZER_TRANSITION.read_text())
    try:
        source = (root / inventory.SELECTOR).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.SELECTOR]['current_sha256']:
        return inventory.organizer.checkpoint(root, transition)
    return root


def checkpoint_588(root):
    """Historical fixtures use the authenticated signed enum-advance checkpoint."""
    root = checkpoint_589(root)
    transition = json.loads(WITNESS_MERKLE_TRANSITION.read_text())
    try:
        source = (root / inventory.SELECTOR).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.SELECTOR]['current_sha256']:
        return inventory.witness_merkle.checkpoint(root, transition)
    return root


def checkpoint_587(root):
    """Historical fixtures use the authenticated signed optional-write checkpoint."""
    root = checkpoint_588(root)
    transition = json.loads(ENUM_ADVANCE_TRANSITION.read_text())
    try:
        source = (root / inventory.SELECTOR).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.SELECTOR]['current_sha256']:
        return inventory.enum_advance.checkpoint(root, transition)
    return root


def checkpoint_580(root):
    """Historical fixtures use the authenticated signed Unit checkpoint."""
    root = checkpoint_587(root)
    transition = json.loads(OPTIONAL_WRITE_TRANSITION.read_text())
    try:
        source = (root / inventory.SELECTOR).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.SELECTOR]['current_sha256']:
        return inventory.optional_write.checkpoint(root, transition)
    return root


def checkpoint_586(root):
    """Historical fixtures reconstruct the authenticated pre-retirement source."""
    root = checkpoint_580(root)
    transition = json.loads(UNIT_COMPOSITION_TRANSITION.read_text())
    try:
        source = (root / inventory.SELECTOR).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(source) == transition['owners'][inventory.SELECTOR]['current_sha256']:
        return inventory.unit_composition.checkpoint(root, transition)
    return root


def checkpoint_585(root):
    root = checkpoint_586(root)
    owner = next(iter(inventory.value_test.OWNERS))
    try:
        current = (root / owner).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(current) == inventory.value_test.OWNERS[owner][1]:
        return inventory.value_test.checkpoint(root, json.loads(VALUE_TEST_EXTENSION.read_text()),
                                               json.loads(NEGATION_TEST_EXTENSION.read_text()))
    return root


def checkpoint_584(root):
    root = checkpoint_585(root)
    owner = next(iter(inventory.negation_test.OWNERS))
    try:
        current = (root / owner).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(current) == inventory.negation_test.OWNERS[owner][1]:
        return inventory.negation_test.checkpoint(root, json.loads(NEGATION_TEST_EXTENSION.read_text()),
                                                  json.loads(ENCODING_EXTENSION.read_text()))
    return root


def checkpoint_582(root):
    root = checkpoint_584(root)
    # Historical fixtures may already contain the exact preceding bytes.
    try:
        current = (root / inventory.encoding.OWNER).read_bytes()
    except FileNotFoundError:
        return root
    if inventory.digest(current) == inventory.encoding.CURRENT_SHA:
        source = inventory.encoding.predecessor_source(root, json.loads(ENCODING_EXTENSION.read_text()))
        return inventory.encoding._EncodingCheckpoint(root, source)
    return root


def checkpoint_581(root):
    """Only reconstruct the fully authenticated current #582 snapshot."""
    root = checkpoint_582(root)
    owner = inequality_test.OWNER
    if inventory.digest((root / owner).read_bytes()) == inventory.aligned_test.OWNERS[owner][1]:
        return inventory.aligned_test.checkpoint(root, json.loads(ALIGNED_TEST_EXTENSION.read_text()),
                                               json.loads(KECCAK_TEST_EXTENSION.read_text()))
    return root


def checkpoint_bytes(owner):
    """Historical test fixtures use authenticated pre-581 test snapshots only."""
    source = (checkpoint_581(inventory.ROOT) / owner).read_bytes()
    if owner not in inventory.keccak_test.OWNERS:
        return source
    snapshots = inventory.keccak_test.predecessor_sources(
        checkpoint_581(inventory.ROOT), json.loads(KECCAK_TEST_EXTENSION.read_text()))
    return snapshots[owner].encode()


def inequality_checkpoint_source(root):
    root = checkpoint_581(root)
    source = (root / inequality_test.OWNER).read_text()
    if inventory.digest(source.encode()) == inventory.keccak_test.OWNERS[inequality_test.OWNER][1]:
        snapshots = inventory.keccak_test.predecessor_sources(
            root, json.loads(KECCAK_TEST_EXTENSION.read_text()))
        source = snapshots[inequality_test.OWNER]
    if inventory.digest(source.encode()) in {inequality_test.PRIOR_SHA, routing_test.PRIOR_SHA}:
        return source
    return inequality_test.predecessor_source(source, json.loads(INEQUALITY_TEST_EXTENSION.read_text()))


def routing_checkpoint_source(root):
    source = inequality_checkpoint_source(root)
    # Historical fixtures may already contain the exact restored checkpoint.
    if inventory.digest(source.encode()) == routing_test.PRIOR_SHA:
        return source
    return routing_test.predecessor_source(source, json.loads(ROUTING_TEST_EXTENSION.read_text()))


RESET_PAYOUT_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-569.json")


SHIELDED_PAYOUT_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-570.json")


FUNDED_MINT_TRANSITION = inventory.BASELINE.with_name("recording_admission_inventory.transition-572.json")


def funded_mint_checkpoint_sources(root):
    root = checkpoint_581(root)
    return funded_mint.predecessor_sources(root, json.loads(FUNDED_MINT_TRANSITION.read_text()))


def shielded_payout_checkpoint_sources(root):
    root = checkpoint_581(root)
    prior = funded_mint_checkpoint_sources(root)
    sources = {owner: (root / owner).read_text() for owner in shielded_payout.REVIEWED_OWNERS}
    return {**prior, **shielded_payout.predecessor_source_snapshot(
        {**sources, **prior}, json.loads(SHIELDED_PAYOUT_TRANSITION.read_text()))}


def reset_payout_checkpoint_sources(root):
    root = checkpoint_581(root)
    prior = shielded_payout_checkpoint_sources(root)
    sources = {owner: (root / owner).read_text() for owner in reset_payout.REVIEWED_OWNERS}
    return {**prior, **reset_payout.predecessor_source_snapshot(
        {**sources, **prior}, json.loads(RESET_PAYOUT_TRANSITION.read_text()))}


def guarded_checkpoint_sources(root):
    root = checkpoint_581(root)
    prior = reset_payout_checkpoint_sources(root)
    sources = {owner: (root / owner).read_text() for owner in guarded.REVIEWED_OWNERS}
    return {routing_test.OWNER: routing_checkpoint_source(root), **prior,
            **guarded.predecessor_source_snapshot(
                {**sources, **prior}, json.loads(GUARDED_TRANSITION.read_text()))}


def merge_checkpoint_sources(root):
    root = checkpoint_581(root)
    prior = guarded_checkpoint_sources(root)
    sources = {owner: (root / owner).read_text() for owner in merge.REVIEWED_OWNERS}
    return {**prior, **merge.predecessor_source_snapshot(
        {**sources, **prior}, json.loads(MERGE_TRANSITION.read_text()))}


def send_checkpoint_sources(root):
    root = checkpoint_581(root)
    prior = merge_checkpoint_sources(root)
    sources = {owner: (root / owner).read_text() for owner in send.REVIEWED_OWNERS}
    return {**prior, **send.predecessor_source_snapshot(
        {**sources, **prior}, json.loads(SEND_TRANSITION.read_text()))}


def membership_checkpoint_sources(root):
    root = checkpoint_581(root)
    prior = send_checkpoint_sources(root)
    return {**prior, **membership.predecessor_source_snapshot(
        prior, json.loads(MEMBERSHIP_TRANSITION.read_text()))}


def zswap_checkpoint_sources(root):
    root = checkpoint_581(root)
    prior = membership_checkpoint_sources(root)
    sources = {owner: (root / owner).read_text() for owner in zswap.REVIEWED_OWNERS}
    return {**prior, **zswap.predecessor_source_snapshot(
        {**sources, **prior}, json.loads(ZSWAP_TRANSITION.read_text()))}


def immediate_checkpoint_sources(root):
    root = checkpoint_581(root)
    prior = zswap_checkpoint_sources(root)
    sources = {owner: (root / owner).read_text() for owner in immediate.REVIEWED_OWNERS}
    sources.update(prior)
    return {**prior, **immediate.predecessor_source_snapshot(sources, json.loads(IMMEDIATE_TRANSITION.read_text()))}


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
        instrumentation = json.loads(inventory.INSTRUMENTATION.read_text())
        report = inventory.verify(inventory.ROOT, baseline,
                                  transition=transition, baseline_bytes=baseline_bytes,
                                  instrumentation=instrumentation,
                                  jubjub_transition=json.loads(JUBJUB_TRANSITION.read_text()),
                                  call_extension=json.loads(CALL_EXTENSION.read_text()),
                                  terminal_transition=json.loads(TERMINAL_TRANSITION.read_text()),
                                  unit_actions_transition=json.loads(UNIT_TRANSITION.read_text()),
                                  context_query_transition=json.loads(CONTEXT_TRANSITION.read_text()),
                                  kernel_transition=json.loads(KERNEL_TRANSITION.read_text()),
                                  composite_transition=json.loads(COMPOSITE_TRANSITION.read_text()),
                                  receive_transition=json.loads(RECEIVE_TRANSITION.read_text()),
                                  unit_domain_extension=json.loads(UNIT_DOMAIN_EXTENSION.read_text()),
                                  voting_transition=json.loads(VOTING_TRANSITION.read_text()),
                                  phase_reset_transition=json.loads(PHASE_TRANSITION.read_text()),
                                  immediate_send_transition=json.loads(IMMEDIATE_TRANSITION.read_text()),
                                  zswap_transition=json.loads(ZSWAP_TRANSITION.read_text()),
                                  membership_transition=json.loads(MEMBERSHIP_TRANSITION.read_text()),
                                  send_transition=json.loads(SEND_TRANSITION.read_text()),
                                  observation_test_extension=json.loads(OBSERVATION_TEST_EXTENSION.read_text()),
                                  merge_transition=json.loads(MERGE_TRANSITION.read_text()),
                                  guarded_deposit_transition=json.loads(GUARDED_TRANSITION.read_text()),
                                  routing_test_extension=json.loads(ROUTING_TEST_EXTENSION.read_text()),
                                  reset_payout_transition=json.loads(RESET_PAYOUT_TRANSITION.read_text()),
                                  shielded_payout_transition=json.loads(SHIELDED_PAYOUT_TRANSITION.read_text()),
                                  inequality_test_extension=json.loads(INEQUALITY_TEST_EXTENSION.read_text()),
                                  funded_mint_transition=json.loads(FUNDED_MINT_TRANSITION.read_text()),
                                  keccak_test_extension=json.loads(KECCAK_TEST_EXTENSION.read_text()),
                                  aligned_test_extension=json.loads(ALIGNED_TEST_EXTENSION.read_text()),
                                  encoding_extension=json.loads(ENCODING_EXTENSION.read_text()),
                                  negation_test_extension=json.loads(NEGATION_TEST_EXTENSION.read_text()),
                                  value_test_extension=json.loads(VALUE_TEST_EXTENSION.read_text()),
                                  unit_composition_transition=json.loads(UNIT_COMPOSITION_TRANSITION.read_text()),
                                  optional_write_transition=json.loads(OPTIONAL_WRITE_TRANSITION.read_text()),
                                  enum_advance_transition=json.loads(ENUM_ADVANCE_TRANSITION.read_text()),
                                  witness_merkle_transition=json.loads(WITNESS_MERKLE_TRANSITION.read_text()),
                                  organizer_transition=json.loads(ORGANIZER_TRANSITION.read_text()),
                                  unsigned_product_transition=json.loads(UNSIGNED_PRODUCT_TRANSITION.read_text()),
                                  opaque_set_transition=json.loads(OPAQUE_SET_TRANSITION.read_text()),
                                  unit_interface_extension=json.loads(UNIT_INTERFACE_EXTENSION.read_text()),
                                  map_value_extension=json.loads(MAP_VALUE_EXTENSION.read_text()),
                                  guarded_map_transition=json.loads(GUARDED_MAP_TRANSITION.read_text()),
                                  opaque_map_transition=json.loads(OPAQUE_MAP_TRANSITION.read_text()))
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
        self.instrumentation = json.loads(inventory.INSTRUMENTATION.read_text())
        self.jubjub_transition = json.loads(JUBJUB_TRANSITION.read_text())
        self.call_extension = json.loads(CALL_EXTENSION.read_text())
        self.terminal_transition = json.loads(TERMINAL_TRANSITION.read_text())
        self.unit_transition = json.loads(UNIT_TRANSITION.read_text())
        self.context_transition = json.loads(CONTEXT_TRANSITION.read_text())
        self.kernel_transition = json.loads(KERNEL_TRANSITION.read_text())
        self.composite_transition = json.loads(COMPOSITE_TRANSITION.read_text())
        self.receive_transition = json.loads(RECEIVE_TRANSITION.read_text())
        self.unit_domain_extension = json.loads(UNIT_DOMAIN_EXTENSION.read_text())
        for relative in {*self.baseline['source_fingerprints'], inventory.ORACLE, jubjub.ORACLE, terminal.ORACLE, unit.ORACLE, context.ORACLE, context.IR, composite.ORACLE, composite.FIELD, receive.ORACLE, *context.SHARED_ANCHORS, *inventory.HOOK_SOURCES, *immediate.REVIEWED_OWNERS, *membership.REVIEWED_OWNERS, *merge.REVIEWED_OWNERS, *guarded.REVIEWED_OWNERS, *reset_payout.REVIEWED_OWNERS, *shielded_payout.REVIEWED_OWNERS, *funded_mint.REVIEWED_OWNERS}:
            destination = self.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(checkpoint_bytes(relative))
        (self.root / observation_test.OWNER).write_text(observation_checkpoint_source(inventory.ROOT))
        # Keep every historical mutation suite on its original 8-family/12-call
        # checkpoint. Reconstruction authenticates both complete source owners;
        # neither Git availability nor refreshed hashes can rewrite the fixture.
        for owner, source in immediate_checkpoint_sources(self.root).items():
            (self.root / owner).write_text(source)
        phase_review = json.loads(PHASE_TRANSITION.read_text())
        for owner, source in phase_reset.predecessor_sources(self.root, phase_review).items():
            (self.root / owner).write_text(source)
        voting_review = json.loads(VOTING_TRANSITION.read_text())
        selector = self.root / voting.SELECTOR
        selector.write_text(voting.predecessor_selector(selector.read_text(), voting_review))
        typed = self.root / voting.TYPED
        typed.write_text(voting.predecessor_typed(typed.read_text()))

    def report(self):
        return inventory.verify(self.root, self.baseline, transition=self.transition,
                                baseline_bytes=self.baseline_bytes,
                                instrumentation=self.instrumentation,
                                jubjub_transition=self.jubjub_transition, call_extension=self.call_extension,
                                terminal_transition=self.terminal_transition, unit_actions_transition=self.unit_transition,
                                context_query_transition=self.context_transition, kernel_transition=self.kernel_transition,
                                composite_transition=self.composite_transition, receive_transition=self.receive_transition,
                                unit_domain_extension=self.unit_domain_extension)

    def test_reviewed_retirement_keeps_frozen_denominator(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([row['id'] for row in report['retired_routes']], receive.RETIRED)
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
        path.write_text(zswap_checkpoint_sources(inventory.ROOT)[inventory.RECORDED])
        selector = self.root / inventory.SELECTOR
        selector.write_text(selector.read_text() + '\nfn new_unreviewed_gate() {}\n')
        self.assertFalse(self.report()['passed'])

    def test_transition_cannot_retire_another_route(self):
        self.transition['retired_route'] = 'selector.zswap'
        self.assertFalse(self.report()['passed'])


    def test_observation_review_is_explicit_and_does_not_change_denominator(self):
        report = inventory.verify(self.root, self.baseline, transition=self.transition,
                                  baseline_bytes=self.baseline_bytes)
        self.assertFalse(report['passed'])
        self.assertTrue(any('main selector changed' in error for error in report['errors']))
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 8)
        self.assertEqual(report['reviewed_instrumentation']['issue'], 515)
        self.assertNotIn('coverage', report['reviewed_instrumentation'])

    def test_changed_observation_owner_fails_without_hiding_retirement_errors(self):
        path = self.root / inventory.RECORDED
        path.write_text(path.read_text() + '\n// unreviewed change\n')
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertTrue(any('observation owner changed' in error for error in report['errors']))
        self.assertTrue(report['retired_routes'][0]['passed'])

    def test_observation_manifest_cannot_authorize_reordered_selector_calls(self):
        calls = self.instrumentation['reviewed_selector']['ordered_calls']
        calls[0], calls[1] = calls[1], calls[0]
        self.assertIn('instrumentation: selector calls or order changed', self.report()['errors'])

    def test_reviewed_source_digest_cannot_hide_new_selector_call(self):
        path = self.root / inventory.SELECTOR
        path.write_text(path.read_text() + '\nfn accidental() { typed_plan::lower_new_route(); }\n')
        self.instrumentation['reviewed_source_fingerprints'][inventory.SELECTOR] = inventory.digest(path.read_bytes())
        report = self.report()
        self.assertFalse(report['passed'])
        self.assertTrue(any('main selector changed' in error for error in report['errors']))

    def test_observation_original_selector_binding_cannot_change(self):
        self.instrumentation['baseline_selector']['function_sha256'] = '0' * 64
        self.assertFalse(self.report()['passed'])

    def test_observation_source_set_is_exact_and_missing_owner_fails(self):
        original = copy.deepcopy(self.instrumentation)
        self.instrumentation['reviewed_source_fingerprints'].pop(inventory.RECORDED)
        self.assertFalse(self.report()['passed'])
        self.instrumentation = original
        (self.root / inventory.HOOK_SOURCES[-1]).unlink()
        self.assertFalse(self.report()['passed'])

    def test_observation_ids_must_match_actual_hooks(self):
        self.instrumentation['instrumented_route_ids'].pop()
        self.assertIn('instrumentation: IDs differ from source observation hooks', self.report()['errors'])

    def test_cli_cannot_overwrite_observation_manifest(self):
        manifest = self.root / 'observations.json'
        manifest.write_text(json.dumps(self.instrumentation))
        before = manifest.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--instrumentation', str(manifest), '--output', str(manifest)])
        self.assertEqual(manifest.read_bytes(), before)


    def test_jubjub_retirement_is_explicit_and_preserves_original_hook_manifest(self):
        report = inventory.verify(self.root, self.baseline, transition=self.transition,
                                  baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation)
        self.assertFalse(report['passed'])
        self.assertEqual(self.instrumentation, json.loads(inventory.INSTRUMENTATION.read_text()))
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        self.assertEqual(current['retired_routes'][1]['id'], 'selector.jubjub-values')
        self.assertEqual(len(current['current_selector']['ordered_calls']), 12)
        self.assertIn('algebra.jubjub-values', current['reviewed_instrumentation']['instrumented_route_ids'])
        self.assertNotIn('selector.jubjub-values', current['reviewed_instrumentation']['instrumented_route_ids'])

    def test_jubjub_old_admission_call_cannot_return(self):
        path = self.root / inventory.SELECTOR
        path.write_text(path.read_text() + '\nfn accidental() { typed_plan :: lower_jubjub_values (); }\n')
        self.assertIn('Jubjub retirement: selector reintroduced old admission', self.report()['errors'])

    def test_jubjub_oracle_module_and_wrapper_stay_test_only(self):
        path = self.root / inventory.TYPED_PLAN
        original = path.read_text()
        for declaration in ['mod jubjub_reference;', "pub(super) fn lower_jubjub_values<'a>("]:
            with self.subTest(declaration=declaration):
                path.write_text(original.replace('#[cfg(test)]\n' + declaration, declaration))
                self.assertIn('Jubjub retirement: oracle module and wrapper must each be uniquely cfg(test)', self.report()['errors'])
        path.write_text(original)

    def test_jubjub_original_admission_body_is_immutable(self):
        path = self.root / jubjub.ORACLE
        path.write_text(path.read_text().replace('self.writes == 1', 'self.writes == 2'))
        self.assertIn('Jubjub retirement: oracle no longer equals original admission body', self.report()['errors'])

    def test_jubjub_shape_audit_cannot_return_to_value_adapter(self):
        path = self.root / jubjub.ORIGINAL
        path.write_text(path.read_text() + '\nstruct Audit;\n')
        self.assertIn('Jubjub retirement: shape admission remains in production value adapter', self.report()['errors'])

    def test_jubjub_transition_cannot_change_other_calls_or_hook_owners(self):
        original = copy.deepcopy(self.jubjub_transition)
        calls = self.jubjub_transition['reviewed_selector']['ordered_calls']
        calls[0], calls[1] = calls[1], calls[0]
        self.assertFalse(self.report()['passed'])
        self.jubjub_transition = original
        self.jubjub_transition['reviewed_source_fingerprints'][inventory.RECORDED] = '0' * 64
        self.assertFalse(self.report()['passed'])

    def test_jubjub_transition_binds_original_evidence(self):
        for key in ['baseline_file_sha256', 'original_source_sha256', 'prior_instrumentation_canonical_sha256']:
            with self.subTest(key=key):
                original = self.jubjub_transition[key]
                self.jubjub_transition[key] = '0' * 64
                self.assertFalse(self.report()['passed'])
                self.jubjub_transition[key] = original

    def test_jubjub_transition_cannot_retire_unreviewed_family(self):
        self.jubjub_transition['retired_route'] = 'selector.kernel'
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_jubjub_transition(self):
        transition = self.root / 'jubjub-transition.json'
        transition.write_text(json.dumps(self.jubjub_transition))
        before = transition.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--jubjub-transition', str(transition), '--output', str(transition)])
        self.assertEqual(transition.read_bytes(), before)


class CallExtensionTests(unittest.TestCase):
    setUp = RetirementTransitionTests.setUp
    report = RetirementTransitionTests.report

    def test_explicit_extension_passes_without_changing_routes_or_prior_evidence(self):
        before = {p: p.read_bytes() for p in (inventory.BASELINE, inventory.INSTRUMENTATION, JUBJUB_TRANSITION)}
        result = self.report()
        self.assertTrue(result['passed'], result['errors'])
        self.assertEqual(result['route_count'], 84)
        self.assertEqual([r['id'] for r in result['retired_routes']], receive.RETIRED)
        self.assertEqual(result['reviewed_call_extension']['new_retired_routes'], [])
        self.assertEqual(result['current_selector']['ordered_calls'], self.receive_transition['reviewed_selector']['ordered_calls'])
        self.assertEqual(result['reviewed_instrumentation']['instrumented_route_ids'], self.receive_transition['instrumented_route_ids'])
        self.assertEqual({p: p.read_bytes() for p in before}, before)

    def test_current_sources_require_explicit_call_extension(self):
        result = inventory.verify(self.root, self.baseline, transition=self.transition,
                                  baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
                                  jubjub_transition=self.jubjub_transition)
        self.assertFalse(result['passed'])
        self.assertNotIn('reviewed_call_extension', result)
        self.assertTrue(any('observation owner changed' in e for e in result['errors']))
        self.assertTrue(any('main selector changed' in e for e in result['errors']))

    def test_frozen_predecessor_hashes_and_three_original_owners_are_bound(self):
        for key in ('prior_effectful_transition_canonical_sha256',
                    'prior_instrumentation_canonical_sha256', 'prior_jubjub_transition_canonical_sha256'):
            with self.subTest(key=key):
                original = self.call_extension[key]
                self.call_extension[key] = '0' * 64
                self.assertTrue(any('frozen predecessor' in e for e in self.report()['errors']))
                self.call_extension[key] = original
        for owner in calls.SOURCES:
            with self.subTest(owner=owner):
                original = self.call_extension['prior_source_fingerprints'][owner]
                self.call_extension['prior_source_fingerprints'][owner] = '0' * 64
                self.assertTrue(any('prior source fingerprints' in e for e in self.report()['errors']))
                self.call_extension['prior_source_fingerprints'][owner] = original

    def test_predecessor_manifest_content_cannot_be_rewritten(self):
        self.jubjub_transition['claim'] += ' changed'
        self.assertTrue(any('prior_jubjub_transition' in e for e in self.report()['errors']))

    def test_selector_order_and_prior_selector_cannot_change(self):
        original = copy.deepcopy(self.call_extension)
        ordered = self.call_extension['reviewed_selector']['ordered_calls']
        ordered[0], ordered[1] = ordered[1], ordered[0]
        self.assertIn('call extension: legacy selector calls or order changed', self.report()['errors'])
        self.call_extension = original
        self.call_extension['prior_selector']['function_sha256'] = '0' * 64
        self.assertIn('call extension: prior selector is not bound to frozen evidence', self.report()['errors'])

    def test_current_hashes_are_checked_against_exact_source_and_selector(self):
        original = copy.deepcopy(self.call_extension)
        for owner in calls.SOURCES:
            with self.subTest(owner=owner):
                self.call_extension = copy.deepcopy(original)
                self.call_extension['reviewed_source_fingerprints'][owner] = '0' * 64
                self.assertTrue(any('observation owner changed' in e for e in self.report()['errors']))
        self.call_extension = original
        self.call_extension['reviewed_selector']['function_sha256'] = '0' * 64
        self.assertTrue(any('main selector changed' in e for e in self.report()['errors']))

    def test_extension_cannot_authorize_new_owners_or_observation_ids(self):
        original = copy.deepcopy(self.call_extension)
        self.call_extension['reviewed_source_fingerprints'][inventory.HOOK_SOURCES[-1]] = '0' * 64
        self.assertFalse(self.report()['passed'])
        self.call_extension = original
        self.call_extension['instrumented_route_ids'].append('selector.unreviewed')
        self.assertIn('call extension: observation IDs changed', self.report()['errors'])

    def test_unreviewed_source_drift_remains_fatal(self):
        for relative in (*sorted(calls.SOURCES), inventory.HOOK_SOURCES[-1]):
            with self.subTest(relative=relative):
                path = self.root / relative
                original = path.read_bytes()
                path.write_bytes(original + b'\n// unreviewed drift\n')
                self.assertTrue(any('observation owner changed' in e for e in self.report()['errors']))
                path.write_bytes(original)

    def test_selector_reordering_cannot_be_hidden_by_updated_digests(self):
        path = self.root / inventory.SELECTOR
        source = path.read_text().replace('typed_plan::lower_voting_commit(', '__swap__(')
        source = source.replace('typed_plan::lower_funded_mint(', 'typed_plan::lower_voting_commit(')
        source = source.replace('__swap__(', 'typed_plan::lower_funded_mint(')
        path.write_text(source)
        self.call_extension['reviewed_source_fingerprints'][inventory.SELECTOR] = inventory.digest(path.read_bytes())
        self.call_extension['reviewed_selector'] = inventory.selector_signature(source)
        self.assertIn('call extension: legacy selector calls or order changed', self.report()['errors'])

    def test_retired_ids_cannot_be_removed_reordered_or_added(self):
        original = copy.deepcopy(self.call_extension)
        for ids in (calls.RETIRED[:-1], list(reversed(calls.RETIRED)), calls.RETIRED + ['selector.kernel']):
            self.call_extension = copy.deepcopy(original)
            self.call_extension['retired_route_ids'] = ids
            self.assertIn('call extension: existing retirements changed or new retirement claimed', self.report()['errors'])
        self.call_extension = original
        self.call_extension['new_retired_routes'] = ['selector.terminal-returns']
        self.assertFalse(self.report()['passed'])

    def test_call_extension_needs_all_predecessors(self):
        for key in ('transition', 'instrumentation', 'jubjub_transition'):
            args = dict(transition=self.transition, instrumentation=self.instrumentation,
                        jubjub_transition=self.jubjub_transition)
            args[key] = None
            result = inventory.verify(self.root, self.baseline, baseline_bytes=self.baseline_bytes,
                                      call_extension=self.call_extension, **args)
            self.assertFalse(result['passed'])
            self.assertNotIn('reviewed_call_extension', result)

    def test_cli_preserves_call_manifest_and_requires_explicit_flag(self):
        manifest = self.root / 'calls.json'
        manifest.write_text(json.dumps(self.call_extension))
        before = manifest.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--call-extension', str(manifest), '--output', str(manifest)])
        self.assertEqual(manifest.read_bytes(), before)
        output = self.root / 'report.json'
        argv = ['--root', str(self.root), '--transition', str(inventory.BASELINE.with_name('recording_admission_inventory.transition-511.json')),
                '--instrumentation', str(inventory.INSTRUMENTATION), '--jubjub-transition', str(JUBJUB_TRANSITION),
                '--output', str(output)]
        self.assertEqual(inventory.main(argv), 1)
        self.assertEqual(inventory.main([*argv, '--call-extension', str(manifest), '--terminal-transition', str(TERMINAL_TRANSITION), '--unit-actions-transition', str(UNIT_TRANSITION), '--context-query-transition', str(CONTEXT_TRANSITION), '--kernel-transition', str(KERNEL_TRANSITION), '--composite-transition', str(COMPOSITE_TRANSITION), '--receive-transition', str(RECEIVE_TRANSITION), '--unit-domain-extension', str(UNIT_DOMAIN_EXTENSION)]), 0)
        self.assertTrue(json.loads(output.read_text())['passed'])
        self.assertEqual(manifest.read_bytes(), before)


class TerminalTransitionTests(unittest.TestCase):
    setUp = RetirementTransitionTests.setUp
    report = RetirementTransitionTests.report

    def test_explicit_transition_retires_only_terminal_and_keeps_denominator(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual([r['id'] for r in report['retired_routes']], receive.RETIRED)
        self.assertTrue(all(r['passed'] for r in report['retired_routes']))
        self.assertEqual(len(report['current_selector']['ordered_calls']), 12)
        ids = report['reviewed_instrumentation']['instrumented_route_ids']
        self.assertIn(terminal.REPLACEMENT, ids)
        self.assertNotIn(terminal.ROUTE, ids)
        without = inventory.verify(self.root, self.baseline, transition=self.transition,
                                   baseline_bytes=self.baseline_bytes, instrumentation=self.instrumentation,
                                   jubjub_transition=self.jubjub_transition, call_extension=self.call_extension)
        self.assertFalse(without['passed'])
        self.assertEqual(len(without['retired_routes']), 2)

    def test_prior_manifest_and_original_source_bindings_are_required(self):
        for key in ('baseline_file_sha256', 'original_source_sha256',
                    'prior_call_extension_canonical_sha256'):
            with self.subTest(key=key):
                original = self.terminal_transition[key]
                self.terminal_transition[key] = '0' * 64
                self.assertFalse(self.report()['passed'])
                self.terminal_transition[key] = original
        for owner in terminal.SOURCES:
            original = self.terminal_transition['prior_source_fingerprints'][owner]
            self.terminal_transition['prior_source_fingerprints'][owner] = '0' * 64
            self.assertFalse(self.report()['passed'])
            self.terminal_transition['prior_source_fingerprints'][owner] = original

    def test_predecessor_cannot_be_rewritten(self):
        self.call_extension['claim'] = 'edited historical evidence'
        self.assertIn('terminal retirement: frozen call extension changed or unavailable', self.report()['errors'])

    def test_selector_call_reintroduction_fails_even_with_whitespace(self):
        path = self.root / inventory.SELECTOR
        path.write_text(path.read_text() + '\nfn accidental() { typed_plan :: lower_terminal_returns (); }\n')
        self.assertIn('terminal retirement: selector reintroduced old admission', self.report()['errors'])

    def test_oracle_module_and_wrapper_remain_test_only(self):
        path = self.root / inventory.TYPED_PLAN
        original = path.read_text()
        for declaration in ('mod terminal_returns_reference;', "pub(super) fn lower_terminal_returns<'a>("):
            with self.subTest(declaration=declaration):
                changed = original.replace('#[cfg(test)]\n' + declaration, declaration)
                self.assertNotEqual(original, changed)
                path.write_text(changed)
                self.assertIn('terminal retirement: oracle module and wrapper must each be uniquely cfg(test)', self.report()['errors'])
        path.write_text(original)

    def test_original_oracle_and_retained_adapter_are_both_pinned(self):
        for relative, find, replacement, expected in (
            (terminal.ORACLE, 'Type::Field | Type::Boolean', 'Type::Field', 'oracle no longer equals original admission body'),
            (terminal.ORIGINAL, 'prefix.is_empty()', 'false', 'retained lexical adapter differs from original'),
        ):
            with self.subTest(relative=relative):
                path = self.root / relative
                original = path.read_text()
                self.assertIn(find, original)
                path.write_text(original.replace(find, replacement, 1))
                self.assertIn('terminal retirement: ' + expected, self.report()['errors'])
                path.write_text(original)

    def test_shape_audit_cannot_return_to_production_adapter(self):
        path = self.root / terminal.ORIGINAL
        path.write_text(path.read_text() + '\nstruct Audit;\n')
        self.assertIn('terminal retirement: shape admission remains in production adapter', self.report()['errors'])

    def test_body_fingerprints_cannot_approve_a_different_oracle(self):
        self.terminal_transition['oracle_body_sha256'] = '0' * 64
        self.assertIn('terminal retirement: oracle must preserve original admission body exactly', self.report()['errors'])

    def test_unrelated_selector_reordering_is_rejected_even_with_updated_hashes(self):
        path = self.root / inventory.SELECTOR
        original = path.read_text()
        source = original.replace('typed_plan::lower_voting_commit(', '__swap__(')
        source = source.replace('typed_plan::lower_funded_mint(', 'typed_plan::lower_voting_commit(')
        source = source.replace('__swap__(', 'typed_plan::lower_funded_mint(')
        self.assertNotEqual(original, source)
        path.write_text(source)
        self.terminal_transition['reviewed_source_fingerprints'][inventory.SELECTOR] = inventory.digest(path.read_bytes())
        self.terminal_transition['reviewed_selector'] = inventory.selector_signature(source)
        self.assertIn('terminal retirement: unrelated selector calls/order changed', self.report()['errors'])

    def test_source_drift_other_owner_and_hook_ids_cannot_be_approved(self):
        original = copy.deepcopy(self.terminal_transition)
        self.terminal_transition['reviewed_source_fingerprints'][inventory.RECORDED] = '0' * 64
        self.assertFalse(self.report()['passed'])
        self.terminal_transition = original
        self.terminal_transition['instrumented_route_ids'].append('algebra.unreviewed')
        self.assertIn('terminal retirement: unrelated observation IDs changed', self.report()['errors'])
        self.terminal_transition = json.loads(TERMINAL_TRANSITION.read_text())
        for owner in terminal.SOURCES:
            path = self.root / owner
            original = path.read_bytes()
            path.write_bytes(original + b'\n// unreviewed edit\n')
            self.assertTrue(any('observation owner changed' in error for error in self.report()['errors']))
            path.write_bytes(original)

    def test_retirement_set_cannot_expand_or_forget_prior_retirements(self):
        original = copy.deepcopy(self.terminal_transition)
        for ids in (terminal.RETIRED[:-1], terminal.RETIRED[1:], terminal.RETIRED + ['selector.kernel']):
            self.terminal_transition = copy.deepcopy(original)
            self.terminal_transition['retired_route_ids'] = ids
            self.assertIn('terminal retirement: unreviewed retirement set', self.report()['errors'])
        self.terminal_transition = original
        self.terminal_transition['new_retired_routes'] = ['selector.kernel']
        self.assertFalse(self.report()['passed'])

    def test_cli_cannot_overwrite_terminal_evidence(self):
        manifest = self.root / 'terminal.json'
        manifest.write_text(json.dumps(self.terminal_transition))
        before = manifest.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--terminal-transition', str(manifest), '--output', str(manifest)])
        self.assertEqual(before, manifest.read_bytes())


if __name__ == '__main__':
    unittest.main()
