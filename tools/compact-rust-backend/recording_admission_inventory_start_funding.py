# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate exclusive StartFunding fallback retirement with exact signed-source authority."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '87110f5d29e930faeea28977557a3c1a843cf2c0575a6dd9241ca47f57b0e877'
OWNER = 'tools/compact-rust-backend/src/recorded.rs'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('funding617: unreviewed emission extension')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('funding617: current complete source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('funding617: predecessor replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('funding617: signed predecessor reconstruction failed: ' + owner)
        sources[owner] = source
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('funding617: checked authority or tests changed: ' + owner)
    return sources


class _Checkpoint:
    """Only exact reviewed predecessors; every other file remains live."""
    def __init__(self, root, sources, extension):
        if canonical(extension) != MANIFEST_SHA or set(sources) != set(extension['owners']):
            raise ValueError('funding617: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != extension['owners'][owner]['previous_sha256']:
                raise ValueError('funding617: unauthenticated predecessor owner: ' + owner)
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, predecessor_sources(root, extension), extension)


ROUTE = 'effectful.start-funding'
TYPED_PLAN = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
SHARED = 'tools/compact-rust-backend/src/recorded/recording_plan.rs'


def verify_transition(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source route inventory; not language or behavioral coverage',
              'route_count': len(baseline.get('routes', [])), 'route_checks': [],
              'retired_routes': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, extension))
        if not prior['passed']:
            report['errors'] = ['funding617: immutable predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        actual = selector_signature((root / SELECTOR).read_text())
        if actual != extension['current_selector'] or prior['current_selector'] != extension['previous_selector']:
            raise ValueError('funding617: actual or predecessor selector changed')
        if len(prior['retired_routes']) != 36 or prior['route_count'] != 84 or extension['retired_route_ids'] != [ROUTE]:
            raise ValueError('funding617: exact single-retirement accounting changed')
        recorded = (root / OWNER).read_text()
        if any(token in recorded for token in ['typed_plan::lower_start_funding(', 'LegacyStartFunding', ROUTE]):
            raise ValueError('funding617: retired fallback or trace remains')
        for module in ['typed_plan', 'intent_effect']:
            if recorded.count('#[cfg(test)]\nmod ' + module + ';') != 1:
                raise ValueError('funding617: old module must remain test-only: ' + module)
        if recorded.count('mod recording_plan;') != 1:
            raise ValueError('funding617: shared recording carrier is missing')
        start = recorded.index('fn lower_effectful_return(')
        end = recorded.index('fn checked_effectful_return(', start)
        lowered = recorded[start:end]
        for anchor in ['checked_effectful_return(circuit, ledger_fields, witnesses, pure_circuits, circuits)?',
                       'return Ok(RecordingOutcome::Supported(plan));',
                       'RecordingOutcome::Unsupported(gap) => gap',
                       'Ok(RecordingOutcome::Unsupported(algebra_gap))']:
            if lowered.count(anchor) != 1:
                raise ValueError('funding617: checked effectful authority changed: ' + anchor)
        typed = (root / TYPED_PLAN).read_text()
        if typed.count('pub(super) use super::recording_plan::{TypedPlan, shielded_send_result};') != 1:
            raise ValueError('funding617: frozen oracle shared definitions are not exact reexports')
        shared = (root / SHARED).read_text()
        if shared.count('pub(super) struct TypedPlan {') != 1 or shared.count('pub(super) fn shielded_send_result(ty: &Type) -> bool {') != 1:
            raise ValueError('funding617: unique shared definitions changed')
        route = next(row for row in baseline['routes'] if row['id'] == ROUTE)
        if route['anchors'] != [{'source': OWNER, 'text': 'typed_plan::lower_start_funding(', 'baseline_occurrences': 1}]:
            raise ValueError('funding617: original route anchor changed')
        checks = [row for row in report['route_checks'] if row['id'] == ROUTE]
        if len(checks) != 1 or checks[0]['failures'] or not checks[0]['passed']:
            raise ValueError('funding617: prior route was not authenticated')
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        for label in [ROUTE, 'effectful.checked-migration', 'algebra.ordinary', 'guard.root-let-snapshot', 'legacy.action-dispatch', 'return.dispatch']:
            if ids.count(label) != 1:
                raise ValueError('funding617: prior surviving route identity changed: ' + label)
        report['retired_routes'].append({'id': ROUTE, 'status': 'retired-from-production-test-oracle-retained', 'passed': True, 'evidence': extension})
        report['reviewed_instrumentation'] = {'issue': 617, 'instrumented_route_ids': [label for label in ids if label != ROUTE], 'claim': 'reviewed source instrumentation; execution of every boundary is not claimed'}
        report['current_selector'] = actual
        report['reviewed_start_funding_retirement'] = {'issue': 617, 'claim': extension['claim'], 'predecessor_commit': extension['baseline_commit'], 'owners': copy.deepcopy(extension['owners']), 'qualification_limits': extension['qualification_limits']}
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected, 'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
