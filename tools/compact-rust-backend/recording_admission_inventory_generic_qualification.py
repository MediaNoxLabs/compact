# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate generic qualification and result-role preparation; retire no route."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = "a4bdbdd7d5ea92525a0daa29858f3eb6464dccd051a4b7ab4264a75c76b6ca2f"
BASE = 'tools/compact-rust-backend/src/recorded'
SELECTOR = BASE + '/profile_selection.rs'


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('qualification619: unreviewed extension')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('qualification619: current source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('qualification619: inverse is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('qualification619: signed predecessor differs: ' + owner)
        sources[owner] = source
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('qualification619: new authority changed: ' + owner)
    return sources


class _Checkpoint:
    """Exact signed predecessors for reviewed owners; all other files stay live."""
    def __init__(self, root, extension):
        self._root = root
        self._sources = predecessor_sources(root, extension)

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def verify_transition(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, extension))
        if not prior['passed']:
            report['errors'] = ['qualification619: predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 37 or extension['retired_route_ids']:
            raise ValueError('qualification619: preparation cannot change retirement counts')
        actual = selector_signature((root / SELECTOR).read_text())
        if actual != extension['current_selector'] or prior['current_selector'] != extension['previous_selector']:
            raise ValueError('qualification619: current or predecessor selector changed')
        recorded = (root / (BASE + '.rs')).read_text()
        for module in ['generic_reference', 'generic_migration_tests']:
            if recorded.count('#[cfg(test)]\nmod ' + module + ';') != 1:
                raise ValueError('qualification619: reference/laws must be test-only')
        selector = (root / SELECTOR).read_text()
        wrapper = selector[selector.index('pub(super) fn select('):selector.index('pub(super) fn select_checked(')]
        checked = selector[selector.index('pub(super) fn select_checked('):selector.index('fn admitted(')]
        if wrapper.count('match select_checked(') != 1 or 'lower_with_calls_diagnostic(' in wrapper:
            raise ValueError('qualification619: wrapper must consume one outcome')
        if checked.count('effect_plan::lower_with_calls_diagnostic(') != 1:
            raise ValueError('qualification619: one ordinary checker required')
        lower = (root / (BASE + '/effect_plan/lower.rs')).read_text()
        validate = (root / (BASE + '/effect_plan/validate.rs')).read_text()
        if lower.count('if !execution_result_type(&circuit.result) {') != 1 or lower.count('if !root_interface_type(&p.ty, circuit.internal) {') != 1:
            raise ValueError('qualification619: lowerer result/input roles changed')
        if validate.count('if !execution_result_type(&plan.result_type) {') != 1 or validate.count('if !root_interface_type(&value.ty, internal)') != 1:
            raise ValueError('qualification619: validator result/input roles changed')
        if validate.count('pub(super) fn execution_result_type(ty: &Type) -> bool {\n    value_type(ty)\n}') != 1:
            raise ValueError('qualification619: execution domain must reuse local values')
        report['current_selector'] = actual
        report['reviewed_generic_qualification'] = {
            'issue': 619, 'related_issue': 618, 'claim': extension['claim'],
            'predecessor_commit': extension['baseline_commit'],
            'owners': copy.deepcopy(extension['owners']),
            'qualification_limits': extension['qualification_limits'],
            'new_retirements': 0,
        }
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected, 'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
