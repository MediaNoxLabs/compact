# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""One archived positive baseline, independently observed current epoch refusal."""
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import archived_conformance_test_support as support
import historical_law_dispatcher as dispatcher
import specification_conformance as conformance


class ArchivedConformanceEpochTests(unittest.TestCase):
    def descriptor(self):
        return dispatcher.load_descriptor(dispatcher.ROOT)

    def test_current_epoch_refuses_exact_retained_baseline_without_projection(self):
        descriptor = self.descriptor()
        baseline = conformance.load_json(Path(conformance.__file__).with_suffix('.json').read_text())
        result = conformance.check(dispatcher.ROOT, baseline['grammar_inventory'], baseline)
        self.assertEqual(result['status'], 'failed')
        self.assertEqual(result['errors'], descriptor['archived_conformance']['expected_current_errors'])
        self.assertEqual(len(result['errors']), 167)

    def test_only_exact_original_case_acquires_archived_view(self):
        from test_specification_conformance import RepositoryBaselineTests
        descriptor = self.descriptor()
        owner = dispatcher.ROOT / 'tools/compact-rust-backend/src/lib.rs'
        actual = owner.read_bytes()
        observations = []

        class CurrentCase(unittest.TestCase):
            def __init__(self, name):
                super().__init__()
                self.name = name
            def runTest(self):
                observations.append(owner.read_bytes())
            def id(self):
                return 'current_conformance.' + self.name

        original = RepositoryBaselineTests('test_checked_in_baseline_matches_current_sources_and_mappings')
        self.assertEqual(original.id(), descriptor['archived_conformance']['case_id'])
        before, after = CurrentCase('before'), CurrentCase('after')
        suites = {group: unittest.TestSuite() for group in dispatcher.GROUPS}
        suites['ordinary'] = unittest.TestSuite([before, original, after])
        identities = {group: [] for group in dispatcher.GROUPS}
        identities['ordinary'] = [case.id() for case in (before, original, after)]
        result = dispatcher.run_suites(dispatcher.ROOT, descriptor, suites, identities,
                                        verbosity=0, stream=io.StringIO())
        self.assertTrue(result['passed'], result)
        self.assertEqual(observations, [actual, actual])
        self.assertEqual([row['source_view'] for row in result['groups']['ordinary']['segments']],
                         ['actual', 'archived-conformance', 'ordinary'])
        self.assertEqual(owner.read_bytes(), actual)

    def test_archived_owner_drift_refuses_before_projection(self):
        descriptor = self.descriptor()
        archive = Path(descriptor['archived_conformance']['archive_root'])
        target = (archive / 'tools/compact-rust-backend/src/lib.rs').resolve()
        original = Path.read_bytes

        def changed(path):
            return original(path) + b'\n' if path.resolve() == target else original(path)

        with mock.patch.object(Path, 'read_bytes', changed):
            with self.assertRaisesRegex(ValueError, 'archived conformance input drift'):
                with support.archived_conformance_reads(dispatcher.ROOT, descriptor):
                    self.fail('changed archive must not acquire a view')

    def test_actual_owner_drift_is_observable_before_and_inside_projection(self):
        descriptor = self.descriptor()
        target = (dispatcher.ROOT / 'tools/compact-rust-backend/src/lib.rs').resolve()
        original = Path.read_bytes
        changes = {}

        def current(path):
            return changes.get(path.resolve(), original(path))

        with mock.patch.object(Path, 'read_bytes', current):
            changes[target] = original(target) + b'\n'
            with self.assertRaisesRegex(ValueError, 'actual input drift'):
                with support.archived_conformance_reads(dispatcher.ROOT, descriptor):
                    self.fail('changed current owner must not acquire a view')
            changes.clear()
            with self.assertRaisesRegex(ValueError, 'actual input drift'):
                with support.archived_conformance_reads(dispatcher.ROOT, descriptor):
                    changes[target] = original(target) + b'\n'
                    with self.assertRaisesRegex(ValueError, 'actual input drift'):
                        target.read_bytes()
            changes.clear()
            self.assertEqual(target.read_bytes(), original(target))

    def test_temp_reads_remain_live_and_exception_cleanup_restores_current_source(self):
        descriptor = self.descriptor()
        target = dispatcher.ROOT / 'tools/compact-rust-backend/src/lib.rs'
        actual = target.read_bytes()
        with tempfile.TemporaryDirectory() as directory:
            mutation = Path(directory) / 'owner.rs'
            mutation.write_bytes(b'changed temporary owner')
            with self.assertRaisesRegex(RuntimeError, 'deliberate archived failure'):
                with support.archived_conformance_reads(dispatcher.ROOT, descriptor):
                    self.assertNotEqual(target.read_bytes(), actual)
                    self.assertEqual(mutation.read_bytes(), b'changed temporary owner')
                    mutation.write_bytes(b'changed again')
                    self.assertEqual(mutation.read_bytes(), b'changed again')
                    raise RuntimeError('deliberate archived failure')
        self.assertEqual(target.read_bytes(), actual)


if __name__ == '__main__':
    unittest.main()
