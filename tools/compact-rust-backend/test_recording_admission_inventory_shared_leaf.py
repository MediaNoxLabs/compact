# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Shared leaf ownership, immutable C history and narrowly projected mutations."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import recording_admission_inventory as inventory
import recording_admission_inventory_lexical_symbols as lexical_symbols
from recording_admission_inventory_lexical_symbols_test_support import install_historical_reads as install_symbol_reads
import recording_admission_inventory_shared_aggregate as shared_aggregate
from recording_admission_inventory_shared_aggregate_test_support import install_historical_reads as install_aggregate_reads
import recording_admission_inventory_shared_crypto as shared_crypto
from recording_admission_inventory_shared_crypto_test_support import install_historical_reads as install_crypto_reads
import recording_admission_inventory_shared_arithmetic as shared_arithmetic
from recording_admission_inventory_shared_arithmetic_test_support import install_historical_reads as install_arithmetic_reads
import recording_admission_inventory_shared_conversion as shared_conversion
from recording_admission_inventory_shared_conversion_test_support import install_historical_reads
import recording_admission_inventory_shared_leaf as leaf
import recording_admission_inventory_unit_fragment as unit
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
from recording_admission_inventory_shared_leaf_test_support import project_candidate
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.shared-leaf-622d1a.json')


class SharedLeafTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(shared_conversion.latest_checkpoint(
            shared_arithmetic.latest_checkpoint(
            shared_crypto.latest_checkpoint(
            shared_aggregate.latest_checkpoint(
            lexical_symbols.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE))
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = leaf.predecessor_sources(self.root, self.extension)
        install_symbol_reads(self, inventory.ROOT, inventory.BASELINE)
        install_aggregate_reads(self, inventory.ROOT, inventory.BASELINE)
        install_crypto_reads(self, inventory.ROOT, inventory.BASELINE)
        install_arithmetic_reads(self, inventory.ROOT, inventory.BASELINE)
        install_historical_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'shared_leaf_extension': self.extension, **overrides})

    def args(self):
        args = ['--shared-leaf-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args

    def mutation(self, owner, before, after, diagnostic):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaisesRegex(ValueError, diagnostic):
            leaf.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_actual_C_and_B_authorities_preserve_accounting_and_pending_body_scope(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(leaf.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for k in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[k], prior[k])
        for k in prior:
            if k.startswith('reviewed_'):
                self.assertEqual(current[k], prior[k])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_shared_leaf_extension']
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (71, 49))
        self.assertEqual(stage['slice'], 'D1a')
        self.assertIn('pending622', stage['production_closure']['body_syntax_bridge'])
        self.assertEqual(len(stage['production_closure']['preserved_semantic_functions']), 4)
        for key in ('reviewed_unit_fragment_extension', 'reviewed_shared_coercion_extension'):
            forged = {**prior, key: {}}
            report = leaf.verify_extension(self.root, self.baseline, self.extension,
                                           lambda _: forged, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('actual C/B', report['errors'][0])
        for key in ('unit_fragment_extension', 'shared_coercion_extension', 'shared_leaf_extension'):
            self.assertFalse(self.report(**{key: None})['passed'])

    def test_complete_owner_new_core_history_and_original49_authorities_reject_drift(self):
        paths = {leaf.SELECTOR}
        for key in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            paths.update(self.extension[key])
        original = json.loads((inventory.ROOT / self.extension['original_oracle_preservation']['authority']).read_text())['files']
        self.assertEqual(len(original), 49)
        paths.update(row['path'] for row in original)
        for owner in paths:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (inventory.ROOT / owner).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_missing_forged_partial_or_retiring_transition_cannot_claim_leaf_delivery(self):
        for kind in ('inverse', 'owner', 'history', 'retirement', 'scope'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][leaf.LIB]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'owner':
                forged['owners'].pop(leaf.TEST_MODULE)
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            else:
                forged['claim'] = 'complete syntax-free body compiler and epic done'
            self.assertFalse(self.report(shared_leaf_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                leaf.checkpoint(self.root, forged)

    def test_current_leaf_adapter_and_unmigrated_family_have_specific_guard_attribution(self):
        self.mutation(leaf.LIB, 'Expr::Parameter { name } => LeafSource::Parameter(name),',
                      'Expr::Parameter { name } => LeafSource::Unit,', 'current leaf adapter changed')
        for family in leaf.FAMILIES:
            anchor = 'fn pure_' + family + '_expression('
            self.mutation(leaf.LIB, anchor, anchor + '\n// changed family\n',
                          'unmigrated semantic family changed: ' + family)
        self.mutation(leaf.TEST_MODULE, 'mod shared_leaf;',
                      'mod shared_leaf;\n// existing test module changed', 'existing native test module changed')

    def test_neutral_facts_and_native_consumer_cannot_acquire_duplicate_checking(self):
        self.mutation(leaf.LEAF, 'pub(crate) enum LeafSource',
                      'fn hidden() { crate::pure_native::leaf::NativeLeafSink::new(); }\npub(crate) enum LeafSource',
                      'neutral semantic owner acquired native checking')
        self.mutation(leaf.FORMATTER, 'pub(crate) struct NativeLeafSink',
                      'fn recheck() { unsigned_maximum("0"); }\npub(crate) struct NativeLeafSink',
                      'native formatter acquired semantic checking')
        self.mutation(leaf.SINK, "pub(crate) struct UnsignedLiteralFacts<'a> {",
                      "pub(crate) struct UnsignedLiteralFacts<'a> {\n    unchecked: bool,", 'current source authority changed')
        self.mutation(leaf.FORMATTER, 'byte_literals(&facts.bytes()[..31])',
                      'byte_literals(&facts.bytes()[..32])', 'shared leaf boundary changed')

    def test_ownership_borrowed_source_keys_and_first_error_resource_order_are_owned_once(self):
        self.mutation(leaf.OWNERSHIP, 'elements.iter().all(copy_type)',
                      'elements.iter().any(copy_type)', 'original ownership policy changed')
        self.mutation(leaf.VALUE, '    crate::pure_semantics::ownership::copy_type(ty)',
                      '    true', 'ownership forwarding changed')
        self.mutation(leaf.SCOPE, 'HashMap::get(self, source_name).map(|(ty, binding)| (*ty, binding))',
                      'HashMap::get(self, "renamed").map(|(ty, binding)| (*ty, binding))', 'shared leaf boundary changed')
        self.mutation(leaf.LEAF, 'drop(recording_type_nodes(ty)?);',
                      '// no resource bound', 'shared leaf boundary changed')
        self.mutation(leaf.LEAF, 'let maximum = unsigned_maximum(max)?;',
                      'let maximum = unsigned_maximum(value)?;', 'shared leaf boundary changed')
        self.mutation(leaf.LEAF, 'let variant = check_name(variant)?;',
                      'let variant = check_name("replacement")?;', 'shared leaf boundary changed')

    def test_authenticated_candidate_projection_preserves_intended_C_guards_and_refuses_changed_leaf(self):
        for owner, row in self.extension['owners'].items():
            candidate = (inventory.ROOT / owner).read_bytes()
            self.assertEqual(digest(project_candidate(owner, candidate, self.extension)), row['previous_sha256'])
        forged = copy.deepcopy(self.extension)
        forged['owners'][leaf.LIB]['predecessor_reconstruction'][0]['current'] += '\n'
        with self.assertRaises(ValueError):
            project_candidate(leaf.LIB, (inventory.ROOT / leaf.LIB).read_bytes(), forged)
        actual = (inventory.ROOT / leaf.LIB).read_text()
        changed_leaf = actual.replace('LeafSource::Parameter(name)', 'LeafSource::Unit', 1).encode()
        with self.assertRaisesRegex(ValueError, 'current leaf mutated'):
            project_candidate(leaf.LIB, changed_leaf, self.extension)
        changed_import = actual.replace('use value_lowering::{copy_type, wide_uint_type};',
                                        'use value_lowering::{copy_type};', 1).encode()
        with self.assertRaisesRegex(ValueError, 'current inverse region changed'):
            project_candidate(leaf.LIB, changed_import, self.extension)
        old_root = leaf.checkpoint(inventory.ROOT, self.extension)
        c = self.previous['unit_fragment_extension']
        c_previous = unit.predecessor_sources(old_root, c)
        for before, after, diagnostic in (
            ('fn pure_crypto_expression(', 'fn pure_crypto_expression(\n// guard control\n', 'unmigrated semantic family changed: crypto'),
            ('native_fragment_with_calls(otherwise, parameters, circuits)?;', 'skip_otherwise_occurrence(otherwise)?;', 'native view/checking boundary changed')):
            candidate = actual.replace(before, after, 1).encode()
            projected = project_candidate(leaf.LIB, candidate, self.extension)
            self.assertIn(after.encode(), projected)
            self.assertEqual(function(projected.decode(), 'pure_leaf_expression'),
                             function(self.predecessor[leaf.LIB], 'pure_leaf_expression'))
            overlay = _Overlay(old_root)
            overlay.changes[leaf.LIB] = projected
            with self.assertRaisesRegex(ValueError, diagnostic):
                unit.source_closure(overlay, c, c_previous)

    def test_cli_protects_all_transitive_sources_oracles_helpers_and_historical_authorities(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (leaf.LIB, leaf.LEAF, leaf.SCOPE, leaf.FORMATTER,
                      'tools/compact-rust-backend/recording_admission_inventory_shared_leaf_test_support.py'):
            self.assertIn(inventory.ROOT / owner, paths)
        for path in paths:
            before = path.read_bytes()
            with self.subTest(owner=str(path)), mock.patch.object(
                    Path, 'write_text', side_effect=AssertionError('authority write attempted')) as write:
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    inventory.main([*self.args(), '--output', str(path)])
                write.assert_not_called()
            self.assertEqual(path.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
