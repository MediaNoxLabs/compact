# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive complete conversion-family authority; full syntax-free body fold pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = 'c6e2960e06fdaf4d3eaa66a17f7f1228e0e7effcdc6530423f1fb2f638998bab'
BASELINE_COMMIT = '8ecf01cbf0cde90831b597b2a3245e6ce1b00f77'
BASE = 'tools/compact-rust-backend/src'
LIB = BASE + '/lib.rs'
VALUE = BASE + '/value_lowering.rs'
SEMANTICS = BASE + '/pure_semantics.rs'
NATIVE = BASE + '/pure_native.rs'
TEST_MODULE = BASE + '/pure_native/tests.rs'
CONVERSION = BASE + '/pure_semantics/conversion.rs'
CHILDREN = BASE + '/pure_semantics/children.rs'
ENVIRONMENT = BASE + '/pure_native/children.rs'
COERCION_FORMATTER = BASE + '/value_lowering/coercion.rs'
SINK = BASE + '/pure_semantics/sink.rs'
SCOPE = BASE + '/pure_semantics/scope.rs'
OWNERSHIP = BASE + '/pure_semantics/ownership.rs'
FORMATTER = BASE + '/pure_native/conversion.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
OWNERS = {LIB, VALUE, SEMANTICS, NATIVE, TEST_MODULE, SINK, COERCION_FORMATTER}
FAMILIES = ('aggregate', 'arithmetic', 'crypto')


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'D1b'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != OWNERS):
        raise ValueError('conversion622D1b: unreviewed extension or incomplete owner set')


def inverse_regions(source, row, owner):
    """Restore only exact approved regions; mutations outside them survive."""
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('conversion622D1b: current inverse region changed: ' + owner)
        source = source.replace(edit['current'], edit['previous'], 1)
    return source


def predecessor_sources(root, extension):
    authenticate(extension)
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('conversion622D1b: current owner changed: ' + owner)
        source = inverse_regions(source, row, owner)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('conversion622D1b: signed D1a predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('conversion622D1b: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 72:
        raise ValueError('conversion622D1b: all 72 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.shared-conversion-622d1b.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('conversion622D1b: shared conversion boundary changed: ' + anchor)


def ordered(source, anchors):
    for anchor in anchors:
        require(source, anchor)
    positions = [source.index(x) for x in anchors]
    if positions != sorted(positions):
        raise ValueError('conversion622D1b: first-error ordering changed')


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib = text(LIB)
    preserved = {}
    for family in FAMILIES:
        name = 'pure_' + family + '_expression'
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('conversion622D1b: unmigrated semantic family changed: ' + family)
        preserved[name] = digest(function(lib, name).encode())
    for name in ('pure_leaf_expression', 'pure_control_fragment',
                 'native_fragment_with_calls', 'expression_with_calls'):
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('conversion622D1b: accepted leaf/control/entry changed: ' + name)
        preserved[name] = digest(function(lib, name).encode())
    adapter = function(lib, 'pure_conversion_expression')
    if digest(adapter.encode()) != extension['current_conversion_adapter_sha256']:
        raise ValueError('conversion622D1b: current conversion adapter changed')
    old = function(previous[LIB], 'pure_conversion_expression')
    if re.findall(r'Expr::(\w+)', adapter) != re.findall(r'Expr::(\w+)', old):
        raise ValueError('conversion622D1b: complete four-operation map changed')
    require(adapter, 'let typed = fold_conversion(')
    require(adapter, '&mut pure_native::children::NativeChildren::new(parameters, circuits),')
    require(adapter, '&mut pure_native::leaf::NativeLeafSink::new(),')
    for forbidden in ('parse_quote!', 'unsigned_maximum(', 'plan_coercion(', 'coerce_expression(',
                      'unsigned_cast_syntax(', 'expression_with_calls('):
        if forbidden in adapter:
            raise ValueError('conversion622D1b: semantic rule remained in native adapter: ' + forbidden)
    sink = text(SINK)
    appendix = extension['owners'][SINK]['predecessor_reconstruction'][0]['current']
    if sink.count(appendix) != 1 or sink.replace(appendix, '', 1) != previous[SINK]:
        raise ValueError('conversion622D1b: accepted leaf fact/sink domain changed')
    require(sink, 'pub(crate) trait ConversionSink: LeafSink {')
    require(sink, 'facts: super::conversion::ConversionFacts,')
    require(sink, 'child: Self::Value,')
    value = text(VALUE)
    if value.replace('pub(crate) mod coercion;', 'mod coercion;', 1) != previous[VALUE]:
        raise ValueError('conversion622D1b: native formatter module visibility exceeded')
    formatter_helper = text(COERCION_FORMATTER)
    restored = formatter_helper.replace('pub(crate) fn render_unsigned_cast(', 'pub(super) fn render_unsigned_cast(', 1)
    restored = restored.replace('pub(crate) fn render(', 'pub(super) fn render(', 1)
    if restored != previous[COERCION_FORMATTER]:
        raise ValueError('conversion622D1b: existing native interpreter body changed')
    for owner in (CONVERSION, CHILDREN, SINK):
        for forbidden in ('syn::', 'quote::', 'proc_macro', 'pure_native', 'NativeLeafSink',
                          'expression_with_calls(', 'native_fragment_with_calls(', 'rust_type('):
            if forbidden in text(owner):
                raise ValueError('conversion622D1b: neutral conversion acquired native checking: ' + owner)
    core = text(CONVERSION)
    require(core, 'let (facts, value, ty) = match source {')
    require(core, 'output: sink.conversion(facts, value),')
    require(core, 'children.visit(value, sink)?;', 4)
    require(core, '#[cfg(test)]\nmod tests;')
    operations = ('FieldCast', 'FieldToBytes32', 'Coerce', 'UnsignedCast')
    boundaries = [core.index('ConversionSource::' + name + ' {') for name in operations]
    arms = [core[a:b] for a, b in zip(boundaries, [*boundaries[1:], len(core)])]
    ordered(arms[0], ('let child = children.visit(value, sink)?;',
                     'let Type::Unsigned { max } = &child.ty else', 'unsigned_maximum(max)?'))
    ordered(arms[1], ('let child = children.visit(value, sink)?;', 'if child.ty != Type::Field'))
    ordered(arms[2], ('let child = children.visit(value, sink)?;',
                     'let plan = plan_coercion(&child.ty, ty)?;', 'ty.clone()'))
    ordered(arms[3], ('let target = unsigned_maximum(max)?;',
                     'let child = children.visit(value, sink)?;',
                     'let Type::Unsigned { max: source_max } = child.ty else',
                     'let source = unsigned_maximum(&source_max)?;',
                     'let cast = cast_strategy_from_bounds(source, target);'))
    contract = text(CHILDREN)
    require(contract, 'pub(crate) trait Children<S: LeafSink> {')
    require(contract, 'fn visit(&mut self, source: &Expr, sink: &mut S) -> Result<Typed<S::Value>, RenderError>;')
    native = text(ENVIRONMENT)
    require(native, "bindings: &'call HashMap<&'ir str, (&'ir Type, syn::Ident)>,", 2)
    require(native, "declarations: &'call HashMap<&'ir str, &'ir PureCircuit>,", 2)
    require(native, 'crate::expression_with_calls(source, self.bindings, self.declarations)?;')
    for forbidden in ('.clone()', 'HashMap::new(', '.collect(', '.insert(', 'fold_conversion('):
        if forbidden in native:
            raise ValueError('conversion622D1b: native child environment changed: ' + forbidden)
    formatter = text(FORMATTER)
    require(formatter, 'impl ConversionSink for NativeLeafSink {')
    require(formatter, 'coercion::render(&plan, child, 0)')
    require(formatter, 'coercion::render_unsigned_cast(child, cast)')
    for forbidden in ('RenderError', 'ir::Expr', 'ir::Type', '&Type', 'fold_conversion(',
                      'children.visit(', 'plan_coercion(', 'unsigned_maximum(', '.parse::<'):
        if forbidden in formatter:
            raise ValueError('conversion622D1b: native consumer acquired semantic checking: ' + forbidden)
    tests = text(TEST_MODULE)
    require(tests, 'mod shared_conversion;')
    if tests.replace('mod shared_conversion;\n', '', 1) != previous[TEST_MODULE]:
        raise ValueError('conversion622D1b: existing native test module changed')
    oracle = text(BASE + '/pure_native/tests/old_oracle/conversion.rs')
    if function(oracle, 'pure_conversion_expression') != old:
        raise ValueError('conversion622D1b: immutable old conversion routine changed')
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('conversion622D1b: current source authority changed: ' + owner)
    return {'preserved_functions': preserved, 'conversion_source_operations': 4,
            'conversion_owner': 'one shared generic complete conversion family with checked facts',
            'child_contract': 'borrowed visitor outside semantic core; transitional native child environment',
            'native_output': 'existing LeafSink value domain and fact-only native interpreters; visibility-only helper changes',
            'resource_policy': 'unchanged public/selected-parameter bounds and per-conversion627 cap; no cumulative body guarantee',
            'body_syntax_bridge': 'still constructs syntax; complete syntax-free body/binding fold pending622',
            'new_retired_routes': [], 'claim': 'conversion-family groundwork; source inventory is not language coverage'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['conversion622D1b: predecessor checks failed', *prior['errors']]
            return report
        leaf = prior.get('reviewed_shared_leaf_extension', {})
        unit = prior.get('reviewed_unit_fragment_extension', {})
        if (leaf.get('issue') != 622 or leaf.get('slice') != 'D1a'
                or unit.get('issue') != 622 or unit.get('slice') != 'C1+C2'
                or prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627):
            raise ValueError('conversion622D1b: mandatory actual D1a/C/B predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('conversion622D1b: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('conversion622D1b: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('conversion622D1b: reviewed conversion closure changed')
        report['reviewed_shared_conversion_extension'] = {
            'issue': 622, 'slice': 'D1b', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': BASELINE_COMMIT, 'historical_manifest_count': 72,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report


