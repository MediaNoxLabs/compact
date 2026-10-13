# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Native Unit views from checked children; complete syntax-free body fold pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = 'dc6e6d164e9cff94bddf9795840a1bc3fd527f7e359f84ce3b8e686907c87bbe'
BASELINE_COMMIT = '4eaa43a98987a072793f1b8066d7c2a1933ecc8d'
BASE = 'tools/compact-rust-backend/src'
LIB = BASE + '/lib.rs'
BRIDGE = BASE + '/recorded/effect_plan/pure_certification/native.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
FAMILIES = ('aggregate', 'conversion', 'arithmetic', 'crypto', 'leaf')


def predecessor_sources(root, extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'C1+C2'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []):
        raise ValueError('unit622C: unreviewed extension')
    if set(extension['owners']) != {LIB, BRIDGE}:
        raise ValueError('unit622C: both complete owners required')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('unit622C: current owner changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('unit622C: inverse is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('unit622C: signed predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('unit622C: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 70:
        raise ValueError('unit622C: all 70 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.unit-fragments-622c.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('unit622C: native view/checking boundary changed: ' + anchor)


def variant_map(source):
    """Associate each pattern occurrence with its directly invoked family."""
    mapping = {}
    for match in re.finditer(r'Expr::([A-Za-z0-9_]+)', source):
        variant = match.group(1)
        arrow = source.find('=>', match.end())
        owner = re.search(r'pure_(control|aggregate|conversion|arithmetic|crypto|leaf)_(?:expression|fragment)\(', source[arrow:])
        if arrow < 0 or owner is None or variant in mapping:
            raise ValueError('unit622C: ambiguous or duplicate direct variant owner: ' + variant)
        mapping[variant] = owner.group(1)
    return mapping


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib, bridge = text(LIB), text(BRIDGE)
    preserved = {}
    for family in FAMILIES:
        name = 'pure_' + family + '_expression'
        now, old = function(lib, name), function(previous[LIB], name)
        if now != old:
            raise ValueError('unit622C: unmigrated semantic family changed: ' + family)
        preserved[name] = digest(now.encode())
    for name in ('check_defaults', 'check_named_structs'):
        now, old = function(bridge, name), function(previous[BRIDGE], name)
        if now != old:
            raise ValueError('unit622C: recording supplement changed: ' + name)
        preserved[name] = digest(now.encode())
    oldcheck, check = function(previous[BRIDGE], 'check'), function(bridge, 'check')
    oldanchor = '    let (_, actual) = crate::expression_with_calls('
    anchor = '    let (_, actual) = crate::native_fragment_with_calls('
    if oldcheck[:oldcheck.index(oldanchor)] != check[:check.index(anchor)]:
        raise ValueError('unit622C: signature/default scope checks changed')
    require(bridge, 'crate::native_fragment_with_calls(&source.body, &scope, pure)?;')
    for owner in (LIB, BRIDGE, BASE + '/pure_native.rs', BASE + '/pure_native/control.rs',
                  BASE + '/pure_native/fragment.rs', BASE + '/pure_native/dispatch.rs'):
        if re.search(r'\bunit_statements\s*\(', text(owner)):
            raise ValueError('unit622C: production recursive Unit typing remains: ' + owner)
    require(text(BASE + '/pure_native.rs'), '#[cfg(test)]\nmod tests;')
    require(text(BASE + '/pure_native.rs'), '#[cfg(test)]\npub(crate) mod visitation;')
    require(lib, 'mod pure_native;')
    require(lib, 'pure_native::dispatch::dispatch!(', 2)
    for name in ('native_fragment_with_calls', 'expression_with_calls'):
        entry = function(lib, name)
        if entry.count('pure_native::dispatch::dispatch!(') != 1:
            raise ValueError('unit622C: entry no longer uses one shared variant map')
        if 'expression_with_calls(' in entry[entry.index('{') + 1:] or 'native_fragment_with_calls(' in entry[entry.index('{') + 1:]:
            raise ValueError('unit622C: extra recursive output entry wrapper')
    dispatcher = text(BASE + '/pure_native/dispatch.rs')
    require(dispatcher, 'macro_rules! dispatch {')
    for family in FAMILIES:
        if '$crate::pure_' + family + '_expression(' not in dispatcher:
            raise ValueError('unit622C: direct semantic owner missing: ' + family)
    variants = variant_map(dispatcher)
    oldvariants = variant_map(function(previous[LIB], 'expression_with_calls'))
    if variants != oldvariants or re.search(r'\b_\s*=>', dispatcher):
        raise ValueError('unit622C: exhaustive original variant ownership changed')
    require(dispatcher, 'let result = match $expr {')
    require(dispatcher, '$value($expr, result)')
    require(dispatcher, '$value(', 1)
    require(dispatcher, 'return $control(', 1)
    require(dispatcher, '$crate::pure_control_fragment(', 1)
    group = ('$crate::ir::Expr::Assert { .. }\n'
             '            | $crate::ir::Expr::Sequence { .. }\n'
             '            | $crate::ir::Expr::If { .. }\n'
             '            | $crate::ir::Expr::Let { .. } => {')
    require(dispatcher, group)
    control = function(lib, 'pure_control_fragment')
    for anchor in ('native_fragment_with_calls(step, parameters, circuits)?;',
                   'statements.extend(fragment.into_views().1);',
                   'native_fragment_with_calls(then, parameters, circuits)?;',
                   'native_fragment_with_calls(otherwise, parameters, circuits)?;',
                   'if then_ty != otherwise_ty {', 'equal_arms: then == otherwise,'):
        require(control, anchor)
    ordering = ('native_fragment_with_calls(then, parameters, circuits)?;',
                'native_fragment_with_calls(otherwise, parameters, circuits)?;',
                'if then_ty != otherwise_ty {', 'equal_arms: then == otherwise,')
    positions = [control.index(x) for x in ordering]
    if positions != sorted(positions):
        raise ValueError('unit622C: equal-arm coalescing precedes source child checking')
    for owner in (BASE + '/pure_native/control.rs', BASE + '/pure_native/fragment.rs'):
        source = text(owner)
        for forbidden in ('ir::Expr', '&Expr', 'ir::Type', '&Type', 'RenderError', 'expression_with_calls(', 'native_fragment_with_calls('):
            if forbidden in source:
                raise ValueError('unit622C: formatter acquired source/type checking: ' + owner)
    require(lib, 'native_fragment_with_calls(&circuit.body, &parameters, &callable_circuits)?;')
    require(lib, 'let (body, unit_body) = body.into_views();')
    # The test oracle retains complete old semantic routines, not a copy of the
    # new fragment path. Imports/observer are adapters; every old body is exact.
    oracle = {}
    for file, names in {
        'control': ('lift_block_condition', 'unit_statements', 'pure_control_expression'),
        'dispatch': ('expression_with_calls',),
        **{family: ('pure_' + family + '_expression',) for family in FAMILIES},
    }.items():
        source = text(BASE + '/pure_native/tests/old_oracle/' + file + '.rs')
        for name in names:
            body = function(source, name)
            if body != function(previous[LIB], name):
                raise ValueError('unit622C: frozen old native oracle changed: ' + name)
            oracle[name] = digest(body.encode())
    return {'preserved_semantic_functions': preserved, 'old_oracle_functions': oracle,
            'source_variant_count': len(variants),
            'dispatcher_layout': 'one common post-match value Result; one grouped control projection', 'recursive_unit_typing': 'retired from production; test oracle retained',
            'native_views': 'expression/Unit statements from one checked child set',
            'static_work_correction': 'identical If arms are separately checked before output coalescing; no identical visit-count claim',
            'body_syntax_bridge': 'still constructs syntax; complete syntax-free body/binding fold pending622',
            'new_retired_routes': [], 'claim': 'native Unit/dispatch groundwork; source inventory is not full language coverage'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['unit622C: predecessor checks failed', *prior['errors']]
            return report
        if prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627:
            raise ValueError('unit622C: mandatory actual627 predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('unit622C: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('unit622C: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('unit622C: reviewed native fragment closure changed')
        report['reviewed_unit_fragment_extension'] = {
            'issue': 622, 'slice': 'C1+C2', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': extension['baseline_commit'], 'historical_manifest_count': 70,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
