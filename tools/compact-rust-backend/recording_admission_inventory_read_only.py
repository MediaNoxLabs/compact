# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate retirement of the early read-only verifier with exact signed-source authority."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '642e65f166bf00735278130e2dfe04ae43d60058b1a9c19ce4938c653ca81edc'
OWNER = 'tools/compact-rust-backend/src/recorded.rs'
AUDITED_LOCAL = 'tools/compact-rust-backend/src/recorded/audited_local.rs'
ROUTE = 'early.read-only-verification'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('readonly615: unreviewed emission extension')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('readonly615: current complete source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('readonly615: predecessor replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('readonly615: signed predecessor reconstruction failed: ' + owner)
        sources[owner] = source
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('readonly615: checked authority or tests changed: ' + owner)
    return sources


class _Checkpoint:
    """Only exact reviewed predecessors; every other file remains live."""
    def __init__(self, root, sources, extension):
        if canonical(extension) != MANIFEST_SHA or set(sources) != set(extension['owners']):
            raise ValueError('readonly615: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != extension['owners'][owner]['previous_sha256']:
                raise ValueError('readonly615: unauthenticated predecessor owner: ' + owner)
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
            report['errors'] = ['readonly615: immutable predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != extension['current_selector'] or prior['current_selector'] != extension['previous_selector']:
            raise ValueError('readonly615: actual selector priority or identity changed')
        if actual_selector != prior['current_selector']:
            raise ValueError('readonly615: retirement changed unrelated selector identity')
        if len(prior['retired_routes']) != 35 or prior['route_count'] != 84:
            raise ValueError('readonly615: predecessor inventory counts changed')
        recorded = (root / OWNER).read_text()
        audited = (root / AUDITED_LOCAL).read_text()
        if any(call in recorded for call in ['audited_local::render(', 'read_only_verification::render(', ROUTE]):
            raise ValueError('readonly615: retired early call or instrumentation still live')
        if recorded.count('#[cfg(test)]\nmod audited_local;') != 1 or recorded.count('#[cfg(test)]\nmod read_only_verification;') != 1:
            raise ValueError('readonly615: complete old modules must remain test-only')
        if recorded.count('#[cfg(test)]\nuse crate::retained_value;') != 1:
            raise ValueError('readonly615: retained-value import must remain test-only')
        if audited.count('pub(super) fn unit_helper(') != 1:
            raise ValueError('readonly615: old shared Unit helper was changed')
        route = next(row for row in baseline['routes'] if row['id'] == ROUTE)
        if route['anchors'] != [{'source': OWNER, 'text': 'read_only_verification::render(', 'baseline_occurrences': 1}]:
            raise ValueError('readonly615: original route anchor changed')
        checks = [row for row in report['route_checks'] if row['id'] == ROUTE]
        if len(checks) != 1 or checks[0]['failures'] or not checks[0]['passed']:
            raise ValueError('readonly615: prior route was not authenticated')
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        if any(ids.count(route) != 1 for route in [ROUTE, 'algebra.unit-composition', 'algebra.membership']):
            raise ValueError('readonly615: prior route identity changed')
        report['retired_routes'].append({'id': ROUTE, 'status': 'retired-from-production-test-oracle-retained', 'passed': True, 'evidence': extension})
        report['reviewed_instrumentation'] = {'issue': 615, 'instrumented_route_ids': [route for route in ids if route != ROUTE], 'claim': 'reviewed source instrumentation; execution of every boundary is not claimed'}
        report['reviewed_read_only_retirement'] = {'issue': 615, 'claim': extension['claim'], 'predecessor_commit': extension['baseline_commit'], 'owners': copy.deepcopy(extension['owners'])}
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected,
             'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
