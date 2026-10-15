# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate semantic helper discovery; inventory counts are not coverage."""
import copy

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = 'f0098830c803d0f270195a9481c451de42374c257fb60abe8421a792c0f2cd1e'
BASELINE_COMMIT = '5efd5166fbc7162a2403c07abcc88ea32489db9a'
BASE = 'tools/compact-rust-backend/src/recorded'
ROOT = BASE + '.rs'
HELPERS = BASE + '/helpers.rs'
SELECTOR = BASE + '/profile_selection.rs'
FACTS = 'tools/compact-rust-backend/src/resource_limits/recording_calls.rs'
GRAPH = BASE + '/helpers/call_graph.rs'
ROUTES = ('helper.discovery', 'helper.field-call-discovery', 'helper.unit-field-shape')


def predecessor_sources(root, transition):
    if (canonical(transition) != MANIFEST_SHA or transition.get('schema_version') != 1
            or transition.get('issue') != 620 or transition.get('baseline_commit') != BASELINE_COMMIT):
        raise ValueError('helpers620: unreviewed transition')
    sources = {}
    for owner, row in transition['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('helpers620: current source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('helpers620: inverse is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('helpers620: signed predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in transition[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('helpers620: ' + label + ' changed: ' + owner)
    if len(transition['historical_manifests']) != 64:
        raise ValueError('helpers620: all 64 scoped historical manifests required')
    oracle = transition['helper_oracle']
    source = (root / oracle['path']).read_text()
    if digest(source.encode()) != oracle['current_sha256']:
        raise ValueError('helpers620: helper reference source changed')
    marker = 'fn collect_field_callees('
    previous = sources[HELPERS]
    if (source.count(marker) != 1 or previous.count(marker) != 1
            or source[source.index(marker):] != previous[previous.index(marker):]):
        raise ValueError('helpers620: signed helper algorithm body changed')
    for edit in oracle['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('helpers620: helper reference inverse is not unique')
        source = source.replace(edit['current'], edit['previous'], 1)
    if (digest(source.encode()) != oracle['previous_sha256']
            or source != sources[HELPERS]):
        raise ValueError('helpers620: exact signed helper reference differs')
    return sources


class _Checkpoint:
    """Only reviewed predecessors; unrelated reads remain on the live checkout."""
    def __init__(self, root, transition):
        self._root = root
        self._sources = predecessor_sources(root, transition)

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, transition):
    return _Checkpoint(root, transition)


def source_closure(root, transition):
    helpers = (root / HELPERS).read_text()
    for forbidden in ('collect_shared_callees(', 'collect_field_callees(',
                      'let unit_body =', 'let direct_field_body =',
                      'circuit.result == Type::Field', 'circuit.result == Type::Unit'):
        if forbidden in helpers:
            raise ValueError('helpers620: old helper shape mechanism remains: ' + forbidden)
    if helpers.count('pub(crate) fn plan_recorded_helpers(') != 1:
        raise ValueError('helpers620: transitive helper admission boundary changed')
    recorded = (root / ROOT).read_text()
    for module in ('helpers_reference', 'helper_discovery_tests'):
        if recorded.count('#[cfg(test)]\nmod ' + module + ';') != 1:
            raise ValueError('helpers620: helper oracle/laws must be test-only')
    if recorded.count('if circuit.internal && !helper {') != 1:
        raise ValueError('helpers620: internal export boundary changed')
    for anchor, expected in {
        'CallGraph::from_declarations(ordered_circuits)?': 1,
        'graph.reachable_from(': 2,
        'render_recorded_helper(': 3,
        'render_recorded_circuit(': 1,
        'if !prospective[index] {': 1,
        'if !candidates[index] {': 1,
        'crate::naming::recorded_helper_ident(name, circuits)': 1,
        'pub(crate) use reachability::retain_reachable_helpers;': 1,
    }.items():
        if helpers.count(anchor) != expected:
            raise ValueError('helpers620: preserved planning/naming boundary changed: ' + anchor)
    graph = (root / GRAPH).read_text()
    for anchor in ('resource_limits::recording_call_fact_limits()',
                   'bound("declarations", circuits.len(), declarations)?;',
                   'resource_limits::recording_call_targets(circuit)?;',
                   'target_count.saturating_add(raw.len())',
                   'bound("nodes", target_count, targets)?;',
                   'graph.has_calls.push(!raw.is_empty());',
                   'indices.get(name).copied()', 'edges.sort_unstable();',
                   'vec![false; self.edges.len()]', 'if !selected[index]',
                   'while let Some(index) = pending.pop()'):
        if graph.count(anchor) != 1:
            raise ValueError('helpers620: bounded stateful graph changed: ' + anchor)
    for forbidden in ('pure_circuits', 'witnesses', 'StateReturn::', 'Type::Field', 'Type::Unit'):
        if forbidden in graph:
            raise ValueError('helpers620: graph confuses declaration kinds or body admission: ' + forbidden)
    facts = (root / FACTS).read_text()
    for anchor in ('pub(crate) fn recording_call_fact_limits() -> (usize, usize)',
                   '(Limits::DEFAULT.declarations, Limits::DEFAULT.nodes)',
                   'pub(crate) fn recording_call_targets(',
                   'Result<HashSet<&str>, crate::RenderError>',
                   'scan.push(Node::StatefulCircuit(circuit), 1, None)?;',
                   'while let Some(visit) = scan.pending.pop()',
                   'Node::StateAction(StateAction::CircuitCall { name, .. })',
                   'Node::Expr(Expr::Call { name, .. })',
                   'targets.insert(name.as_str());', 'children(visit, &mut scan)?;',
                   'check_frames(&scan.metrics, limits)?;'):
        if facts.count(anchor) != 1:
            raise ValueError('helpers620: bounded borrowed fact producer changed: ' + anchor)
    # Complete exact owners are pinned; these additional fences document which
    # semantic invariants the review requires, independently of their SHA guard.
    for owner, checks in transition['production_checks'].items():
        source = (root / owner).read_text()
        for anchor, expected in checks['required_occurrences'].items():
            if source.count(anchor) != expected:
                raise ValueError('helpers620: checked graph boundary changed: ' + anchor)
        for anchor in checks['forbidden']:
            if anchor in source:
                raise ValueError('helpers620: checked graph shape/fact confusion: ' + anchor)
    return {'helper_owner_sha256': digest(helpers.encode()),
            'borrowed_facts_sha256': digest(facts.encode()),
            'declaration_graph_sha256': digest(graph.encode()),
            'retained_semantic_route': 'helper.transitive-admission',
            'claim': 'bounded discovery and checked helper eligibility; no language coverage claim'}


def verify_transition(root, baseline, transition, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, transition))
        if not prior['passed']:
            report['errors'] = ['helpers620: predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        old_ids = [row['id'] for row in prior['retired_routes']]
        baseline_ids = [row['id'] for row in baseline['routes']]
        ids = transition['retired_route_ids']
        if (prior['route_count'] != 84 or len(old_ids) != 66 or ids != list(ROUTES)
                or any(baseline_ids.count(route) != 1 or route in old_ids for route in ids)):
            raise ValueError('helpers620: original denominator/new retirement identities changed')
        selector = selector_signature((root / SELECTOR).read_text())
        if selector != transition['current_selector'] or prior['current_selector'] != transition['previous_selector']:
            raise ValueError('helpers620: recording compiler authority changed')
        closure = source_closure(root, transition)
        if closure != transition['production_closure']:
            raise ValueError('helpers620: reviewed helper closure changed')
        for route in ids:
            row = next(row for row in report['route_checks'] if row['id'] == route)
            if row['failures']:
                raise ValueError('helpers620: predecessor helper route was not authenticated: ' + route)
            row['current_status'] = 'retired-from-production-test-oracle-retained'
            row['retirement_issue'] = 620
            report['retired_routes'].append({
                'id': route, 'status': 'retired-from-production-test-oracle-retained',
                'passed': True, 'evidence': {'issue': 620, 'production_closure': closure}})
        report['current_selector'] = selector
        report['reviewed_semantic_helper_discovery'] = {
            'issue': 620, 'claim': transition['claim'],
            'predecessor_commit': transition['baseline_commit'],
            'new_retired_routes': ids, 'production_closure': closure,
            'preserved_routes': transition['preserved_routes'],
            'historical_manifest_count': 64, 'owners': copy.deepcopy(transition['owners'])}
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected,
             'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
