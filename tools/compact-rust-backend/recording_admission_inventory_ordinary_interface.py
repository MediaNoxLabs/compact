# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate a general checked ordinary interface without changing baseline retirement."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '83f0147e3fbf6bf34ebfeab44227f50c41e96d142c374df3e24df5fb20bb667c'
OWNER = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
ROUTE = 'algebra.ordinary'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('ordinary606: unreviewed emission extension')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('ordinary606: current complete source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('ordinary606: predecessor replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('ordinary606: signed predecessor reconstruction failed: ' + owner)
        sources[owner] = source
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('ordinary606: checked authority or tests changed: ' + owner)
    return sources


class _Checkpoint:
    """Only exact reviewed predecessors; every other file remains live."""
    def __init__(self, root, sources, extension):
        if canonical(extension) != MANIFEST_SHA or set(sources) != set(extension['owners']):
            raise ValueError('ordinary606: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != extension['owners'][owner]['previous_sha256']:
                raise ValueError('ordinary606: unauthenticated predecessor owner: ' + owner)
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, predecessor_sources(root, extension), extension)


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source emission authority; not language or behavioral coverage',
              'route_count': len(baseline.get('routes', [])), 'route_checks': [],
              'retired_routes': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, extension))
        if not prior['passed']:
            report['errors'] = ['ordinary606: immutable predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != extension['current_selector'] or prior['current_selector'] != extension['previous_selector']:
            raise ValueError('ordinary606: actual adapter priority or selector identity changed')
        if len(prior['retired_routes']) != 34 or prior['route_count'] != 84:
            raise ValueError('ordinary606: predecessor inventory counts changed')
        report['reviewed_ordinary_interface_extension'] = {
            'issue': 606, 'claim': extension['claim'],
            'retirement_count_unchanged': True,
            'predecessor_commit': extension['baseline_commit'],
            'owners': copy.deepcopy(extension['owners']),
        }
        if actual_selector['ordered_calls'] or prior['current_selector']['ordered_calls']:
            raise ValueError('ordinary606: old named selectors must remain retired')
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        if ROUTE in ids or (root / SELECTOR).read_text().count('"algebra.ordinary"') != 1:
            raise ValueError('ordinary606: new ordinary route identity differs')
        report['current_selector'] = actual_selector
        report['reviewed_instrumentation'] = {
            'issue':606, 'instrumented_route_ids': [*ids, ROUTE],
            'claim':'Current source instrumentation; 84 historical boundaries remain the denominator, not all current routes.'}
        report['additional_current_routes'] = [{'id':ROUTE, 'status':'active-checked-interface', 'passed':True,
            'source':SELECTOR, 'retirement_counted':False, 'source_authority':extension}]
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected,
             'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError) as error:
        report['errors'].append(str(error))
    return report
