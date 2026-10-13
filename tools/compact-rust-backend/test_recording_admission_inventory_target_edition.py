# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Fixed Rust2024 gen escaping, exact lexer history and no-write controls."""
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
import recording_admission_inventory_shared_leaf as shared_leaf
import recording_admission_inventory_unit_fragment as unit_fragment
import recording_admission_inventory_shared_coercion as shared_coercion
import recording_admission_inventory_target_edition as edition
from test_recording_admission_inventory_generic_qualification import _Overlay

MANIFEST = inventory.BASELINE.with_name('recording_admission_inventory.target-edition-626.json')


class TargetEditionTests(unittest.TestCase):
    def setUp(self):
        self.extension = json.loads(MANIFEST.read_text())
        latest_root = unit_fragment.latest_checkpoint(
            shared_leaf.latest_checkpoint(
            shared_conversion.latest_checkpoint(
            shared_arithmetic.latest_checkpoint(
            shared_crypto.latest_checkpoint(
            shared_aggregate.latest_checkpoint(
            lexical_symbols.latest_checkpoint(inventory.ROOT, inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE), inventory.BASELINE)
        self.source_root = shared_coercion.latest_checkpoint(latest_root, inventory.BASELINE)
        self.root = _Overlay(self.source_root)
        self.baseline_bytes = inventory.BASELINE.read_bytes()
        self.baseline = json.loads(self.baseline_bytes)
        self.previous = {key: json.loads((inventory.ROOT / path).read_text())
                         for key, path in self.extension['predecessor_options'].items()}
        self.predecessor = edition.predecessor_sources(self.root, self.extension)
        install_symbol_reads(self, inventory.ROOT, inventory.BASELINE)
        install_aggregate_reads(self, inventory.ROOT, inventory.BASELINE)
        install_crypto_reads(self, inventory.ROOT, inventory.BASELINE)
        install_arithmetic_reads(self, inventory.ROOT, inventory.BASELINE)
        install_historical_reads(self, inventory.ROOT, inventory.BASELINE)

    def report(self, **overrides):
        return inventory.verify(self.root, self.baseline, baseline_bytes=self.baseline_bytes,
                                **{**self.previous, 'target_edition_identifiers_extension': self.extension,
                                   **overrides})

    def cli_arguments(self):
        args = ['--target-edition-identifiers-extension', str(MANIFEST)]
        for key, path in self.extension['predecessor_options'].items():
            args.extend(['--' + key.replace('_', '-'), str(inventory.ROOT / path)])
        return args

    def test_exact_ed42_predecessor_retains_all_recording_counts_and_reports(self):
        current = self.report()
        self.assertTrue(current['passed'], current['errors'])
        prior = inventory.verify(edition.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.baseline_bytes, **self.previous)
        self.assertTrue(prior['passed'], prior['errors'])
        for key in ('retired_routes', 'route_checks', 'current_selector'):
            self.assertEqual(current[key], prior[key])
        for key in prior:
            if key.startswith('reviewed_'):
                self.assertEqual(current[key], prior[key])
        self.assertEqual((current['route_count'], len(current['retired_routes'])), (84, 69))
        summary = current['reviewed_target_edition_identifiers_extension']
        self.assertEqual((summary['historical_manifest_count'], summary['original_oracles']), (68, 49))
        closure = summary['production_closure']
        self.assertEqual((closure['target_edition'], closure['corrected_name'],
                          closure['corrected_output_spelling']), ('2024', 'gen', 'r#gen'))
        self.assertEqual(closure['new_retired_routes'], [])
        self.assertIn('pending', prior['reviewed_shared_pure_semantics_extension']['production_closure']['body_syntax_bridge'])

    def test_both_complete_owners_new_test_shared_scanner_and_entire_history_reject_drift(self):
        paths = {edition.SELECTOR}
        for label in ('owners', 'new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
            paths.update(self.extension[label])
        authority = self.extension['original_oracle_preservation']['authority']
        original = json.loads((inventory.ROOT / authority).read_text())['files']
        self.assertEqual(len(original), 49)
        paths.update(row['path'] for row in original)
        for owner in paths:
            with self.subTest(owner=owner):
                self.root.changes[owner] = (inventory.ROOT / owner).read_bytes() + b'\n'
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_missing_forged_or_incomplete_checkpoint_cannot_claim_a_broader_name_fix(self):
        for overrides in ({'target_edition_identifiers_extension': None},
                          {'shared_pure_semantics_extension': None}):
            self.assertFalse(self.report(**overrides)['passed'])
        for kind in ('inverse', 'retirement', 'history', 'scope'):
            forged = copy.deepcopy(self.extension)
            if kind == 'inverse':
                forged['owners'][edition.OWNER]['predecessor_reconstruction'][0]['previous'] += '\n'
            elif kind == 'retirement':
                forged['new_retired_route_ids'] = ['helper.transitive-admission']
            elif kind == 'history':
                forged['historical_manifests'].pop(next(iter(forged['historical_manifests'])))
            else:
                forged['claim'] = 'every target and all names fixed; full622 done'
            with self.subTest(kind=kind):
                self.assertFalse(self.report(target_edition_identifiers_extension=forged)['passed'])
                with self.assertRaises(ValueError):
                    edition.checkpoint(self.root, forged)

    def test_gen_escaping_must_follow_unchanged_acceptance_and_preserve_the_semantic_key(self):
        source = (inventory.ROOT / edition.OWNER).read_text()
        mutations = [
            ('raw: raw || text == "gen",', 'raw: raw || text == "async",'),
            ('raw: raw || text == "gen",', 'raw,'),
            ('text: text.to_owned(),', 'text: format!("r#{text}"),'),
            ('.or_else(|| parse_raw_retry(&normalized))', '.or_else(|| Some(("gen", false)))'),
            ('fn parse_identifier(input: &str) -> Option<(&str, bool)> {',
             'fn parse_identifier(input: &str) -> Option<(&str, bool)> {\n    // edited lexical acceptance'),
        ]
        for before, after in mutations:
            with self.subTest(anchor=before):
                self.assertEqual(source.count(before), 1)
                self.root.changes[edition.OWNER] = source.replace(before, after, 1).encode()
                with self.assertRaises(ValueError):
                    edition.source_closure(self.root, self.extension, self.predecessor)
                self.assertFalse(self.report()['passed'])
                self.root.changes.clear()

    def test_fixed_target_and_shared622A_predecessor_are_mandatory(self):
        owner = 'tools/compact-rust-backend/Cargo.toml'
        source = (inventory.ROOT / owner).read_text()
        self.assertEqual(source.count('edition = "2024"'), 1)
        self.root.changes[owner] = source.replace('edition = "2024"', 'edition = "2021"', 1).encode()
        with self.assertRaises(ValueError):
            edition.source_closure(self.root, self.extension, self.predecessor)
        self.assertFalse(self.report()['passed'])
        self.root.changes.clear()
        prior = inventory.verify(edition.checkpoint(self.root, self.extension), self.baseline,
                                 baseline_bytes=self.baseline_bytes, **self.previous)
        for stage in ({}, {'issue': 622, 'slice': 'B'}, {'issue': 626, 'slice': 'A'}):
            forged = {**prior, 'reviewed_shared_pure_semantics_extension': stage}
            report = edition.verify_extension(self.root, self.baseline, self.extension,
                                              lambda _: forged, inventory.selector_signature)
            self.assertFalse(report['passed'])
            self.assertIn('mandatory shared622A', report['errors'][0])

    def test_cli_protects_all_transitive_owners_targets_scanners_and_oracle_authorities(self):
        paths = inventory.protected_authority_closure(inventory.ROOT, [MANIFEST])
        for owner in (edition.OWNER, edition.TESTS, 'tools/compact-rust-backend/Cargo.toml'):
            self.assertIn(inventory.ROOT / owner, paths)
        for path in paths:
            before = path.read_bytes()
            with self.subTest(owner=str(path)), mock.patch.object(
                    Path, 'write_text', side_effect=AssertionError('authority write attempted')) as write:
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    inventory.main([*self.cli_arguments(), '--output', str(path)])
                write.assert_not_called()
            self.assertEqual(path.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
