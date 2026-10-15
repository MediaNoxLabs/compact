# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Shared conversion facts and per-conversion compiler work; body fold pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '38530b80581a2efad67390b61cf311c692a4735a4750470f1f39ed9f9345f9b9'
BASELINE_COMMIT = '05217556bd2fe699ffdc2a0828d4908b894f98ae'
BASE = 'tools/compact-rust-backend/src'
DOMAIN = BASE + '/recorded/effect_plan/validate/domain.rs'
NATIVE = BASE + '/value_lowering.rs'
NUMERIC = BASE + '/pure_semantics/numeric.rs'
LIMITS = BASE + '/resource_limits.rs'
POLICY = BASE + '/pure_semantics/coercion.rs'
OWNERS = (BASE + '/pure_semantics.rs', NUMERIC, NATIVE, LIMITS, DOMAIN)
SELECTOR = BASE + '/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 627 or extension.get('parent_issue') != 622
            or extension.get('slice') != 'B' or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []):
        raise ValueError('coercion627: unreviewed extension')
    if set(extension['owners']) != set(OWNERS):
        raise ValueError('coercion627: all five complete owners required')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('coercion627: current owner changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('coercion627: inverse is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('coercion627: signed predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('coercion627: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 69:
        raise ValueError('coercion627: all 69 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.shared-coercion-627.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('coercion627: semantic/resource boundary changed: ' + anchor)


def function(source, name):
    match = re.search(r'(?m)^(?:pub(?:\([^\n]*\))? )?fn ' + re.escape(name) + r'\b', source)
    if match is None:
        raise ValueError('coercion627: missing preserved function: ' + name)
    start = source.index('{', match.start())
    depth = 1
    for end in range(start + 1, len(source)):
        depth += (source[end] == '{') - (source[end] == '}')
        if depth == 0:
            return source[match.start():end + 1]
    raise ValueError('coercion627: incomplete preserved function: ' + name)


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    kept = {}
    for owner, names in {
        DOMAIN: ('public_interface_type', 'value_type', 'derives_supported', 'witness_result_type',
                 'default_tuple_traits_supported', 'default_value_type'),
        NATIVE: ('wide_uint_type', 'field_to_bytes_32_syntax', 'unsigned_arithmetic_syntax', 'rust_type',
                 'map_slot_value_type', 'map_slot_types', 'copy_type', 'retained_value'),
        NUMERIC: ('field_literal_bytes', 'unsigned_maximum', 'checked_unsigned_arithmetic'),
    }.items():
        for name in names:
            now, old = function(text(owner), name), function(previous[owner], name)
            if now != old:
                raise ValueError('coercion627: unrelated function changed: ' + name)
            kept[owner + ':' + name] = digest(now.encode())
    domain = function(text(DOMAIN), 'coercible')
    guards = ('crate::resource_limits::recording_type_nodes(source).is_ok()',
              'crate::resource_limits::recording_type_nodes(target).is_ok()',
              'value_type(source)', 'value_type(target)',
              'crate::pure_semantics::coercion::plan_coercion(source, target).is_ok()')
    for anchor in guards:
        require(domain, anchor)
    positions = [domain.index(anchor) for anchor in guards]
    if positions != sorted(positions):
        raise ValueError('coercion627: root bounds/domain guards must precede shared relation')
    native = text(NATIVE)
    for anchor in ('let plan = crate::pure_semantics::coercion::plan_coercion(actual, target)?;',
                   'coercion::render(&plan, value, depth)'):
        require(native, anchor, 2)
    require(native, '#[cfg(test)]\nfn coercion(')
    require(native, '#[cfg(test)]\nstruct Coercion {')
    policy, equality = text(POLICY), text(BASE + '/pure_semantics/coercion/equality.rs')
    for source in (policy, equality):
        for forbidden in ('syn::', 'quote::', 'proc_macro2::', 'parse_quote!'):
            if forbidden in source:
                raise ValueError('coercion627: neutral relation constructs syntax')
    for anchor in ('preflight(actual, target)?;', 'plan_bounded(actual, target, &mut Work::new())',
                   'drop(resource_limits::recording_type_nodes(actual)?);',
                   'drop(resource_limits::recording_type_nodes(target)?);',
                   'limit: resource_limits::coercion_work_limit(),',
                   'pending_limit: resource_limits::coercion_pending_limit(),',
                   'self.observed.saturating_add(units)', 'if self.observed > self.limit',
                   'resource: "coercion_work",', 'work.charge(count)?;',
                   'let mut elements = Vec::with_capacity(count);',
                   'if equivalent(actual, target, work)?', 'cast_strategy_from_bounds(source, target_bound)'):
        require(policy, anchor)
    if policy.index('work.charge(count)?;') >= policy.index('let mut elements = Vec::with_capacity(count);'):
        raise ValueError('coercion627: aggregate allocation precedes work charging')
    for anchor in ('if observed > work.pending_limit', 'resource: "coercion_pending",',
                   'work.charge(1)?;', 'pending.push(next);'):
        require(equality, anchor)
    if equality.index('work.charge(1)?;') >= equality.index('pending.push(next);'):
        raise ValueError('coercion627: pending allocation precedes work charging')
    formatter = text(BASE + '/value_lowering/coercion.rs')
    for forbidden in ('ir::Type', '&Type', 'unsigned_maximum(', 'TypeMismatch', 'Result<'):
        if forbidden in formatter:
            raise ValueError('coercion627: native formatter reacquired pair/admission decisions')
    require(formatter, 'pub(super) fn render(plan: &CoercionPlan, value: syn::Expr, depth: usize)')
    require(text(LIMITS), 'pub(crate) fn coercion_work_limit() -> usize {\n    Limits::DEFAULT.expanded_work\n}')
    require(text(LIMITS), 'pub(crate) fn coercion_pending_limit() -> usize {\n    Limits::DEFAULT.pending\n}')
    defaults = text(BASE + '/resource_limits/policy.rs').split('pub(super) const DEFAULT: Self = Self {')[1]
    require(defaults, 'expanded_work: 131072,')
    require(defaults, 'pending: 4096,')
    return {'preserved_functions': kept, 'root_domain_preconditions': 'existing value_type(source/target) retained before shared plan',
            'neutral_plan_sha256': digest(policy.encode()), 'native_formatter_sha256': digest(formatter.encode()),
            'compiler_work': {'issue': 627, 'limit': 131072, 'pending_limit': 4096,
                              'scope': 'per conversion conservative compiler units; not total contract memory or CPU guarantee'},
            'body_syntax_bridge': 'still pending622; conversion groundwork only', 'new_retired_routes': []}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['coercion627: predecessor checks failed', *prior['errors']]
            return report
        if prior.get('reviewed_target_edition_identifiers_extension', {}).get('issue') != 626:
            raise ValueError('coercion627: mandatory actual626 predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('coercion627: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('coercion627: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('coercion627: reviewed conversion closure changed')
        report['reviewed_shared_coercion_extension'] = {
            'issue': 627, 'parent_issue': 622, 'slice': 'B', 'claim': extension['claim'],
            'new_retired_routes': [], 'predecessor_commit': extension['baseline_commit'],
            'historical_manifest_count': 69, 'original_oracles': 49,
            'production_closure': closure, 'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
