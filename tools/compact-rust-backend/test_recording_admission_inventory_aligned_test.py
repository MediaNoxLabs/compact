# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Five exact test adaptations cannot authorize source or historical drift."""
import contextlib
import io
import json
import unittest

import recording_admission_inventory as inventory
import recording_admission_inventory_aligned_test as aligned
import recording_admission_inventory_inequality_test as inequality
import test_recording_admission_inventory as previous
import test_recording_admission_inventory_funded_mint as prior_tests


class AlignedTestExtensionTests(unittest.TestCase):
    def setUp(self):
        prior_tests.FundedMintTransitionTests.setUp(self)
        self.aligned_extension = json.loads(previous.ALIGNED_TEST_EXTENSION.read_text())
        self.keccak_extension = json.loads(previous.KECCAK_TEST_EXTENSION.read_text())
        for owner in aligned.OWNERS:
            path = self.root / owner
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes((previous.checkpoint_584(inventory.ROOT) / owner).read_bytes())

    def report(self):
        return prior_tests.FundedMintTransitionTests.report(
            self, keccak_test_extension=self.keccak_extension, aligned_test_extension=self.aligned_extension)

    def test_exact_extension_preserves_all_routes_and_reconstructs_whole_owners(self):
        report = self.report()
        self.assertTrue(report['passed'], report['errors'])
        self.assertEqual(report['route_count'], 84)
        self.assertEqual(len(report['retired_routes']), 19)
        self.assertEqual(len(report['current_selector']['ordered_calls']), 1)
        self.assertEqual(report['reviewed_aligned_test_extension']['new_retired_routes'], [])
        snapshots = aligned.predecessor_sources(self.root, self.aligned_extension)
        for owner, source in snapshots.items():
            self.assertEqual(inventory.digest(source.encode()), aligned.OWNERS[owner][0])
        checkpoint = aligned.checkpoint(self.root, self.aligned_extension, self.keccak_extension)
        older = inventory.keccak_test.predecessor_sources(checkpoint, self.keccak_extension)
        old = inequality.predecessor_source(older[inequality.OWNER], self.inequality_extension)
        self.assertEqual(inventory.digest(old.encode()), inequality.PRIOR_SHA)
        with self.assertRaises(ValueError):
            inequality.predecessor_source((self.root / inequality.OWNER).read_text(), self.inequality_extension)

    def test_omitted_or_forged_extension_fails_closed(self):
        for replacement in [None, {**self.aligned_extension, 'claim': 'forged'},
                            {**self.aligned_extension, 'owners': {}},
                            {**self.aligned_extension, 'new_retired_routes': ['selector.unit-composition']}]:
            saved, self.aligned_extension = self.aligned_extension, replacement
            report = self.report()
            self.assertFalse(report['passed'])
            self.assertNotIn('reviewed_aligned_test_extension', report)
            self.aligned_extension = saved

    def test_each_current_owner_and_partial_rollback_are_authenticated(self):
        for owner in aligned.OWNERS:
            path = self.root / owner
            original = path.read_text()
            edit = self.aligned_extension['owners'][owner]['test_adaptations'][0]
            for changed in [original + '\n// drift\n', original.replace(edit['current'], edit['previous'], 1)]:
                self.assertNotEqual(original, changed)
                path.write_text(changed)
                self.assertFalse(self.report()['passed'], owner)
            path.write_text(original)

    def test_rehashed_source_or_reconstruction_cannot_create_new_authority(self):
        owner = next(iter(aligned.OWNERS))
        path = self.root / owner
        path.write_text(path.read_text() + '\n// unaudited\n')
        self.aligned_extension['owners'][owner]['current_sha256'] = inventory.digest(path.read_bytes())
        self.assertFalse(self.report()['passed'])
        with self.assertRaises(ValueError):
            aligned.predecessor_sources(self.root, self.aligned_extension)

    def test_immediate_predecessors_and_complete_older_chain_remain_required(self):
        for field in ['keccak_extension', 'funded_transition', 'inequality_extension', 'payout_transition',
                      'routing_extension', 'extension', 'transition']:
            original = getattr(self, field)
            for changed in [None, {**original, 'claim': 'unreviewed'}]:
                setattr(self, field, changed)
                report = self.report()
                self.assertFalse(report['passed'], field)
                self.assertNotIn('reviewed_aligned_test_extension', report)
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
            aligned._TestCheckpoint(self.root, {inventory.SELECTOR: 'forged'})

    def test_cli_cannot_overwrite_extension(self):
        before = previous.ALIGNED_TEST_EXTENSION.read_bytes()
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            inventory.main(['--aligned-test-extension', str(previous.ALIGNED_TEST_EXTENSION),
                            '--output', str(previous.ALIGNED_TEST_EXTENSION)])
        self.assertEqual(previous.ALIGNED_TEST_EXTENSION.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
