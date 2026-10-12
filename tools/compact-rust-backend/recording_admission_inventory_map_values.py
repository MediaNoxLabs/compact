# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate scalar Map value-codec expansion without changing route retirement."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '197ea0551a58ad50cb403cb16aeb21cc076265b5d5dec5ff2a73337810d1fa82'
OWNER = 'tools/compact-rust-backend/src/recorded/effect_plan/validate/maps.rs'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'


def require_predecessor_without_extension(root):
    # The #593 checkpoint predates Map policy authentication. Its explicit
    # verifier now requires that exact Map source unless #595 reconstructs it.
    if digest((root / OWNER).read_bytes()) != '9bf8a45e53888dd36550f0c9655cd63ddc7cf306de9673bbd75f6f0b8cb4eab7':
        raise ValueError('mapvalues595: changed Map codec policy requires explicit extension')


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('mapvalues595: unreviewed domain extension')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('mapvalues595: current complete source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('mapvalues595: predecessor replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('mapvalues595: signed predecessor reconstruction failed: ' + owner)
        sources[owner] = source
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('mapvalues595: checked authority or tests changed: ' + owner)
    return sources


class _Checkpoint:
    """Only exact reviewed predecessors; every other file remains live."""
    def __init__(self, root, sources, extension):
        if canonical(extension) != MANIFEST_SHA or set(sources) != set(extension['owners']):
            raise ValueError('mapvalues595: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != extension['owners'][owner]['previous_sha256']:
                raise ValueError('mapvalues595: unauthenticated predecessor owner: ' + owner)
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, predecessor_sources(root, extension), extension)


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source interface policy; not language or behavioral coverage',
              'route_count': len(baseline.get('routes', [])), 'route_checks': [],
              'retired_routes': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, extension))
        if not prior['passed']:
            report['errors'] = ['mapvalues595: immutable predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != extension['selector'] or actual_selector != prior['current_selector']:
            raise ValueError('mapvalues595: actual adapter priority or selector identity changed')
        if len(prior['retired_routes']) != 26 or prior['route_count'] != 84:
            raise ValueError('mapvalues595: predecessor inventory counts changed')
        source = (root / OWNER).read_text()
        for marker in extension['preserved_adapter_obligations']:
            if source.count(marker) != 1:
                raise ValueError('mapvalues595: checked adapter obligation changed')
        report['reviewed_map_value_extension'] = {
            'issue': 595, 'claim': extension['claim'],
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
