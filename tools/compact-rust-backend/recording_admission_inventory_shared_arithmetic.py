# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive complete arithmetic-family authority; full syntax-free body fold pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = '877396a018a1c331c3a7185a221783a4431d596d19bfdfa73369bdbd40a07656'
BASELINE_COMMIT = '5c995cff32088daf49c9dc2b926030b86ee00fcb'
BASE = 'tools/compact-rust-backend/src'
LIB = BASE + '/lib.rs'
VALUE = BASE + '/value_lowering.rs'
SEMANTICS = BASE + '/pure_semantics.rs'
NATIVE = BASE + '/pure_native.rs'
TEST_MODULE = BASE + '/pure_native/tests.rs'
ARITHMETIC = BASE + '/pure_semantics/arithmetic.rs'
CHILDREN = BASE + '/pure_semantics/children.rs'
ENVIRONMENT = BASE + '/pure_native/children.rs'
COERCION_FORMATTER = BASE + '/value_lowering/coercion.rs'
SINK = BASE + '/pure_semantics/sink.rs'
SCOPE = BASE + '/pure_semantics/scope.rs'
OWNERSHIP = BASE + '/pure_semantics/ownership.rs'
FORMATTER = BASE + '/pure_native/arithmetic.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
OWNERS = {LIB, VALUE, SEMANTICS, NATIVE, TEST_MODULE, SINK}
FAMILIES = ('aggregate', 'crypto')


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'D1c'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != OWNERS):
        raise ValueError('arithmetic622D1c: unreviewed extension or incomplete owner set')


def inverse_regions(source, row, owner):
    """Restore only exact approved regions; mutations outside them survive."""
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('arithmetic622D1c: current inverse region changed: ' + owner)
        source = source.replace(edit['current'], edit['previous'], 1)
    return source


def predecessor_sources(root, extension):
    authenticate(extension)
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('arithmetic622D1c: current owner changed: ' + owner)
        source = inverse_regions(source, row, owner)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('arithmetic622D1c: signed D1b predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('arithmetic622D1c: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 73:
        raise ValueError('arithmetic622D1c: all 73 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.shared-arithmetic-622d1c.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('arithmetic622D1c: shared arithmetic boundary changed: ' + anchor)


def ordered(source, anchors):
    for anchor in anchors:
        require(source, anchor)
    positions = [source.index(x) for x in anchors]
    if positions != sorted(positions):
        raise ValueError('arithmetic622D1c: first-error ordering changed')


def rust_tokens(source):
    """Relocation comparison only: preserve quoted literal contents exactly."""
    return re.findall(r'"(?:\\.|[^"\\])*"|\w+|[^\s]', source)


def unique_variants(source):
    # Native syn::Expr constructors in the old family are output, not source
    # variants. Root adapters and the signed old source map use bare Expr.
    return set(re.findall(r'(?<!:)\bExpr::(\w+)', source))


def match_body(source, marker):
    parts = rust_tokens(source)
    start = rust_tokens(marker)
    found = [i for i in range(len(parts) - len(start) + 1) if parts[i:i + len(start)] == start]
    if len(found) != 1:
        raise ValueError('arithmetic622D1c: checked native match boundary changed')
    opening = found[0] + len(start)
    if parts[opening] != '{':
        raise ValueError('arithmetic622D1c: checked native match opening changed')
    depth = 0
    for index in range(opening, len(parts)):
        if parts[index] == '{':
            depth += 1
        elif parts[index] == '}':
            depth -= 1
            if depth == 0:
                return parts[opening:index + 1]
    raise ValueError('arithmetic622D1c: incomplete native match')


def renderer_relocation(value, previous):
    old = function(previous, 'unsigned_arithmetic_syntax')
    wrapper = function(value, 'unsigned_arithmetic_syntax')
    old_prefix = rust_tokens(old[:old.index('    Ok(')])
    prefix = rust_tokens(wrapper[:wrapper.index('    let strategy =')])
    if prefix != old_prefix:
        raise ValueError('arithmetic622D1c: retained helper opcode/signature prefix changed')
    call = 'checked_unsigned_arithmetic(kind, left_max, right_max, result_max)?'
    require(wrapper, call)
    require(wrapper, 'Ok(render_unsigned_arithmetic(strategy, left, right))')
    renderer = function(value, 'render_unsigned_arithmetic')
    old_body = match_body(old, 'match ' + call)
    body = match_body(renderer, 'match strategy')
    if body != old_body:
        raise ValueError('arithmetic622D1c: exact moved native rendering match changed')
    for forbidden in ('RenderError', '&Expr', '&Type', 'unsigned_maximum(',
                      'checked_unsigned_arithmetic(', '.parse::<'):
        if forbidden in renderer:
            raise ValueError('arithmetic622D1c: checked renderer acquired semantic decisions')
    return digest(json.dumps(old_body, separators=(',', ':')).encode())


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib = text(LIB)
    current_import = rust_tokens(re.search(r'use ir::\{.*?\};', lib, re.S).group())
    previous_import = rust_tokens(re.search(r'use ir::\{.*?\};', previous[LIB], re.S).group())
    index = previous_import.index('ComparisonOperator')
    if previous_import[index + 1] != ',':
        raise ValueError('arithmetic622D1c: signed root import context changed')
    if current_import != previous_import[:index] + previous_import[index + 2:]:
        raise ValueError('arithmetic622D1c: approved unused root import removal exceeded')
    preserved = {}
    for family in FAMILIES:
        name = 'pure_' + family + '_expression'
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('arithmetic622D1c: unmigrated semantic family changed: ' + family)
        preserved[name] = digest(function(lib, name).encode())
    for name in ('pure_conversion_expression', 'pure_leaf_expression', 'pure_control_fragment',
                 'native_fragment_with_calls', 'expression_with_calls'):
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('arithmetic622D1c: accepted semantic owner/entry changed: ' + name)
        preserved[name] = digest(function(lib, name).encode())
    adapter = function(lib, 'pure_arithmetic_expression')
    if digest(adapter.encode()) != extension['current_arithmetic_adapter_sha256']:
        raise ValueError('arithmetic622D1c: current arithmetic adapter changed')
    old = function(previous[LIB], 'pure_arithmetic_expression')
    variants = unique_variants(adapter)
    if variants != unique_variants(old) or len(variants) != 9:
        raise ValueError('arithmetic622D1c: unique complete arithmetic operation map changed')
    require(adapter, 'fold_arithmetic(')
    require(adapter, 'pure_native::children::NativeChildren::new(parameters, circuits)')
    require(adapter, 'pure_native::leaf::NativeLeafSink::new()')
    for forbidden in ('parse_quote!', 'unsigned_maximum(', 'check_tuple_traits(',
                      'checked_unsigned_arithmetic(', 'expression_with_calls('):
        if forbidden in adapter:
            raise ValueError('arithmetic622D1c: semantic rule remained in native adapter')
    sink = text(SINK)
    appendix = extension['owners'][SINK]['predecessor_reconstruction'][0]['current']
    if sink.count(appendix) != 1 or sink.replace(appendix, '', 1) != previous[SINK]:
        raise ValueError('arithmetic622D1c: accepted leaf/conversion sink domain changed')
    require(sink, 'pub(crate) trait ArithmeticSink: LeafSink {')
    renderer_sha = renderer_relocation(text(VALUE), previous[VALUE])
    for owner in (ARITHMETIC, SINK):
        for forbidden in ('syn::', 'quote::', 'proc_macro', 'pure_native', 'NativeLeafSink',
                          'expression_with_calls(', 'native_fragment_with_calls(', 'rust_type('):
            if forbidden in text(owner):
                raise ValueError('arithmetic622D1c: neutral arithmetic acquired native checking: ' + owner)
    core = text(ARITHMETIC)
    # Both visits precede every branch, operand-kind/bound and operation guard.
    ordered(core, ('let left = children.visit(left_source, sink)?;',
                   'let right = children.visit(right_source, sink)?;',
                   'let (facts, ty) = match source {'))
    require(core, 'children.visit(', 2)
    require(core, 'sink.arithmetic(facts, left.output, right.output)')
    require(core, '#[cfg(test)]\nmod tests;')
    unsigned = core[core.index('ArithmeticSource::Unsigned {'):core.index('ArithmeticSource::Field {')]
    ordered(unsigned, ('let Type::Unsigned { max: left_max } = left.ty else',
                       'let Type::Unsigned { max: right_max } = right.ty else',
                       'checked_unsigned_arithmetic(kind, &left_max, &right_max, max)?'))
    equality = core[core.index('ArithmeticSource::Equality {'):core.index('ArithmeticSource::Ordered {')]
    ordered(equality, ('if right.ty != left.ty', 'check_tuple_traits('))
    ordered(core[core.index('ArithmeticSource::Ordered {'):],
            ('for actual in [left.ty, right.ty]', 'let Type::Unsigned { max } = &actual else',
             'unsigned_maximum(max)?', 'RenderError::UnsupportedWideUnsignedOperation'))
    for forbidden in ('plan_coercion(', 'CoercionPlan', 'recording_type_nodes(', 'DEFAULT ='):
        if forbidden in core:
            raise ValueError('arithmetic622D1c: arithmetic applicability/resource owner changed')
    native = text(FORMATTER)
    require(native, 'impl ArithmeticSink for NativeLeafSink {')
    require(native, 'render_unsigned_arithmetic(strategy, left, right)')
    equality_output = native[native.index('ArithmeticFacts::Equality { negated } =>'):
                             native.index('ArithmeticFacts::Ordered(operator) =>')]
    for anchor in ('syn::BinOp::Ne(syn::token::Ne::default())',
                   'syn::BinOp::Eq(syn::token::EqEq::default())',
                   'syn::Expr::Binary(syn::ExprBinary {', 'left: Box::new(left)',
                   'right: Box::new(right)'):
        require(equality_output, anchor)
    if 'syn::parse_quote!' in equality_output:
        raise ValueError('arithmetic622D1c: nested equality association correction removed')
    for forbidden in ('RenderError', 'ir::Expr', '&Expr', 'ir::Type', '&Type',
                      'children.visit(', 'unsigned_maximum(', 'checked_unsigned_arithmetic(',
                      'check_tuple_traits(', '.parse::<'):
        if forbidden in native:
            raise ValueError('arithmetic622D1c: native consumer acquired semantic decisions')
    tests = text(TEST_MODULE)
    require(tests, 'mod shared_arithmetic;')
    if tests.replace('mod shared_arithmetic;\n', '', 1) != previous[TEST_MODULE]:
        raise ValueError('arithmetic622D1c: existing native test module changed')
    old_routine = text(BASE + '/pure_native/tests/old_oracle/arithmetic.rs')
    if function(old_routine, 'pure_arithmetic_expression') != old:
        raise ValueError('arithmetic622D1c: old complete arithmetic oracle changed')
    old_unsigned = text(BASE + '/pure_native/tests/shared_arithmetic/old_unsigned.rs')
    if function(old_unsigned, 'unsigned_arithmetic_syntax') != function(previous[VALUE], 'unsigned_arithmetic_syntax'):
        raise ValueError('arithmetic622D1c: signed complete unsigned helper oracle changed')
    require(text(BASE + '/pure_native/tests/shared_arithmetic.rs'), 'mod workers;')
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('arithmetic622D1c: current source authority changed: ' + owner)
    return {'preserved_functions': preserved, 'unique_source_variants': sorted(variants),
            'moved_native_match_tokens_sha256': renderer_sha,
            'arithmetic_owner': 'complete shared generic arithmetic with both child visits before checks',
            'native_output': 'checked facts only; exact existing unsigned native match relocated',
            'equality_output': 'boxed binary children preserve association; deliberate legacy nested-equality parser panic correction, not blanket old/current equivalence',
            'resource_policy': 'unchanged bounded child premise/tuple operation policy/DEFAULT; no cumulative body guarantee',
            'body_syntax_bridge': 'still constructs syntax; complete syntax-free body/binding fold pending622',
            'new_retired_routes': [], 'claim': 'arithmetic-family groundwork; source inventory is not language coverage'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['arithmetic622D1c: predecessor checks failed', *prior['errors']]
            return report
        expected = {'reviewed_shared_conversion_extension': 'D1b',
                    'reviewed_shared_leaf_extension': 'D1a', 'reviewed_unit_fragment_extension': 'C1+C2'}
        if (any(prior.get(key, {}).get('issue') != 622 or prior.get(key, {}).get('slice') != stage
                for key, stage in expected.items())
                or prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627):
            raise ValueError('arithmetic622D1c: mandatory actual D1b/D1a/C/B predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('arithmetic622D1c: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('arithmetic622D1c: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('arithmetic622D1c: reviewed arithmetic closure changed')
        report['reviewed_shared_arithmetic_extension'] = {
            'issue': 622, 'slice': 'D1c', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': BASELINE_COMMIT, 'historical_manifest_count': 73,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
