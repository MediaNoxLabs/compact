# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate whole generic fallback retirement; historical evidence stays immutable.

Exact source reconstruction is a source-change gate, not language coverage or a
Rust semantic proof. Compiler laws and generated consumer tests are separate.
"""
import copy
import json

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = 'ee0510cfd3a4952f2ad064ebf5e0d3c70a7c006827d4cac4d2326e4a1f01e78f'
BASELINE_COMMIT = 'c64c6bb062ea13ea721ed4d8e05a7e7218603972'
BASE = 'tools/compact-rust-backend/src/recorded'
ROOT = BASE + '.rs'
SELECTOR = BASE + '/profile_selection.rs'
REJECTION = BASE + '/effect_plan/rejection.rs'
ROUTES = (
    'guard.root-let-snapshot', 'final.opaque-parameter-allowlist',
    'final.no-emitted-statements', 'legacy.action-dispatch',
    'legacy.action-bindings', 'legacy.action-assertions', 'legacy.action-control',
    'legacy.action-collections', 'legacy.action-merkle', 'legacy.return-conditional',
    'return.unit', 'return.cell', 'return.counter', 'return.discarded-read',
    'return.set', 'return.list', 'return.map', 'return.merkle',
    'return.pure-field-call', 'value.field-expression', 'value.boolean-expression',
    'value.scalar-expression', 'value.pure-call-audit', 'return.dispatch',
    'return.counter-to-bytes', 'return.saved-root-value',
    'return.direct-field-expression', 'return.direct-boolean-expression',
    'final.shielded-gap-precedence',
)


def predecessor_sources(root, transition):
    if (canonical(transition) != MANIFEST_SHA or transition.get('schema_version') != 1
            or transition.get('issue') != 618 or transition.get('baseline_commit') != BASELINE_COMMIT):
        raise ValueError('generic618: unreviewed transition')
    sources = {}
    for owner, row in transition['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('generic618: current source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('generic618: inverse is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('generic618: signed predecessor differs: ' + owner)
        sources[owner] = source
    for label in ['new_and_shared_sources', 'historical_manifests', 'oracle_authorities']:
        for owner, expected in transition[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('generic618: ' + label + ' changed: ' + owner)
    if len(transition['historical_manifests']) != 62:
        raise ValueError('generic618: all 62 historical manifests required')
    authorities = transition['original_oracle_preservation']
    original = json.loads((root / authorities['authority']).read_text())['files']
    if len(original) != 49:
        raise ValueError('generic618: original 49 oracle identities required')
    stages = [json.loads((root / owner).read_text()) for owner in authorities['stages']]
    overlap = sorted(row['path'] for row in original if row['path'] in sources)
    if overlap != authorities['signed_predecessor_owner_overlap']:
        raise ValueError('generic618: unreviewed original oracle owner overlap')
    for row in original:
        # The noncircular inner oracle transition restores #619 from live source
        # before the immutable stages. The outer inventory inverse is separate.
        source = (root / row['path']).read_text()
        for stage in stages:
            owner = stage['owners'].get(row['path'])
            if owner is None:
                continue
            if digest(source.encode()) != owner['current_sha256']:
                raise ValueError('generic618: current oracle predecessor changed: ' + row['path'])
            for edit in owner['predecessor_reconstruction']:
                if not edit['current'] or source.count(edit['current']) != 1:
                    raise ValueError('generic618: oracle inverse is not unique: ' + row['path'])
                source = source.replace(edit['current'], edit['previous'], 1)
            if digest(source.encode()) != owner['previous_sha256']:
                raise ValueError('generic618: oracle stage differs: ' + row['path'])
        if digest(source.encode()) != row['sha256']:
            raise ValueError('generic618: original oracle differs: ' + row['path'])
    return sources


class _Checkpoint:
    """Reviewed complete predecessors only; unrelated source reads remain live."""
    def __init__(self, root, transition):
        self._root = root
        self._sources = predecessor_sources(root, transition)

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, transition):
    return _Checkpoint(root, transition)


def _once(source, anchor, label):
    if source.count(anchor) != 1:
        raise ValueError('generic618: ' + label + ': ' + anchor)


def _source_closure(root, baseline):
    recorded = (root / ROOT).read_text()
    # The root renderer is the final function; its complete suffix is pinned.
    _once(recorded, 'fn render_recorded_item(', 'root renderer identity')
    body = recorded[recorded.index('fn render_recorded_item('):]
    for module in ['legacy', 'pure_calls', 'generic_reference', 'generic_migration_tests']:
        _once(recorded, '#[cfg(test)]\nmod ' + module + ';', 'test-only closure')
    for anchor in ['profile_selection::select_checked(',
                   'profile_selection::CheckedSelection::Admitted(plan) => plan,',
                   'profile_selection::CheckedSelection::Refused(rejection) =>',
                   'RecordingOutcome::Unsupported(rejection.into_checked_gap())',
                   'lower_effectful_return(', 'if circuit.internal && !helper {',
                   'RenderError::DuplicateParameter(', 'let arg_ty = rust_type(',
                   'let result_ty = rust_type(', 'let uses_witness = circuit_uses_witness(',
                   'let item = item::assemble(']:
        _once(body, anchor, 'checked root/retained boundary')
    for forbidden in ['append_steps(', 'boolean_expression(', 'field_expression(',
                      'closed_pure_field_call(', 'profile_selection::select(',
                      'shielded_send_rejection', 'unit_composition_rejection',
                      'organizer_steps', 'typed_result', 'record_option!',
                      'match &circuit.return_value', 'return.dispatch']:
        if forbidden in body:
            raise ValueError('generic618: production generic fallback remains: ' + forbidden)
    for row in baseline['routes']:
        if row['id'] not in ROUTES:
            continue
        for anchor in row['anchors']:
            owner = anchor['source']
            if owner == ROOT:
                if anchor['text'] in body:
                    raise ValueError('generic618: root route remains: ' + row['id'])
            elif not (owner.startswith(BASE + '/legacy/') or owner == BASE + '/pure_calls.rs'):
                raise ValueError('generic618: unclassified retirement owner: ' + owner)
    selector = (root / SELECTOR).read_text()
    for anchor in ['#[cfg(test)]\npub(super) fn select(',
                   '#[cfg(test)]\nfn admitted(', '#[cfg(test)]\nfn compatibility_rejection(',
                   '#[cfg(test)]\npub(super) struct ProfileSelection']:
        _once(selector, anchor, 'historical projection closure')
    checked = selector[selector.index('pub(super) fn select_checked('):selector.index('#[cfg(test)]\nfn admitted(')]
    for anchor in ['effect_plan::lower_with_calls_diagnostic(',
                   'Ok(CheckedSelection::Admitted(plan))',
                   'Ok(CheckedSelection::Refused(rejection))',
                   'Err(effect_plan::DiagnosticPlanError::Invalid(gap))',
                   'Err(RenderError::RecordingPlanInvariant {']:
        # Admitted also appears in the Effectful prechecked early return.
        expected = 2 if anchor == 'Ok(CheckedSelection::Admitted(plan))' else 1
        if checked.count(anchor) != expected:
            raise ValueError('generic618: checked compiler authority changed: ' + anchor)
    for forbidden in ['typed_plan::', 'legacy_select(', 'compatibility_rejection(', 'append_steps(']:
        if forbidden in checked:
            raise ValueError('generic618: checked compiler fallback: ' + forbidden)
    rejection = (root / REJECTION).read_text()
    for anchor in ['pub(in crate::recorded) fn into_checked_gap(mut self) -> RecordingGap',
                   'if let Some(node) = self.concrete_node {', 'self.gap.ir_node = node.name;',
                   '#[cfg(test)]\n    pub(in crate::recorded) fn refine_action(',
                   '#[cfg(test)]\nfn encloses(']:
        _once(rejection, anchor, 'checked refusal provenance')
    return {'root_function_sha256': digest(body.encode()),
            'checked_function_sha256': digest(checked.encode()),
            'ordinary_checker_call_sites': 1,
            'generic_modules_test_only': ['legacy', 'pure_calls'],
            'diagnostic_policy': 'checked compiler refusals'}


def verify_transition(root, baseline, transition, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, transition))
        if not prior['passed']:
            report['errors'] = ['generic618: predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        old_ids = [row['id'] for row in prior['retired_routes']]
        baseline_ids = [row['id'] for row in baseline['routes']]
        ids = transition['retired_route_ids']
        if (prior['route_count'] != 84 or len(old_ids) != 37 or ids != list(ROUTES)
                or any(baseline_ids.count(route) != 1 or route in old_ids for route in ids)):
            raise ValueError('generic618: original denominator/new retirement identities changed')
        if prior['current_selector'] != transition['previous_selector']:
            raise ValueError('generic618: signed predecessor selector changed')
        actual = selector_signature((root / SELECTOR).read_text())
        if actual != transition['current_selector']:
            raise ValueError('generic618: current selector source changed')
        closure = _source_closure(root, baseline)
        if closure != transition['production_closure']:
            raise ValueError('generic618: reviewed production closure changed')
        for route in ids:
            row = next(row for row in report['route_checks'] if row['id'] == route)
            if row['failures']:
                raise ValueError('generic618: predecessor route was not authenticated: ' + route)
            row['current_status'] = 'retired-from-production-test-oracle-retained'
            row['retirement_issue'] = 618
            report['retired_routes'].append({
                'id': route, 'status': 'retired-from-production-test-oracle-retained',
                'passed': True, 'evidence': {'issue': 618, 'production_closure': closure}})
        report['current_selector'] = actual
        report['reviewed_generic_retirement'] = {
            'issue': 618, 'claim': transition['claim'],
            'predecessor_commit': transition['baseline_commit'],
            'new_retired_routes': ids, 'production_closure': closure,
            'historical_manifest_count': 62, 'original_oracle_count': 49,
            'original_oracle_owner_overlap': transition['original_oracle_preservation']['signed_predecessor_owner_overlap'],
            'owners': copy.deepcopy(transition['owners'])}
        historical_ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        removed = ['guard.root-let-snapshot', 'legacy.action-dispatch', 'return.dispatch']
        if any(historical_ids.count(route) != 1 for route in removed):
            raise ValueError('generic618: predecessor generic trace identities changed')
        report['reviewed_instrumentation'] = {
            'issue': 618,
            'instrumented_route_ids': [route for route in historical_ids if route not in removed],
            'claim': 'production generic ownership removed; historical test traces retained'}
        for row in report['additional_current_routes']:
            if row['id'] == 'algebra.ordinary':
                row['current_source_authority'] = transition
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected,
             'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
