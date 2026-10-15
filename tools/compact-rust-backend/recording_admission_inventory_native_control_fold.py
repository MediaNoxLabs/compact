# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive complete native expression/control fold; formal body checker pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = 'cad45e8a884a0923356c242fd11a2018fdcd228b7dd7b7b7f3d0b7d3a328d382'
BASELINE_COMMIT = '05fa04d8dfab866b08e3cd3af433def8b12f26e4'
BASE = 'tools/compact-rust-backend/src'
LIB = BASE + '/lib.rs'
NATIVE = BASE + '/pure_native.rs'
SEMANTICS = BASE + '/pure_semantics.rs'
TEST_MODULE = BASE + '/pure_native/tests.rs'
CHILDREN = BASE + '/pure_semantics/children.rs'
SCOPE = BASE + '/pure_semantics/scope.rs'
SINK = BASE + '/pure_semantics/sink.rs'
EXPRESSION = BASE + '/pure_semantics/expression.rs'
SOURCE = BASE + '/pure_semantics/expression/source.rs'
VISITOR = BASE + '/pure_semantics/visitor.rs'
CONTROL = BASE + '/pure_semantics/control.rs'
NATIVE_SINK = BASE + '/pure_native/sink.rs'
NATIVE_VALUE = BASE + '/pure_native/value.rs'
IR = BASE + '/ir.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
OWNERS = {LIB, NATIVE, SEMANTICS, TEST_MODULE, CHILDREN, SCOPE, SINK}
FAMILIES = ('aggregate', 'conversion', 'arithmetic', 'crypto', 'leaf')


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'D4b'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != OWNERS):
        raise ValueError('native622D4b: unreviewed extension or incomplete owner set')


def inverse_regions(source, row, owner):
    """Restore only exact approved regions; mutations outside them survive."""
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('native622D4b: current inverse region changed: ' + owner)
        source = source.replace(edit['current'], edit['previous'], 1)
    return source


def predecessor_sources(root, extension):
    authenticate(extension)
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('native622D4b: current owner changed: ' + owner)
        source = inverse_regions(source, row, owner)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('native622D4b: signed D4a predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('native622D4b: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 77:
        raise ValueError('native622D4b: all 77 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.native-control-fold-622d4b.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('native622D4b: shared native expression boundary changed: ' + anchor)


def ordered(source, anchors):
    for anchor in anchors:
        require(source, anchor)
    positions = [source.index(x) for x in anchors]
    if positions != sorted(positions):
        raise ValueError('native622D4b: first-error ordering changed')


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
        raise ValueError('native622D4b: checked native match boundary changed')
    opening = found[0] + len(start)
    if parts[opening] != '{':
        raise ValueError('native622D4b: checked native match opening changed')
    depth = 0
    for index in range(opening, len(parts)):
        if parts[index] == '{':
            depth += 1
        elif parts[index] == '}':
            depth -= 1
            if depth == 0:
                return parts[opening:index + 1]
    raise ValueError('native622D4b: incomplete native match')


def declared_expr_variants(source):
    start = source.index('pub enum Expr {')
    opening = source.index('{', start)
    depth = 1
    for end in range(opening + 1, len(source)):
        depth += (source[end] == '{') - (source[end] == '}')
        if depth == 0:
            return set(re.findall(r'(?m)^    ([A-Z]\w*)(?:,| \{)', source[start:end + 1]))
    raise ValueError('native622D4b: incomplete IR Expr enum')


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib = text(LIB)
    compatibility = {}
    for family in FAMILIES:
        name = 'pure_' + family + '_expression'
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('native622D4b: exact compatibility adapter body changed: ' + name)
        require(lib, '#[cfg(test)]\nfn ' + name + '(')
        compatibility[name] = digest(function(lib, name).encode())
    for forbidden in ('fn pure_control_fragment(', 'fn keep_expression(', 'fn keep_fragment(',
                      'fn expression_from_fragment(', 'fn fragment_from_expression(', 'pure_native::dispatch::'):
        if forbidden in lib:
            raise ValueError('native622D4b: retired native semantic dispatch/control returned')
    for name in ('native_fragment_with_calls', 'expression_with_calls'):
        wrapper = function(lib, name)
        if digest(wrapper.encode()) != extension['wrapper_sha256'][name]:
            raise ValueError('native622D4b: current full-fold wrapper changed: ' + name)
        require(wrapper, 'pure_semantics::expression::fold_expression(')
        require(wrapper, '&mut pure_native::sink::NativeSink::new()')
        for forbidden in ('match expr', 'Expr::', 'parse_quote!', 'rust_type(', 'NativeChildren', 'dispatch!'):
            if forbidden in wrapper:
                raise ValueError('native622D4b: native fallback/checker entered full-fold wrapper')
    for name in ('condition_needs_statement', 'public_parameter_idents'):
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('native622D4b: existing source predicate/signature owner changed: ' + name)
    native = text(NATIVE)
    if 'mod dispatch;' in native:
        raise ValueError('native622D4b: historical dispatch module became registered')
    require(native, '#[cfg(test)]\npub(crate) mod children;')
    for owner in (EXPRESSION, SOURCE, VISITOR, CONTROL, SCOPE, SINK):
        for forbidden in ('syn::', 'quote::', 'proc_macro', 'pure_native', 'NativeLeafSink', 'NativeSink',
                          'NativeChildren', 'expression_with_calls(', 'native_fragment_with_calls(', 'rust_type('):
            if forbidden in text(owner):
                raise ValueError('native622D4b: neutral fold acquired native fallback: ' + owner)
    classification = text(SOURCE)
    variants = set(re.findall(r'(?<!:)\bExpr::(\w+)', classification))
    declared = declared_expr_variants(text(IR))
    if variants != declared or any(len(re.findall(r'(?<!:)\bExpr::' + re.escape(name) + r'\b', classification)) != 1 for name in declared):
        raise ValueError('native622D4b: complete explicit IR variant map changed')
    if re.search(r'\b_\s*=>', classification):
        raise ValueError('native622D4b: wildcard source fallback returned')
    expression = text(EXPRESSION)
    ordered(expression, ('let typed = match source::classify(expr)', 'Source::Control(source) => return fold_control(',
                         '}?;', 'let view = if typed.ty == Type::Unit', 'output: sink.ordinary(view, typed.output)'))
    require(expression, 'Expr::Unit | Expr::Default { ty: Type::Unit }')
    for family in FAMILIES:
        require(expression, 'fold_' + family + '(')
    require(expression, 'ExpressionVisitor::new(bindings, declarations)', 4)
    visitor = text(VISITOR)
    for anchor in ('fold_expression(source, self.bindings, self.declarations, sink)',
                   'let mut scope = LocalScope::new(self.bindings);',
                   'scope.insert(source_key, ty, binding.clone());',
                   'fold_expression(source, scope.bindings(), self.declarations, sink)'):
        require(visitor, anchor)
    scope = text(SCOPE)
    for anchor in ("pub(crate) type Scope<'a, B> = HashMap<&'a str, (&'a Type, B)>;",
                   'bindings: bindings.clone()', 'self.bindings.insert(key, (annotation, binding));'):
        require(scope, anchor)
    for forbidden in ('annotation.clone()', 'ty.clone()', 'check_name(', 'RenderError', 'check_tuple_traits('):
        if forbidden in scope:
            raise ValueError('native622D4b: lexical metadata session acquired type clone/checking')
    control = text(CONTROL)
    assert_case = control[control.index('ControlSource::Assert { condition, message } =>'):control.index('ControlSource::Sequence { steps, value } =>')]
    ordered(assert_case, ('children.visit(condition, sink)?', 'expect(condition.ty, Type::Boolean)?'))
    sequence = control[control.index('ControlSource::Sequence { steps, value } =>'):control.index('ControlSource::If {', control.index('ControlSource::Sequence { steps, value } =>'))]
    ordered(sequence, ('children.visit(step, sink)?', 'expect(step.ty, Type::Unit)?', 'values.push(step.output)', 'children.visit(value, sink)?'))
    condition = control[control.index('let condition_value ='):control.index('ControlSource::Let { .. } =>')]
    ordered(condition, ('children.visit(condition, sink)?', 'expect(condition_value.ty, Type::Boolean)?',
                        'children.visit(then, sink)?', 'children.visit(otherwise, sink)?',
                        'if then_value.ty != otherwise_value.ty', 'equal_arms: then == otherwise'))
    lexical = function(control, 'lexical')
    ordered(lexical, ('let mut scope = LocalScope::new(outer);', 'let name = check_name(&binding.name)?;',
                      '.visit(&binding.value, sink)?', 'if value.ty != binding.ty',
                      'let carrier = fold_carrier(&binding.ty, sink.carriers())?;', 'let local = sink.local_binding(',
                      'scope.insert(binding.name.as_str(), &binding.ty, local);', '.visit(body, sink)?'))
    require(lexical, 'binding: local.clone()')
    for name in ('LeafSink', 'ConversionSink', 'ArithmeticSink', 'CryptoSink', 'AggregateSink', 'ControlSink'):
        require(text(SINK), name + ' + ' if name != 'ControlSink' else '+ ControlSink')
    native_sink = text(NATIVE_SINK)
    # Final test-only observation hook is not in the production consumer closure.
    production_sink = native_sink.split('    #[cfg(test)]', 1)[0]
    for owner, source in ((NATIVE_SINK, production_sink), (NATIVE_VALUE, text(NATIVE_VALUE))):
        for forbidden in ('RenderError', 'ir::Expr', 'ir::Type', '&Type', 'check_name(', 'check_tuple_traits(',
                          'plan_coercion(', 'copy_type(', 'fold_expression(', 'ExpressionVisitor',
                          'expression_with_calls(', 'native_fragment_with_calls('):
            if forbidden in source:
                raise ValueError('native622D4b: native fragment/value consumer acquired semantic checking: ' + owner)
    require(native_sink, 'type Value = NativeFragment;')
    require(native_sink, 'super::local::allocate(name, raw_key, occupied)')
    for anchor in ('self.syntax.crypto(value::crypto(checked))', 'self.syntax.aggregate(value::aggregate(checked))',
                   'super::fragment::sequence(statements, value)', 'super::fragment::lexical(statements, body)',
                   'super::control::conditional(', 'super::control::assertion('):
        require(native_sink, anchor)
    for anchor in ('OrdinaryView::EmptyUnit => NativeFragment::unit(value.into_expression(), Vec::new())',
                   'OrdinaryView::ExecutableUnit =>', 'syn::parse_quote!(#expression;)'):
        require(native_sink, anchor)
    tests = text(TEST_MODULE)
    require(tests, 'mod control_fold;')
    if tests.replace('mod control_fold;\n', '', 1) != previous[TEST_MODULE]:
        raise ValueError('native622D4b: existing native test module changed')
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('native622D4b: protected current semantic/output source changed: ' + owner)
    return {'compatibility_test_only_adapters': compatibility, 'explicit_ir_variants': sorted(variants),
            'production_recursion': 'two thin native wrappers -> complete fold -> same generic visitor/scoped recursion; no native dispatch/family fallback',
            'controls': 'all four controls typed in shared owner; equal arms both visited; Sequence Unit before later child',
            'scope': 'concrete borrowed LocalScope metadata map; checked sequential Let/name/carrier/raw insertion order; no Type/declaration clone to create scope',
            'ordinary_views': 'one post-type projection; explicit Unit/Default Unit empty, other inferredUnit executable; controls retain complete dual views',
            'native_output': 'NativeFragment consumer delegates existing checked family/control formatters; value-only maps never recheck or recurse',
            'resource_policy': 'unchanged DEFAULT/public bounded premise; actual fresh native worker cycles required, no cumulative memory guarantee',
            'body_syntax_bridge': 'native full fold complete; certificate Syntax bridge remains, production BodyCheckSink pending622',
            'new_retired_routes': [], 'claim': 'native full-fold source authority, not language/spec/formal coverage or recording retirement'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['native622D4b: predecessor checks failed', *prior['errors']]
            return report
        expected = {'reviewed_shared_aggregate_extension': 'D3',
                    'reviewed_shared_crypto_extension': 'D2',
                    'reviewed_shared_arithmetic_extension': 'D1c',
                    'reviewed_shared_conversion_extension': 'D1b',
                    'reviewed_shared_leaf_extension': 'D1a', 'reviewed_unit_fragment_extension': 'C1+C2'}
        if (any(prior.get(key, {}).get('issue') != 622 or prior.get(key, {}).get('slice') != stage
                for key, stage in expected.items())
                or prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627
                or prior.get('reviewed_lexical_symbols_extension', {}).get('issue') != 629
                or prior.get('reviewed_lexical_symbols_extension', {}).get('slice') != 'D4a'):
            raise ValueError('native622D4b: mandatory actual D4a/D3/D2/D1c/D1b/D1a/C/B predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('native622D4b: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('native622D4b: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('native622D4b: reviewed native fold closure changed')
        report['reviewed_native_control_fold_extension'] = {
            'issue': 622, 'slice': 'D4b', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': BASELINE_COMMIT, 'historical_manifest_count': 77,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
