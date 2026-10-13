# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Complete crypto ownership, exact helpers and qualified historical guards."""
import contextlib
import copy
import io
import json
from pathlib import Path
import unittest
from unittest import mock
import recording_admission_inventory as inventory
import recording_admission_inventory_shared_aggregate as shared_aggregate
from recording_admission_inventory_shared_aggregate_test_support import install_historical_reads as install_aggregate_reads
import recording_admission_inventory_shared_crypto as crypto
import recording_admission_inventory_shared_arithmetic as arithmetic
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
from recording_admission_inventory_shared_crypto_test_support import (
    install_historical_reads, qualified_read_data,
)
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.shared-crypto-622d2.json')


class SharedCryptoTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        self.root = _Overlay(shared_aggregate.latest_checkpoint(inventory.ROOT, inventory.BASELINE))
        self.bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.bytes)
        self.previous = {k: json.loads((inventory.ROOT / p).read_text())
                         for k, p in self.extension['predecessor_options'].items()}
        self.predecessor = crypto.predecessor_sources(self.root, self.extension)
        install_aggregate_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.bytes,
                                **{**self.previous, 'shared_crypto_extension': self.extension, **overrides})

    def args(self):
        args = ['--shared-crypto-extension', str(MANIFEST)]
        for k, p in self.extension['predecessor_options'].items():
            args.extend(['--' + k.replace('_', '-'), str(inventory.ROOT / p)])
        return args

    def mutation(self, owner, before, after, diagnostic):
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count(before), 1)
        self.root.changes[owner] = source.replace(before, after, 1).encode()
        with self.assertRaisesRegex(ValueError, diagnostic):
            crypto.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()

    def test_actual_prior_authorities_counts_complete_family_and_pending_body_scope(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(crypto.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[key], prior[key])
        for key in prior:
            if key.startswith('reviewed_'):
                self.assertEqual(current[key], prior[key])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        stage = current['reviewed_shared_crypto_extension']
        self.assertEqual((stage['historical_manifest_count'], stage['original_oracles']), (74, 49))
        self.assertEqual(stage['slice'], 'D2')
        closure = stage['production_closure']
        self.assertEqual(len(closure['unique_source_variants']), 16)
        self.assertIn('pending622', closure['body_syntax_bridge'])
        self.assertIn('native-only', closure['keccak_qualification'])
        for key in ('reviewed_shared_arithmetic_extension', 'reviewed_shared_conversion_extension',
                    'reviewed_shared_leaf_extension', 'reviewed_unit_fragment_extension',
                    'reviewed_shared_coercion_extension'):
            forged = {**prior, key: {}}
            report = crypto.verify_extension(self.root, self.baseline, self.extension,
                                              lambda _: forged, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('actual D1c/D1b/D1a/C/B', report['errors'][0])
        for key in ('shared_arithmetic_extension', 'shared_conversion_extension', 'shared_leaf_extension',
                    'unit_fragment_extension', 'shared_coercion_extension', 'shared_crypto_extension'):
            self.assertFalse(self.report(**{key: None})['passed'])

    def test_all_current_inverse_shared_history_original49_and_selector_sources_reject_drift(self):
        paths = {crypto.SELECTOR}
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

    def test_forged_partial_inverse_owners_history_scope_and_retirement_cannot_claim_delivery(self):
        for kind in ('inverse', 'owner', 'history', 'scope', 'retirement'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][crypto.LIB]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'owner':
                forged['owners'].pop(crypto.HASH)
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            else:
                forged['claim'] = 'all Compact formally covered by complete CheckSink'
            self.assertFalse(self.report(shared_crypto_extension=forged)['passed'])
            with self.assertRaises(ValueError):
                crypto.checkpoint(self.root, forged)

    def test_current_adapter_aggregate_and_accepted_families_keep_specific_guard_attribution(self):
        self.mutation(crypto.LIB, 'fn pure_crypto_expression(',
                      'fn pure_crypto_expression(\n// changed adapter\n', 'current crypto adapter changed')
        self.mutation(crypto.LIB, 'fn pure_aggregate_expression(',
                      'fn pure_aggregate_expression(\n// changed family\n', 'unmigrated semantic family changed: aggregate')
        for name in ('pure_arithmetic_expression', 'pure_conversion_expression', 'pure_leaf_expression', 'pure_control_fragment'):
            anchor = 'fn ' + name + '('
            self.mutation(crypto.LIB, anchor, anchor + '\n// changed accepted owner\n',
                          'accepted semantic owner/entry changed: ' + name)
        self.mutation(crypto.TEST_MODULE, 'mod shared_crypto;',
                      'mod shared_crypto;\n// changed prior module', 'existing native test module changed')
        self.mutation(crypto.SINK, 'pub(crate) struct Typed<V> {',
                      'pub(crate) struct Typed<V> {\n    unchecked: bool,', 'accepted sink domain changed')

    def test_arity_child_order_opening_before_payload_and_neutral_rules_are_guarded(self):
        self.mutation(crypto.CRYPTO, 'let right = children.visit(right, sink)?;',
                      'let right = children.visit(left, sink)?;', 'shared crypto boundary changed')
        self.mutation(crypto.CRYPTO, 'pub(crate) enum CheckedCrypto<V>',
                      'fn bypass() { crate::expression_with_calls(); }\npub(crate) enum CheckedCrypto<V>',
                      'neutral crypto acquired native checking')
        self.mutation(crypto.RULES, 'expect(right.ty, Type::Field)?;',
                      'expect(left.ty, Type::Field)?;', 'shared crypto boundary changed')
        self.mutation(crypto.RULES, 'expect(right.ty, Type::Bytes { length: 32 })?;',
                      'expect(left.ty, Type::Bytes { length: 32 })?;', 'shared crypto boundary changed')
        self.mutation(crypto.RULES, 'check_tuple_traits(Operation::TransientHash, &value.ty)?;',
                      'check_tuple_traits(Operation::PersistentHash, &value.ty)?;', 'shared crypto boundary changed')

    def test_exact_opaque_owner_jubjub_order_and_output_only_helpers_cannot_recheck(self):
        self.mutation(crypto.ENCODING, 'Type::Vector { element, .. } => contains_opaque(element),',
                      'Type::Vector { element, .. } => false,', 'exact moved opaque predicate changed')
        self.mutation(crypto.HASH, 'runtime::exact_private_output(#value)',
                      'runtime::exact_hash_input(#value)', 'retained transcript helper changed: private_output')
        self.mutation(crypto.HASH, '    match encoding {',
                      '    check_tuple_traits();\n    match encoding {', 'hash renderer acquired semantic decisions')
        self.mutation(crypto.EXPECTATIONS, 'let scalar = expect(scalar, Type::Field)?;',
                      'let scalar = expect(scalar, Type::JubjubPoint)?;', 'shared crypto boundary changed')
        self.mutation(crypto.JUBJUB, '    Reduce(Operand),',
                      '    Reduce(Operand),\n    Unsupported(Operand),', 'retained legacy Jubjub API changed')
        self.mutation(crypto.FORMATTER, 'impl CryptoSink for NativeLeafSink {',
                      'fn recheck() { check_jubjub(); }\nimpl CryptoSink for NativeLeafSink {', 'native consumer acquired semantic decisions')
        reference = crypto.BASE + '/pure_native/tests/shared_crypto/old_hash_input.rs'
        self.mutation(reference, 'Type::Vector { element, .. } => contains_opaque(element),',
                      'Type::Vector { element, .. } => false,', 'literal complete helper oracle changed')

    def test_authenticated_historical_crypto_mutation_targets_original_guard_and_actual_drift_fails_closed(self):
        actual = {owner: (inventory.ROOT / owner).read_bytes() for owner in self.extension['owners']}
        for owner, data in qualified_read_data(self.root, self.extension).items():
            self.assertEqual(digest(data.encode()), self.extension['owners'][owner]['previous_sha256'])
        self.root.changes[crypto.LIB] = actual[crypto.LIB].decode().replace(
            'fn pure_crypto_expression(', 'fn pure_crypto_expression(\n// actual drift\n', 1).encode()
        with self.assertRaisesRegex(ValueError, 'current owner changed'):
            qualified_read_data(self.root, self.extension)
        self.root.changes.clear()
        forged = copy.deepcopy(self.extension)
        forged['owners'][crypto.LIB]['predecessor_reconstruction'][0]['current'] += '\n'
        with self.assertRaises(ValueError):
            qualified_read_data(self.root, forged)
        old_root = crypto.checkpoint(self.root, self.extension)
        old_stage = self.previous['shared_arithmetic_extension']
        old_sources = arithmetic.predecessor_sources(old_root, old_stage)
        case = unittest.TestCase()
        try:
            install_historical_reads(case, inventory.ROOT, inventory.BASELINE)
            for owner, row in self.extension['owners'].items():
                self.assertEqual(digest((inventory.ROOT / owner).read_bytes()), row['previous_sha256'])
            source = (inventory.ROOT / crypto.LIB).read_text()
            overlay = _Overlay(old_root)
            overlay.changes[crypto.LIB] = source.replace('fn pure_crypto_expression(',
                'fn pure_crypto_expression(\n// original crypto guard mutation\n', 1).encode()
            with self.assertRaisesRegex(ValueError, 'unmigrated semantic family changed: crypto'):
                arithmetic.source_closure(overlay, old_stage, old_sources)
        finally:
            case.doCleanups()
        for owner, data in actual.items():
            self.assertEqual((inventory.ROOT / owner).read_bytes(), data)

    def test_cli_transitively_protects_current_helpers_and_historical_authority_outputs(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (crypto.LIB, crypto.HASH, crypto.JUBJUB, crypto.CRYPTO, crypto.ENCODING,
                      crypto.EXPECTATIONS, crypto.FORMATTER,
                      'tools/compact-rust-backend/recording_admission_inventory_shared_crypto_test_support.py'):
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
