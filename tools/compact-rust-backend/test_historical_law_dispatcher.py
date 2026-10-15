# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Discovery completeness, staged-source authority and observable mutation controls."""
import copy
import io
import json
from pathlib import Path
import sys
import tempfile
import types
import unittest
from unittest import mock

import historical_law_dispatcher as dispatcher


class DispatcherTests(unittest.TestCase):
    def descriptor(self):
        return dispatcher.load_descriptor(dispatcher.ROOT)

    def fixture_suites(self, observations, failing_group=None):
        recorded = dispatcher.ROOT / 'tools/compact-rust-backend/src/recorded.rs'
        checker = dispatcher.ROOT / 'tools/compact-rust-backend/src/recorded/effect_plan/pure_certification/checker.rs'
        modules, suites, identities = {}, {}, {}

        def fixtures(group):
            module = types.ModuleType('_dispatcher_fixture_' + group)

            def stamp(event):
                observations.append((group, event, recorded.read_bytes(), checker.read_bytes()))
                if group == failing_group and event in ('tearDownClass', 'tearDownModule'):
                    raise RuntimeError('deliberate ' + event + ' failure')

            module.setUpModule = lambda: stamp('setUpModule')
            module.tearDownModule = lambda: stamp('tearDownModule')
            case_type = type('Case', (unittest.TestCase,), {
                '__module__': module.__name__,
                'setUpClass': classmethod(lambda cls: stamp('setUpClass')),
                'tearDownClass': classmethod(lambda cls: stamp('tearDownClass')),
                'runTest': lambda self: stamp('case'),
            })
            return module, case_type()

        for group in dispatcher.GROUPS:
            module, case = fixtures(group)
            modules[module.__name__] = module
            suites[group] = unittest.TestSuite([case])
            identities[group] = [case.id()]
        return modules, suites, identities

    def test_real_module_partition_is_complete_and_forged_descriptor_refuses(self):
        descriptor = self.descriptor()
        discovered = {p.stem for p in (dispatcher.ROOT / dispatcher.BACKEND).glob('test_*.py')}
        groups = dispatcher.partition_modules(discovered, descriptor)
        self.assertEqual(set(discovered), {name for names in groups.values() for name in names})
        self.assertIn('test_fixture_inventory', groups['ordinary'])
        self.assertIn('test_source_consumer_gate', groups['ordinary'])
        self.assertIn('test_historical_law_dispatcher', groups['ordinary'])
        self.assertEqual(groups['current_measurement'], ['test_recording_admission_inventory_resource_measurement'])
        with tempfile.TemporaryDirectory() as directory:
            forged = copy.deepcopy(descriptor)
            forged['groups']['ordinary'].clear()
            path = Path(directory) / 'descriptor.json'
            path.write_text(json.dumps(forged))
            with self.assertRaisesRegex(ValueError, 'unreviewed historical law descriptor'):
                dispatcher.load_descriptor(dispatcher.ROOT, path)

    def test_partition_rejects_missing_extra_or_duplicate_module_assignments(self):
        descriptor = self.descriptor()
        discovered = {name for names in descriptor['groups'].values() for name in names}
        for mutation in ('missing', 'extra', 'duplicate'):
            candidate = copy.deepcopy(descriptor)
            if mutation == 'missing':
                candidate['groups']['ordinary'].pop()
            elif mutation == 'extra':
                candidate['groups']['ordinary'].append('test_unreviewed')
            else:
                candidate['groups']['ordinary'].append(candidate['groups']['earlier_inventory'][0])
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, 'partition differs|duplicate'):
                dispatcher.partition_modules(discovered, candidate)

    def test_loading_import_errors_and_duplicate_case_ids_are_fatal(self):
        class Case(unittest.TestCase):
            def runTest(self):
                pass
            def id(self):
                return 'test_first.Case.runTest'

        class Loader:
            errors = []
            def loadTestsFromNames(self, modules):
                return unittest.TestSuite([Case(), Case()]) if modules else unittest.TestSuite()

        descriptor = self.descriptor()
        with self.assertRaisesRegex(ValueError, 'duplicate discovered Python case'):
            dispatcher.load_suites(dispatcher.ROOT, descriptor, Loader())
        broken = Loader()
        broken.errors = ['ImportError: missing dependency']
        with self.assertRaisesRegex(ValueError, 'import/loading failed'):
            dispatcher.load_suites(dispatcher.ROOT, descriptor, broken)

    def test_real_earlier_checkpoint_preserves_read_only_source_and_overlay_drift(self):
        from test_recording_admission_inventory_generic_qualification import _Overlay
        import recording_admission_inventory_start_funding as funding
        descriptor = self.descriptor()
        root = dispatcher.ROOT
        owner = 'tools/compact-rust-backend/src/recorded.rs'
        actual = (root / owner).read_bytes()
        manifest = json.loads((root / dispatcher.BACKEND / 'recording_admission_inventory.start-funding-617.json').read_text())
        with dispatcher.earlier_suite_reads(root, descriptor):
            previous = (root / owner).read_bytes()
            self.assertNotEqual(previous, actual)
            self.assertEqual(dispatcher.digest(previous), descriptor['earlier_view_anchors'][owner])
            mutation = _Overlay(root)
            mutation.changes[owner] = previous + b'\n'
            with self.assertRaisesRegex(ValueError, 'changed|differs'):
                funding.predecessor_sources(mutation, manifest)
        self.assertEqual((root / owner).read_bytes(), actual)

    def test_changed_actual_owner_refuses_before_projection(self):
        descriptor = self.descriptor()
        target = (dispatcher.ROOT / 'tools/compact-rust-backend/src/recorded.rs').resolve()
        original = Path.read_bytes
        def changed(path):
            return original(path) + b'\n' if path.resolve() == target else original(path)
        with mock.patch.object(Path, 'read_bytes', changed):
            with self.assertRaisesRegex(ValueError, 'actual input drift'):
                dispatcher.prepare_earlier_view(dispatcher.ROOT, descriptor)

    def test_retired_bridge_resurrection_cannot_acquire_an_old_read_view(self):
        descriptor = self.descriptor()
        target = (dispatcher.ROOT / 'tools/compact-rust-backend/src/recorded/effect_plan/pure_certification/native.rs').resolve()
        self.assertIsNone(descriptor['actual_source_fingerprints'][str(target.relative_to(dispatcher.ROOT))])
        original = Path.read_bytes
        def resurrected(path):
            return b'' if path.resolve() == target else original(path)
        with mock.patch.object(Path, 'read_bytes', resurrected):
            with self.assertRaisesRegex(ValueError, 'actual input drift'):
                dispatcher.prepare_earlier_view(dispatcher.ROOT, descriptor)

    def test_actual_drift_inside_context_is_not_restored_and_cleanup_runs(self):
        descriptor = self.descriptor()
        target = (dispatcher.ROOT / 'tools/compact-rust-backend/src/recorded.rs').resolve()
        original = Path.read_bytes
        changes = {}
        def current(path):
            return changes.get(path.resolve(), original(path))
        with mock.patch.object(Path, 'read_bytes', current):
            with self.assertRaisesRegex(ValueError, 'actual input drift'):
                with dispatcher.earlier_suite_reads(dispatcher.ROOT, descriptor):
                    changes[target] = original(target) + b'\n'
                    with self.assertRaisesRegex(ValueError, 'projected actual owner drift'):
                        target.read_bytes()
            changes.clear()
            self.assertEqual(target.read_bytes(), original(target))

    def test_exception_cleanup_restores_actual_reads(self):
        descriptor = self.descriptor()
        target = dispatcher.ROOT / 'tools/compact-rust-backend/src/recorded.rs'
        original = target.read_bytes()
        with self.assertRaisesRegex(RuntimeError, 'deliberate failure'):
            with dispatcher.earlier_suite_reads(dispatcher.ROOT, descriptor):
                self.assertNotEqual(target.read_bytes(), original)
                raise RuntimeError('deliberate failure')
        self.assertEqual(target.read_bytes(), original)

    def test_phase_order_uses_old_views_only_for_assigned_groups(self):
        descriptor = self.descriptor()
        root = dispatcher.ROOT
        recorded = root / 'tools/compact-rust-backend/src/recorded.rs'
        checker = root / 'tools/compact-rust-backend/src/recorded/effect_plan/pure_certification/checker.rs'
        actual_recorded, actual_checker = recorded.read_bytes(), checker.read_bytes()
        observations = []
        class Case(unittest.TestCase):
            def __init__(self, group):
                super().__init__()
                self.group = group
            def runTest(self):
                observations.append((self.group, recorded.read_bytes(), checker.read_bytes()))
            def id(self):
                return self.group + '.case'
        suites = {group: unittest.TestSuite([Case(group)]) for group in dispatcher.GROUPS}
        identities = {group: [group + '.case'] for group in dispatcher.GROUPS}
        receipt = dispatcher.run_suites(root, descriptor, suites, identities, verbosity=0, stream=io.StringIO())
        self.assertTrue(receipt['passed'], receipt)
        self.assertEqual(receipt['tests_run'], 4)
        self.assertEqual([row[0] for row in observations], list(dispatcher.GROUPS))
        self.assertEqual(observations[0][1:], (actual_recorded, actual_checker))
        self.assertEqual(dispatcher.digest(observations[1][1]), descriptor['earlier_view_anchors']['tools/compact-rust-backend/src/recorded.rs'])
        self.assertEqual(observations[2][1], actual_recorded)
        self.assertNotEqual(observations[2][2], actual_checker)
        self.assertEqual(observations[3][1:], (actual_recorded, actual_checker))

    def test_skips_are_visible_and_cannot_produce_a_complete_gate_pass(self):
        descriptor = self.descriptor()
        class Skipped(unittest.TestCase):
            def runTest(self):
                self.skipTest('deliberately unavailable case')
        suites = {group: unittest.TestSuite() for group in dispatcher.GROUPS}
        case = Skipped()
        suites['current_measurement'].addTest(case)
        identities = {group: [] for group in dispatcher.GROUPS}
        identities['current_measurement'] = [case.id()]
        receipt = dispatcher.run_suites(dispatcher.ROOT, descriptor, suites, identities,
                                        verbosity=0, stream=io.StringIO())
        self.assertFalse(receipt['passed'])
        self.assertEqual(len(receipt['skipped']), 1)
        self.assertIn('deliberately unavailable', receipt['skipped'][0]['reason'])

    def test_class_and_module_fixtures_finish_inside_their_assigned_source_view(self):
        descriptor = self.descriptor()
        root = dispatcher.ROOT
        recorded = root / 'tools/compact-rust-backend/src/recorded.rs'
        checker = root / 'tools/compact-rust-backend/src/recorded/effect_plan/pure_certification/checker.rs'
        actual = (recorded.read_bytes(), checker.read_bytes())
        observations = []
        modules, suites, identities = self.fixture_suites(observations)
        with mock.patch.dict(sys.modules, modules):
            receipt = dispatcher.run_suites(root, descriptor, suites, identities,
                                            verbosity=0, stream=io.StringIO())
        self.assertTrue(receipt['passed'], receipt)
        self.assertTrue(receipt['actual_case_ids_match_plan'])
        for index, group in enumerate(dispatcher.GROUPS):
            rows = observations[index * 5:(index + 1) * 5]
            self.assertEqual([(row[0], row[1]) for row in rows], [(group, event) for event in
                             ('setUpModule', 'setUpClass', 'case', 'tearDownClass', 'tearDownModule')])
            self.assertTrue(all(row[2:] == rows[0][2:] for row in rows))
            self.assertEqual(receipt['groups'][group]['actual_case_ids'], identities[group])
            if group in ('ordinary', 'current_measurement'):
                self.assertEqual(rows[0][2:], actual)
            elif group == 'earlier_inventory':
                self.assertEqual(dispatcher.digest(rows[0][2]), descriptor['earlier_view_anchors']
                                 ['tools/compact-rust-backend/src/recorded.rs'])
            else:
                self.assertEqual(rows[0][2], actual[0])
                self.assertNotEqual(rows[0][3], actual[1])
        self.assertEqual((recorded.read_bytes(), checker.read_bytes()), actual)

    def test_failing_class_and_module_teardown_preserve_errors_and_restore_actual_reads(self):
        descriptor = self.descriptor()
        recorded = dispatcher.ROOT / 'tools/compact-rust-backend/src/recorded.rs'
        checker = dispatcher.ROOT / 'tools/compact-rust-backend/src/recorded/effect_plan/pure_certification/checker.rs'
        actual = (recorded.read_bytes(), checker.read_bytes())
        observations = []
        modules, suites, identities = self.fixture_suites(observations, failing_group='earlier_inventory')
        with mock.patch.dict(sys.modules, modules):
            receipt = dispatcher.run_suites(dispatcher.ROOT, descriptor, suites, identities,
                                            verbosity=0, stream=io.StringIO())
        self.assertFalse(receipt['passed'])
        self.assertEqual(receipt['tests_run'], 4)
        self.assertEqual(receipt['errors'], 2)
        errors = receipt['groups']['earlier_inventory']['errors']
        self.assertEqual(len(errors), 2)
        for event in ('tearDownClass', 'tearDownModule'):
            self.assertTrue(any(event in error['case'] and 'deliberate ' + event in error['diagnostic']
                                for error in errors), errors)
        self.assertEqual([row[0] for row in observations if row[1] == 'case'], list(dispatcher.GROUPS))
        self.assertTrue(all(row[2:] == actual for row in observations if row[0] == 'current_measurement'))
        self.assertEqual((recorded.read_bytes(), checker.read_bytes()), actual)

    def test_missing_guarded_artifacts_refuse_before_import_and_record_typed_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'failure.json'
            with mock.patch.object(dispatcher.importlib, 'import_module') as imports:
                status = dispatcher.main(['--guarded-point-artifacts', directory, '--output', str(output),
                                          '--archived-conformance-root', str(dispatcher.require_archive_root())])
            self.assertEqual(status, 1)
            imports.assert_not_called()
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt['status'], 'infrastructure-refused')
            self.assertEqual(receipt['phase'], 'fixture-preflight')
            self.assertFalse(receipt['test_execution_started'])

    def test_wrong_guarded_graph_is_not_accepted_as_an_artifact(self):
        descriptor = self.descriptor()
        with tempfile.TemporaryDirectory() as directory:
            artifacts = Path(directory)
            for relative in descriptor['guarded_point_artifacts']['artifact_fingerprints']:
                path = artifacts / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('{}')
            with self.assertRaisesRegex(ValueError, 'guarded point artifact drift'):
                dispatcher.prepare_guarded_point_artifacts(dispatcher.ROOT, descriptor, artifacts)

    def test_guarded_point_artifacts_argument_is_mandatory(self):
        with tempfile.TemporaryDirectory() as directory, mock.patch.object(sys, 'stderr', io.StringIO()):
            with self.assertRaises(SystemExit):
                dispatcher.main(['--output', str(Path(directory) / 'receipt.json')])

    def test_failed_group_stops_later_execution_with_explicit_unrun_identities(self):
        descriptor = self.descriptor()

        class Case(unittest.TestCase):
            def __init__(self, group):
                super().__init__()
                self.group = group
            def runTest(self):
                self.fail('deliberate group failure')
            def id(self):
                return self.group + '.case'

        suites = {group: unittest.TestSuite([Case(group)]) for group in dispatcher.GROUPS}
        identities = {group: [group + '.case'] for group in dispatcher.GROUPS}
        with mock.patch.object(dispatcher, 'earlier_suite_reads') as earlier:
            receipt = dispatcher.run_suites(dispatcher.ROOT, descriptor, suites, identities,
                                            verbosity=0, stream=io.StringIO(), stop_after_failed_group=True)
        earlier.assert_not_called()
        self.assertFalse(receipt['passed'])
        self.assertEqual(receipt['status'], 'failed-partial')
        self.assertEqual(receipt['tests_run'], 1)
        self.assertEqual(receipt['not_run_case_ids'], {group: identities[group] for group in dispatcher.GROUPS[1:]})

    def test_prepare_only_authenticates_inputs_without_executing_testcases(self):
        descriptor = self.descriptor()
        with tempfile.TemporaryDirectory() as directory:
            artifacts = Path(directory)
            authority = descriptor['guarded_point_artifacts']
            for relative in (*authority['artifact_fingerprints'], *authority['authority_fingerprints']):
                path = artifacts / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes((dispatcher.require_graph_root() / relative).read_bytes())
            output = artifacts / 'prepared.json'
            with mock.patch.object(dispatcher, 'run_suites') as runs:
                status = dispatcher.main(['--guarded-point-artifacts', str(artifacts), '--output', str(output),
                                          '--archived-conformance-root', str(dispatcher.require_archive_root()),
                                          '--prepare-only'])
            runs.assert_not_called()
            self.assertEqual(status, 0)
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt['status'], 'prepared')
            self.assertFalse(receipt['test_execution'])
            self.assertFalse(receipt['full_suite_passed'])
            self.assertEqual(receipt['checkpoint_stages'], 17)
            self.assertEqual(len(receipt['signed617_anchors']), 7)


if __name__ == '__main__':
    unittest.main()
