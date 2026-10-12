# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Three exact test adaptations cannot authorize source or historical drift."""
import contextlib
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_keccak_test as keccak
import recording_admission_inventory_inequality_test as inequality
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_funded_mint as prior_tests


class KeccakTestExtensionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.FundedMintTransitionTests.setUp(self)
        self.keccak_extension = json.loads(previous.KECCAK_TEST_EXTENSION.read_text())
        for owner in keccak.OWNERS:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((previous.checkpoint_581(inventory.ROOT) / owner).read_bytes())

    def report(self):
        return prior_tests.FundedMintTransitionTests.report(
            self, keccak_test_extension=self.keccak_extension)

    def test_exact_extension_preserves_all_routes_and_reconstructs_whole_owners(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 19)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 1)
        self.assertEqual(report['reviewed_keccak_test_extension']['new_retired_routes'], [])
        snapshots = keccak.predecessor_sources(self.root, self.keccak_extension)
        for owner, source in snapshots.items():
            self.assertEqual(inventory.digest(source.encode()), keccak.OWNERS[owner][0])
        old = inequality.predecessor_source(snapshots[inequality.OWNER], self.inequality_extension)
        self.assertEqual(inventory.digest(old.encode()), inequality.PRIOR_SHA)
        with self.assertRaises(ValueError):
            inequality.predecessor_source((self.root / inequality.OWNER).read_text(), self.inequality_extension)

    def test_omitted_or_forged_extension_fails_closed(self):
        for replacement in [None, {**self.keccak_extension, 'claim': 'forged'},
                            {**self.keccak_extension, 'owners': {}},
                            {**self.keccak_extension, 'new_retired_routes': ['selector.unit-composition']}]:
            saved, self.keccak_extension = self.keccak_extension, replacement
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertNotIn('reviewed_keccak_test_extension', report)
            self.keccak_extension = saved

    def test_each_current_owner_and_partial_rollback_are_authenticated(self):
        for owner in keccak.OWNERS:
            path = self.root / owner
            original = path.read_text()
            edit = self.keccak_extension['owners'][owner]['test_adaptations'][0]
            for changed in [original + '\n// drift\n', original.replace(edit['current'], edit['previous'], 1)]:
                self.assertNotEqual(original, changed)
                path.write_text(changed)
                self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_rehashed_source_or_reconstruction_cannot_create_new_authority(self):
        owner = next(iter(keccak.OWNERS))
        path = self.root / owner
        path.write_text(path.read_text() + '\n// unaudited\n')
        self.keccak_extension['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            keccak.predecessor_sources(self.root, self.keccak_extension)

    def test_immediate_predecessors_and_complete_older_chain_remain_required(self):
        for field in ['funded_transition', 'inequality_extension', 'payout_transition',
                      'routing_extension', 'extension', 'transition']:
            original = getattr(self, field)
            for changed in [None, {**original, 'claim': 'unreviewed'}]:
                setattr(self, field, changed)
                report = self.report()
                self.assertFalse(report['passed'], field)
                self.assertNotIn('reviewed_keccak_test_extension', report)
            setattr(self, field, original)

    def test_test_overlay_cannot_hide_production_or_frozen_oracle_changes(self):
        for owner in [inventory.SELECTOR, inventory.RECORDED,
                      next(iter(self.funded_transition['shared_sources_sha256']))]:
            path = self.root / owner
            original = path.read_text()
            path.write_text(original + '\n// changed production\n')
            self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)
        with self.assertRaises(ValueError):
            keccak._TestCheckpoint(self.root, {inventory.SELECTOR: 'forged'})

    def test_cli_cannot_overwrite_extension(self):
        before = previous.KECCAK_TEST_EXTENSION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--keccak-test-extension', str(previous.KECCAK_TEST_EXTENSION),
                            '--output', str(previous.KECCAK_TEST_EXTENSION)])
        self.assertEqual(previous.KECCAK_TEST_EXTENSION.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
