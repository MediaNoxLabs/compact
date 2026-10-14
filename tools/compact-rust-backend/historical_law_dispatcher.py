# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Run all discovered Python laws with explicit authenticated historical views."""
import argparse
from contextlib import contextmanager, nullcontext
import hashlib
import importlib
import json
from pathlib import Path
import sys
import time
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
BACKEND = 'tools/compact-rust-backend'
DESCRIPTOR_SHA = 'f40a77cc304f3d4569f21e7a5882b7af428110e37520582f09d9bf4d8a7db47d'
GROUPS = ('ordinary', 'earlier_inventory', 'latest_inventory', 'current_measurement')
STAGES = ('resource_measurement', 'body_checker', 'native_control_fold', 'lexical_symbols',
          'shared_aggregate', 'shared_crypto', 'shared_arithmetic', 'shared_conversion',
          'shared_leaf', 'unit_fragment', 'shared_coercion', 'target_edition', 'shared_pure',
          'native_tuple', 'helper_discovery', 'generic_retirement', 'generic_qualification')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return digest(json.dumps(value, sort_keys=True, separators=(',', ':')).encode())


def authenticate_descriptor(descriptor):
    if (canonical(descriptor) != DESCRIPTOR_SHA or descriptor.get('schema_version') != 1
            or descriptor.get('issue') != 632
            or tuple(row['name'] for row in descriptor['checkpoint_stages']) != STAGES):
        raise ValueError('unreviewed historical law descriptor or stage order')


def load_descriptor(root, path=None):
    path = path or root / BACKEND / 'historical_law_suite.json'
    descriptor = json.loads(path.read_text())
    authenticate_descriptor(descriptor)
    return descriptor


def authenticate_sources(root, descriptor, read_bytes=None):
    """Authenticate actual files and absent paths, before and after any projection."""
    read_bytes = read_bytes or Path.read_bytes
    for relative, expected in descriptor['actual_source_fingerprints'].items():
        path = (root / relative).resolve()
        if not path.is_relative_to(root.resolve()):
            raise ValueError('historical authority escapes root: ' + relative)
        try:
            current = read_bytes(path)
        except FileNotFoundError:
            if expected is None:
                continue
            raise ValueError('historical actual input missing: ' + relative) from None
        if expected is None or digest(current) != expected:
            raise ValueError('historical actual input drift: ' + relative)


def prepare_earlier_view(root, descriptor):
    """Compose the reviewed checkpoint functions, never reconstruct owners ourselves."""
    authenticate_descriptor(descriptor)
    authenticate_sources(root, descriptor)
    view, snapshots = root, {}
    for row in descriptor['checkpoint_stages']:
        module = importlib.import_module('recording_admission_inventory_' + row['name'])
        manifest = json.loads((root / row['manifest']).read_text())
        view = module.checkpoint(view, manifest)
        snapshots.update(view._sources)
    for relative, expected in descriptor['earlier_view_anchors'].items():
        if digest((view / relative).read_bytes()) != expected:
            raise ValueError('authenticated earlier checkpoint anchor differs: ' + relative)
    return snapshots


@contextmanager
def earlier_suite_reads(root, descriptor):
    """Project exact old source only at real-root paths; mutation overlays stay live."""
    snapshots = prepare_earlier_view(root, descriptor)
    rows = {(root / relative).resolve(): source.encode() for relative, source in snapshots.items()}
    read_bytes, read_text = Path.read_bytes, Path.read_text
    expected = {(root / relative).resolve(): pin
                for relative, pin in descriptor['actual_source_fingerprints'].items()}

    def actual_owner(path):
        pin = expected[path]
        try:
            current = read_bytes(path)
        except FileNotFoundError:
            if pin is None:
                return
            raise ValueError('historical projected owner disappeared: ' + str(path)) from None
        if pin is None or digest(current) != pin:
            raise ValueError('historical projected actual owner drift: ' + str(path))

    if not rows.keys() <= expected.keys():
        raise ValueError('historical projection contains unauthenticated owners')

    def qualified_bytes(path):
        target = path.resolve()
        if target not in rows:
            return read_bytes(path)
        actual_owner(target)
        return rows[target]

    def qualified_text(path, *args, **kwargs):
        if path.resolve() not in rows:
            return read_text(path, *args, **kwargs)
        encoding = kwargs.get('encoding') or (args[0] if args else None) or 'utf-8'
        errors = kwargs.get('errors') or (args[1] if len(args) > 1 else None) or 'strict'
        return qualified_bytes(path).decode(encoding, errors)

    try:
        with mock.patch.object(Path, 'read_bytes', qualified_bytes), \
                mock.patch.object(Path, 'read_text', qualified_text):
            yield
    finally:
        # Entire actual closure remains authoritative, including nonprojected
        # sources and retired paths. No ordinary/current test inherits a view.
        authenticate_sources(root, descriptor, read_bytes)


def partition_modules(discovered, descriptor):
    groups = descriptor['groups']
    if set(groups) != set(GROUPS):
        raise ValueError('historical suite groups incomplete')
    assigned = [name for group in GROUPS for name in groups[group]]
    if len(assigned) != len(set(assigned)):
        raise ValueError('duplicate historical module assignment')
    if set(assigned) != set(discovered):
        raise ValueError('discovered Python module partition differs: missing=' +
                         repr(sorted(set(discovered) - set(assigned))) + ' extra=' +
                         repr(sorted(set(assigned) - set(discovered))))
    return groups


def flatten_cases(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from flatten_cases(item)
        else:
            yield item


def prepare_guarded_point_artifacts(root, descriptor, artifacts):
    """Require the authentic retained pair before importing or running tests."""
    authenticate_descriptor(descriptor)
    authenticate_sources(root, descriptor)
    authority = descriptor['guarded_point_artifacts']
    artifacts = artifacts.resolve()
    for relative, expected in authority['artifact_fingerprints'].items():
        path = (artifacts / relative).resolve()
        if not path.is_relative_to(artifacts):
            raise ValueError('guarded point artifact escapes root: ' + relative)
        if digest(path.read_bytes()) != expected:
            raise ValueError('guarded point artifact drift: ' + relative)
    for path, expected in authority['authority_fingerprints'].items():
        if digest(Path(path).read_bytes()) != expected:
            raise ValueError('guarded point retained authority drift: ' + path)
    source = root / authority['source']['path']
    if digest(source.read_bytes()) != authority['source']['sha256']:
        raise ValueError('guarded point current fixture drift')
    return {'root': str(artifacts), 'artifact_fingerprints': authority['artifact_fingerprints'],
            'source': authority['source'], 'producer_identities': authority['producer_identities'],
            'qualification': 'Retained source-bound graph mutation replay; not fresh emission or proof.'}


def load_suites(root, descriptor, loader=None):
    authenticate_descriptor(descriptor)
    authenticate_sources(root, descriptor)
    directory = root / BACKEND
    discovered = {path.stem for path in directory.glob('test_*.py')}
    groups = partition_modules(discovered, descriptor)
    loader = loader or unittest.TestLoader()
    suites, identities, seen = {}, {}, set()
    for group in GROUPS:
        suite = loader.loadTestsFromNames(groups[group])
        cases = list(flatten_cases(suite))
        if loader.errors or any(case.__class__.__name__ == '_FailedTest' for case in cases):
            raise ValueError('Python test import/loading failed: ' + repr(loader.errors))
        ids = [case.id() for case in cases]
        if len(ids) != len(set(ids)) or seen.intersection(ids):
            raise ValueError('duplicate discovered Python case identity')
        if any(not any(identity.startswith(module + '.') for module in groups[group])
               for identity in ids):
            raise ValueError('Python case belongs to an unassigned module')
        seen.update(ids)
        suites[group], identities[group] = suite, ids
    if identities['ordinary'].count(descriptor['archived_conformance']['case_id']) != 1:
        raise ValueError('archived conformance case identity must occur exactly once')
    return suites, identities


def run_suites(root, descriptor, suites, identities, *, verbosity=2, stream=None,
               stop_after_failed_group=False):
    authenticate_descriptor(descriptor)
    from recording_admission_inventory_resource_measurement_test_support import historical_suite_reads
    measurement = json.loads((root / descriptor['checkpoint_stages'][0]['manifest']).read_text())

    class IdentityResult(unittest.TextTestResult):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            self.started_case_ids = []

        def startTest(self, test):
            self.started_case_ids.append(test.id())
            super().startTest(test)

    contexts = {
        'ordinary': nullcontext,
        'earlier_inventory': lambda: earlier_suite_reads(root, descriptor),
        'latest_inventory': lambda: historical_suite_reads(root, measurement),
        'current_measurement': nullcontext,
    }
    authenticate_sources(root, descriptor)
    start = time.monotonic()
    from archived_conformance_test_support import archived_conformance_reads
    archived_case = descriptor['archived_conformance']['case_id']
    results, group_reports = [], {}
    for group in GROUPS:
        # A top-level runner finishes class/module fixtures before its source
        # context exits. Sharing one nested runner leaks fixture transitions
        # across the boundary between actual and historical source views.
        cases = list(flatten_cases(suites[group]))
        segments, ordinary_segment = [], []
        for case in cases:
            if case.id() == archived_case:
                if group != 'ordinary':
                    raise ValueError('archived conformance case belongs to wrong group')
                if ordinary_segment:
                    segments.append((ordinary_segment, contexts[group], 'actual'))
                    ordinary_segment = []
                segments.append(([case], lambda: archived_conformance_reads(root, descriptor),
                                 'archived-conformance'))
            else:
                ordinary_segment.append(case)
        if ordinary_segment or not segments:
            segments.append((ordinary_segment, contexts[group], group))
        group_results, segment_reports = [], []
        for selected, context, view in segments:
            with context():
                result = unittest.TextTestRunner(verbosity=verbosity, stream=stream,
                                                 resultclass=IdentityResult).run(unittest.TestSuite(selected))
            group_results.append(result)
            results.append(result)
            segment_reports.append({'source_view': view, 'actual_case_ids': result.started_case_ids})
        group_reports[group] = {
            'segments': segment_reports,
            'actual_case_ids': [identity for result in group_results for identity in result.started_case_ids],
            'tests_run': sum(result.testsRun for result in group_results),
            'failures': [{'case': case.id(), 'diagnostic': diagnostic}
                         for result in group_results for case, diagnostic in result.failures],
            'errors': [{'case': case.id(), 'diagnostic': diagnostic}
                       for result in group_results for case, diagnostic in result.errors],
            'skipped': [{'case': case.id(), 'reason': reason}
                        for result in group_results for case, reason in result.skipped],
        }
        if stop_after_failed_group and any(not result.wasSuccessful() or result.skipped
                                           for result in group_results):
            break
    authenticate_sources(root, descriptor)
    planned = sum(len(ids) for ids in identities.values())
    tests_run = sum(result.testsRun for result in results)
    skipped = [row for report in group_reports.values() for row in report['skipped']]
    identities_match = all(group in group_reports
                           and group_reports[group]['actual_case_ids'] == identities[group]
                           for group in GROUPS)
    not_run = {group: [identity for identity in identities[group]
                      if identity not in group_reports.get(group, {}).get('actual_case_ids', [])]
               for group in GROUPS}
    not_run = {group: rows for group, rows in not_run.items() if rows}
    return {'schema_version': 1, 'issue': 632, 'descriptor_sha256': canonical(descriptor),
            'status': 'completed' if not not_run else 'failed-partial', 'not_run_case_ids': not_run,
            'planned_case_ids': identities, 'planned_case_count': planned,
            'groups': group_reports, 'actual_case_ids_match_plan': identities_match,
            'tests_run': tests_run, 'failures': sum(len(result.failures) for result in results),
            'errors': sum(len(result.errors) for result in results), 'skipped': skipped,
            'elapsed_seconds': time.monotonic() - start,
            'passed': all(result.wasSuccessful() for result in results)
                      and tests_run == planned and identities_match and not skipped,
            'qualification': 'All discovered cases, explicit historical contexts; no fixed128 success criterion. '
                             'Class/module fixtures finish inside their group context. '
                             'Old assertions/verifiers are unchanged; current/ordinary use actual source.'}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--descriptor', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--verbosity', type=int, default=2)
    parser.add_argument('--guarded-point-artifacts', type=Path, required=True)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args(argv)
    root = args.root.resolve()
    descriptor = load_descriptor(root, args.descriptor)
    protected = {(root / path).resolve() for path in descriptor['actual_source_fingerprints']}
    protected.add((args.descriptor or root / BACKEND / 'historical_law_suite.json').resolve())
    if args.output.exists() or args.output.resolve() in protected:
        parser.error('output must be a new receipt outside protected input paths')
    authenticate_sources(root, descriptor)
    phase, graph_tests, previous_artifacts = 'fixture-preflight', None, None
    try:
        artifacts = prepare_guarded_point_artifacts(root, descriptor, args.guarded_point_artifacts)
        from archived_conformance_test_support import authenticate_archive
        archive_authority = descriptor['archived_conformance']
        authenticate_archive(Path(archive_authority['archive_root']),
                             archive_authority['old_source_fingerprints'])
        sys.path.insert(0, str(root / BACKEND))
        graph_tests = importlib.import_module('test_guarded_point_zkir')
        previous_artifacts = graph_tests.ARTIFACTS
        graph_tests.ARTIFACTS = args.guarded_point_artifacts.resolve()
        phase = 'discovery'
        suites, identities = load_suites(root, descriptor)
        if args.prepare_only:
            phase = 'preparation'
            prepare_earlier_view(root, descriptor)
            authenticate_sources(root, descriptor)
            report = {'schema_version': 1, 'issue': 632, 'status': 'prepared',
                      'descriptor_sha256': canonical(descriptor), 'planned_case_ids': identities,
                      'planned_case_count': sum(len(rows) for rows in identities.values()),
                      'checkpoint_stages': len(STAGES), 'signed617_anchors': descriptor['earlier_view_anchors'],
                      'guarded_point_artifacts': artifacts, 'test_execution': False,
                      'archived_conformance_case': archive_authority['case_id'],
                      'archived_source_count': len(archive_authority['old_source_fingerprints']),
                      'full_suite_passed': False, 'preparation_cache': 'deferred; no validation result cached'}
        else:
            phase = 'execution'
            report = run_suites(root, descriptor, suites, identities, verbosity=args.verbosity,
                                stop_after_failed_group=True)
            report['guarded_point_artifacts'] = artifacts
        prepare_guarded_point_artifacts(root, descriptor, args.guarded_point_artifacts)
    except (OSError, ValueError, KeyError, TypeError) as error:
        report = {'schema_version': 1, 'issue': 632, 'status': 'infrastructure-refused',
                  'phase': phase, 'passed': False, 'full_suite_passed': False,
                  'test_execution_started': phase == 'execution',
                  'error': {'type': type(error).__name__, 'detail': str(error)},
                  'qualification': 'No complete success or inferred executed count; retain runner logs.'}
    finally:
        if graph_tests is not None:
            graph_tests.ARTIFACTS = previous_artifacts
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    return 0 if report['status'] == 'prepared' or report.get('passed', False) else 1


if __name__ == '__main__':
    sys.exit(main())
