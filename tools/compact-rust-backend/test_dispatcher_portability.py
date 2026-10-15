"""External-only portable prerequisite laws; root owns execution."""
import copy
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import archived_conformance_test_support as support
import historical_law_dispatcher as dispatcher


class PortabilityTests(unittest.TestCase):
    def descriptor(self):
        return dispatcher.load_descriptor(dispatcher.ROOT)

    def test_relocated_archive_and_graph_bundles_preserve_all_authorities(self):
        descriptor = self.descriptor()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root/'relocated-source'
            graph = root/'relocated-graph'
            for relative in descriptor['archived_conformance']['old_source_fingerprints']:
                destination = archive/relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(dispatcher.require_archive_root()/relative, destination)
            authority = descriptor['guarded_point_artifacts']
            for relative in (*authority['artifact_fingerprints'], *authority['authority_fingerprints']):
                destination = graph/relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(dispatcher.require_graph_root()/relative, destination)
            originals = support.authenticate_archive(dispatcher.require_archive_root(), descriptor['archived_conformance']['old_source_fingerprints'])
            self.assertEqual(support.authenticate_archive(archive, descriptor['archived_conformance']['old_source_fingerprints']), originals)
            receipt = dispatcher.prepare_guarded_point_artifacts(dispatcher.ROOT, descriptor, graph)
            self.assertEqual(receipt['root'], str(graph.resolve()))
            actual = dispatcher.ROOT/'tools/compact-rust-backend/src/lib.rs'
            with support.archived_conformance_reads(dispatcher.ROOT, descriptor, archive):
                self.assertEqual(actual.read_bytes(), originals['tools/compact-rust-backend/src/lib.rs'])
            damaged = graph/'authority/emission-report.json'
            damaged.write_bytes(damaged.read_bytes()+b'changed')
            with self.assertRaisesRegex(ValueError, 'retained authority drift'):
                dispatcher.prepare_guarded_point_artifacts(dispatcher.ROOT, descriptor, graph)

    def test_missing_archive_refuses_before_any_test_import_and_restores_root_selection(self):
        previous = dispatcher.ROOT, dispatcher.ARCHIVE_ROOT, dispatcher.GRAPH_ROOT
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)/'failure.json'
            with mock.patch.object(dispatcher.importlib, 'import_module') as imports:
                status = dispatcher.main(['--root', str(dispatcher.ROOT),
                    '--guarded-point-artifacts', str(dispatcher.require_graph_root()),
                    '--archived-conformance-root', str(Path(directory)/'absent'), '--output', str(output)])
            imports.assert_not_called()
            self.assertEqual(status, 1)
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt['phase'], 'fixture-preflight')
            self.assertFalse(receipt['test_execution_started'])
        self.assertEqual((dispatcher.ROOT, dispatcher.ARCHIVE_ROOT, dispatcher.GRAPH_ROOT), previous)

    def test_descriptor_contains_only_relative_configuration_and_same_source_guards(self):
        descriptor = self.descriptor()
        self.assertEqual(descriptor['schema_version'], 2)
        self.assertNotIn('archive_root', descriptor['archived_conformance'])
        self.assertNotIn('archive_artifact_paths', descriptor['guarded_point_artifacts'])
        baseline = json.loads((Path(__file__).parent/'historical_law_suite.json').read_text())
        for name in ('actual_source_fingerprints','checkpoint_stages','earlier_view_anchors',
                     'historical_test_method_authority'):
            self.assertEqual(descriptor[name], baseline[name])
        added = {'test_dispatcher_portability', 'test_reviewed_rust_capture',
                 'test_reducer_capture_source', 'test_historical_law_gate',
                 'test_historical_law_source_transition'}
        for group, previous in baseline['groups'].items():
            self.assertEqual([name for name in descriptor['groups'][group] if name not in added], previous)
        self.assertEqual(set(descriptor['groups']['ordinary']) - set(baseline['groups']['ordinary']), added)
        for name in ('old_source_fingerprints','case_id','archive_commit','expected_current_errors','authority_fingerprints'):
            self.assertEqual(descriptor['archived_conformance'][name], baseline['archived_conformance'][name])
        expected = baseline['guarded_point_artifacts']['authority_fingerprints']
        self.assertEqual({Path(k).name:v for k,v in expected.items()},
                         {Path(k).name:v for k,v in descriptor['guarded_point_artifacts']['authority_fingerprints'].items()})
        self.assertEqual(len(descriptor['actual_source_fingerprints']), 932)
        self.assertEqual(len(descriptor['archived_conformance']['old_source_fingerprints']), 150)
        for name in descriptor['guarded_point_artifacts']['authority_fingerprints']:
            self.assertFalse(Path(name).is_absolute())
            self.assertNotIn('..', Path(name).parts)

    def test_archive_flag_is_mandatory_and_escape_cannot_select_external_authority(self):
        with tempfile.TemporaryDirectory() as directory, mock.patch.object(sys, 'stderr', io.StringIO()):
            with self.assertRaises(SystemExit):
                dispatcher.main(['--root', str(dispatcher.ROOT), '--guarded-point-artifacts', directory,
                                 '--output', str(Path(directory)/'receipt.json')])
            with self.assertRaisesRegex(ValueError, 'escapes root'):
                dispatcher.graph_path(Path(directory), '../unreviewed.json')

    def test_actual_cli_without_fixture_environment_uses_one_canonical_module(self):
        candidate = Path(dispatcher.__file__).resolve()
        env = dict(os.environ)
        for name in ('COMPACT_HISTORICAL_LAW_ROOT', 'COMPACT_ARCHIVED_CONFORMANCE_ROOT',
                     'COMPACT_GUARDED_POINT_ROOT'):
            env.pop(name, None)
        env['PYTHONDONTWRITEBYTECODE'] = '1'
        env['PYTHONPATH'] = os.pathsep.join((str(candidate.parent), str(dispatcher.ROOT/dispatcher.BACKEND)))
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)/'prepared.json'
            command = [sys.executable, str(candidate), '--root', str(dispatcher.ROOT),
                       '--archived-conformance-root', str(dispatcher.require_archive_root()),
                       '--guarded-point-artifacts', str(dispatcher.require_graph_root()),
                       '--prepare-only', '--output', str(output)]
            result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=120)
            self.assertEqual(result.returncode, 0, result.stdout+result.stderr)
            receipt = json.loads(output.read_text())
            self.assertEqual(receipt['status'], 'prepared')
            self.assertFalse(receipt['test_execution'])
            self.assertFalse(receipt['full_suite_passed'])
            self.assertEqual(receipt['canonical_dispatcher'], {
                'same_module': True, 'root': str(dispatcher.ROOT),
                'archived_conformance_root': str(dispatcher.require_archive_root()),
                'guarded_point_artifacts': str(dispatcher.require_graph_root()),
            })
            self.assertEqual(receipt['planned_case_count'],
                             sum(len(cases) for cases in receipt['planned_case_ids'].values()))
            self.assertEqual(receipt['planned_case_ids']['ordinary'].count(
                self.descriptor()['archived_conformance']['case_id']), 1)

    def test_cli_refuses_foreign_canonical_module_without_overwrite(self):
        candidate = Path(dispatcher.__file__).resolve()
        code = ('import runpy,sys,types; '
                'sys.modules["historical_law_dispatcher"] = types.ModuleType("foreign"); '
                'runpy.run_path(sys.argv[1], run_name="__main__")')
        result = subprocess.run([sys.executable, '-c', code, str(candidate)],
                                capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('refusing to overwrite foreign historical_law_dispatcher', result.stderr)


if __name__ == '__main__':
    unittest.main()
