# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate structured resource diagnostics without changing route retirement."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = 'a5c1ebc3cac47e6e99adf77c48065c2f366325f2b44d57870e6503251c3f9e34'
OWNER = 'tools/compact-rust-backend/src/recorded/effect_plan/rejection.rs'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('resource597: unreviewed diagnostics extension')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('resource597: current complete source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('resource597: predecessor replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('resource597: signed predecessor reconstruction failed: ' + owner)
        sources[owner] = source
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('resource597: checked authority or tests changed: ' + owner)
    return sources


class _Checkpoint:
    """Only exact reviewed predecessors; every other file remains live."""
    def __init__(self, root, sources, extension):
        if canonical(extension) != MANIFEST_SHA or set(sources) != set(extension['owners']):
            raise ValueError('resource597: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != extension['owners'][owner]['previous_sha256']:
                raise ValueError('resource597: unauthenticated predecessor owner: ' + owner)
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, predecessor_sources(root, extension), extension)


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source diagnostic authority; not language or behavioral coverage',
              'route_count': len(baseline.get('routes', [])), 'route_checks': [],
              'retired_routes': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, extension))
        if not prior['passed']:
            report['errors'] = ['resource597: immutable predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != extension['selector'] or actual_selector != prior['current_selector']:
            raise ValueError('resource597: actual adapter priority or selector identity changed')
        if len(prior['retired_routes']) != 28 or prior['route_count'] != 84:
            raise ValueError('resource597: predecessor inventory counts changed')
        report['reviewed_resource_diagnostics_extension'] = {
            'issue': 597, 'claim': extension['claim'],
            'retirement_count_unchanged': True,
            'predecessor_commit': extension['baseline_commit'],
            'owners': copy.deepcopy(extension['owners']),
        }
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected,
             'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError) as error:
        report['errors'].append(str(error))
    return report
