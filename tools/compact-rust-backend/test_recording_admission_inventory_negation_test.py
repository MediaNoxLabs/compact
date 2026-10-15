# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""EcNeg test evolution preserves authenticated historical refusal evidence."""
import contextlib
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_negation_test as negation
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_encoding as encoding_tests
import test_recording_admission_inventory_funded_mint as prior_tests


class NegationTestExtensionTests(unittest.TestCase):
    def setUp(self):
        encoding_tests.EncodingExtensionTests.setUp(self)
        self.negation_extension = json.loads(previous.NEGATION_TEST_EXTENSION.read_text())
        for owner in negation.OWNERS:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((previous.checkpoint_585(inventory.ROOT) / owner).read_bytes())

    def report(self):
        return prior_tests.FundedMintTransitionTests.report(
            self, keccak_test_extension=self.keccak_extension,
            aligned_test_extension=self.aligned_extension,
            encoding_extension=self.encoding_extension,
            negation_test_extension=self.negation_extension)

    def test_exact_extension_restores_whole_test_owners_without_retirement(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 19)
        self.assertEqual(report['current_selector']['ordered_calls'], ['typed_plan::lower_unit_composition_checked'])
        self.assertEqual(report['reviewed_negation_test_extension']['new_retired_routes'], [])
        for owner, source in negation.predecessor_sources(self.root, self.negation_extension).items():
            self.assertEqual(inventory.digest(source.encode()), negation.OWNERS[owner][0])

    def test_missing_or_modified_manifest_or_predecessor_fails_closed(self):
        for field in ['negation_extension', 'encoding_extension', 'aligned_extension']:
            original = getattr(self, field)
            for replacement in [None, {**original, 'claim': 'forged'}]:
                setattr(self, field, replacement)
                self.assertFalse(self.report()['passed'], field)
            setattr(self, field, original)

    def test_current_owner_drift_and_partial_rollback_are_rejected(self):
        for owner in negation.OWNERS:
            path = self.root / owner
            original = path.read_text()
            edit = self.negation_extension['owners'][owner]['test_adaptations'][0]
            for replacement in [original + '\n// drift\n', original.replace(edit['current'], edit['previous'], 1)]:
                path.write_text(replacement)
                self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_rehashing_and_production_overlays_cannot_create_authority(self):
        owner = next(iter(negation.OWNERS))
        path = self.root / owner
        path.write_text(path.read_text() + '\n// unauthenticated\n')
        self.negation_extension['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            negation._TestCheckpoint(self.root, {inventory.SELECTOR: 'forged'})
        # The checkpoint never supplies production content.
        snapshots = negation.predecessor_sources(previous.checkpoint_585(inventory.ROOT), json.loads(previous.NEGATION_TEST_EXTENSION.read_text()))
        checkpoint = negation._TestCheckpoint(self.root, snapshots)
        self.assertEqual(checkpoint / inventory.SELECTOR, self.root / inventory.SELECTOR)

    def test_cli_cannot_overwrite_extension(self):
        before = previous.NEGATION_TEST_EXTENSION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--negation-test-extension', str(previous.NEGATION_TEST_EXTENSION),
                            '--output', str(previous.NEGATION_TEST_EXTENSION)])
        self.assertEqual(previous.NEGATION_TEST_EXTENSION.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
