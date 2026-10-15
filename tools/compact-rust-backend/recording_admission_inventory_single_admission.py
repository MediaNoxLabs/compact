# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate single ordinary admission with retained diagnostic compatibility with exact signed-source authority."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '3c8a36fea54824c312d174390c1a30741ffec18e05af4bb57ff61ca6333a80d3'
OWNER = 'tools/compact-rust-backend/src/recorded.rs'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('single616: unreviewed emission extension')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('single616: current complete source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('single616: predecessor replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('single616: signed predecessor reconstruction failed: ' + owner)
        sources[owner] = source
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('single616: checked authority or tests changed: ' + owner)
    return sources


class _Checkpoint:
    """Only exact reviewed predecessors; every other file remains live."""
    def __init__(self, root, sources, extension):
        if canonical(extension) != MANIFEST_SHA or set(sources) != set(extension['owners']):
            raise ValueError('single616: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != extension['owners'][owner]['previous_sha256']:
                raise ValueError('single616: unauthenticated predecessor owner: ' + owner)
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
            report['errors'] = ['single616: immutable predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        source = (root / SELECTOR).read_text()
        actual = selector_signature(source)
        if actual != extension['current_selector'] or prior['current_selector'] != extension['previous_selector']:
            raise ValueError('single616: actual or historical selector identity changed')
        if len(prior['retired_routes']) != 36 or prior['route_count'] != 84 or extension['retired_route_ids']:
            raise ValueError('single616: consolidation must preserve retirement accounting')
        production = source[source.index('pub(super) fn select('):]
        if production.count('effect_plan::lower_with_calls_diagnostic(') != 1:
            raise ValueError('single616: expected one ordinary checked call site')
        for anchor in ['if let Some(plan) = effectful_plan', 'return Ok(admitted(plan));',
                       'DiagnosticPlanError::Unsupported(rejection)', 'Ok(compatibility_rejection(circuit, rejection))',
                       'DiagnosticPlanError::Invalid(gap)', 'Err(RenderError::RecordingPlanInvariant {',
                       'organizer_steps: Some(plan.steps)', 'typed_result: Some(plan.result)',
                       'circuit.result == Type::Unit && circuit.return_value == StateReturn::Unit',
                       'selection.unit_composition_rejection = Some(rejection)',
                       'typed_plan::shielded_send_result(&circuit.result)',
                       'selection.shielded_send_rejection = Some(rejection.into_gap())']:
            if production.count(anchor) != 1:
                raise ValueError('single616: checked admission/provenance anchor changed: ' + anchor)
        if source.count('#[cfg(test)]\npub(super) fn legacy_select(') != 1:
            raise ValueError('single616: old staged selector must be test-only')
        for name in ['ordinary_domain', 'unit_domain']:
            if source.count('#[cfg(test)]\n#[path = "profile_selection/' + name + '.rs"]\nmod ' + name + ';') != 1:
                raise ValueError('single616: old adapter module must be test-only')
        recorded = (root / OWNER).read_text()
        for marker in ['#[cfg(test)]\nmacro_rules! record_option', '#[cfg(test)]\nmod profile_attempt;']:
            if recorded.count(marker) != 1:
                raise ValueError('single616: old attempt instrumentation must be test-only')
        labels = extension['consolidated_checked_route_ids']
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        if len(labels) != 12 or len(set(labels)) != 12 or any(ids.count(label) != 1 for label in labels):
            raise ValueError('single616: predecessor checked route identities changed')
        if ids.count('algebra.ordinary') != 1 or any(label in production for label in labels):
            raise ValueError('single616: production retains a staged checked label')
        retained = [label for label in ids if label not in labels]
        for label in ['effectful.checked-migration', 'effectful.start-funding', 'guard.root-let-snapshot', 'legacy.action-dispatch', 'return.dispatch', 'algebra.ordinary']:
            if retained.count(label) != 1:
                raise ValueError('single616: unrelated actual route lost')
        report['current_selector'] = actual
        report['reviewed_instrumentation'] = {'issue': 616, 'instrumented_route_ids': retained, 'claim': 'one ordinary source call site per selector invocation; not once per contract or exhaustive execution coverage'}
        report['reviewed_single_admission'] = {'issue': 616, 'claim': extension['claim'], 'predecessor_commit': extension['baseline_commit'], 'consolidated_checked_route_ids': labels, 'retirement_delta': 0, 'owners': copy.deepcopy(extension['owners'])}
        ordinary = [row for row in report['additional_current_routes'] if row['id'] == 'algebra.ordinary']
        if len(ordinary) != 1 or ordinary[0]['retirement_counted']:
            raise ValueError('single616: ordinary additional-route accounting changed')
        ordinary[0]['current_source_authority'] = extension
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected, 'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
