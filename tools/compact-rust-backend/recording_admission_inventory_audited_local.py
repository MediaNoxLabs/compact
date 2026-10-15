# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate retirement of the early audited-local renderer with exact signed-source authority."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '456d35f4cc0cd27e5288df572cecfaedefd474daa2c6a67e908fdc230deed280'
OWNER = 'tools/compact-rust-backend/src/recorded.rs'
AUDITED_LOCAL = 'tools/compact-rust-backend/src/recorded/audited_local.rs'
ROUTE = 'early.audited-local'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('audited614: unreviewed emission extension')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('audited614: current complete source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('audited614: predecessor replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('audited614: signed predecessor reconstruction failed: ' + owner)
        sources[owner] = source
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('audited614: checked authority or tests changed: ' + owner)
    return sources


class _Checkpoint:
    """Only exact reviewed predecessors; every other file remains live."""
    def __init__(self, root, sources, extension):
        if canonical(extension) != MANIFEST_SHA or set(sources) != set(extension['owners']):
            raise ValueError('audited614: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != extension['owners'][owner]['previous_sha256']:
                raise ValueError('audited614: unauthenticated predecessor owner: ' + owner)
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, predecessor_sources(root, extension), extension)


def verify_transition(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source route inventory; not language or behavioral coverage',
              'route_count': len(baseline.get('routes', [])), 'route_checks': [],
              'retired_routes': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, extension))
        if not prior['passed']:
            report['errors'] = ['audited614: immutable predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != extension['current_selector'] or prior['current_selector'] != extension['previous_selector']:
            raise ValueError('audited614: actual selector priority or identity changed')
        if actual_selector != prior['current_selector']:
            raise ValueError('audited614: retirement changed unrelated selector identity')
        if len(prior['retired_routes']) != 34 or prior['route_count'] != 84:
            raise ValueError('audited614: predecessor inventory counts changed')
        recorded = (root / OWNER).read_text()
        audited = (root / AUDITED_LOCAL).read_text()
        if 'audited_local::render(' in recorded or ROUTE in recorded:
            raise ValueError('audited614: retired call or instrumentation still live')
        if recorded.count('read_only_verification::render(') != 1:
            raise ValueError('audited614: surviving read-only route changed')
        if audited.count('#[cfg(test)]\nfn uncoerced<') != 1 or audited.count('#[cfg(test)]\npub(super) fn render(') != 1:
            raise ValueError('audited614: exclusive old renderer functions must remain test-only')
        if audited.count('pub(super) fn unit_helper(') != 1 or '#[cfg(test)]\npub(super) fn unit_helper(' in audited:
            raise ValueError('audited614: shared Unit helper must remain production')
        route = next(row for row in baseline['routes'] if row['id'] == ROUTE)
        if route['anchors'] != [{'source': OWNER, 'text': 'audited_local::render(', 'baseline_occurrences': 1}]:
            raise ValueError('audited614: original route anchor changed')
        checks = [row for row in report['route_checks'] if row['id'] == ROUTE]
        if len(checks) != 1 or checks[0]['failures'] or not checks[0]['passed']:
            raise ValueError('audited614: prior route was not authenticated')
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        if any(ids.count(route) != 1 for route in [ROUTE, 'early.read-only-verification', 'algebra.unit-composition', 'algebra.membership']):
            raise ValueError('audited614: prior route identity changed')
        report['retired_routes'].append({'id': ROUTE, 'status': 'retired-from-production-test-oracle-retained', 'passed': True, 'evidence': extension})
        report['reviewed_instrumentation'] = {'issue': 614, 'instrumented_route_ids': [route for route in ids if route != ROUTE], 'claim': 'reviewed source instrumentation; execution of every boundary is not claimed'}
        report['reviewed_audited_local_retirement'] = {'issue': 614, 'claim': extension['claim'], 'predecessor_commit': extension['baseline_commit'], 'owners': copy.deepcopy(extension['owners'])}
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected,
             'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
