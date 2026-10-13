# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive complete aggregate-family authority; full syntax-free body fold pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = 'd27fa3b96a1513cacae1b48068205a55c0b8332ff98a4e84206cd0aac24ded12'
BASELINE_COMMIT = 'e2bf38a5bc0373536ad967d56b11a9f04f86f3aa'
BASE = 'tools/compact-rust-backend/src'
LIB = BASE + '/lib.rs'
VALUE = BASE + '/value_lowering.rs'
SEMANTICS = BASE + '/pure_semantics.rs'
NATIVE = BASE + '/pure_native.rs'
TEST_MODULE = BASE + '/pure_native/tests.rs'
CHILDREN = BASE + '/pure_semantics/children.rs'
ENVIRONMENT = BASE + '/pure_native/children.rs'
SINK = BASE + '/pure_semantics/sink.rs'
AGGREGATE = BASE + '/pure_semantics/aggregate.rs'
DECLARATIONS = BASE + '/pure_semantics/declarations.rs'
PROJECTION = BASE + '/pure_semantics/aggregate/projection.rs'
LITERAL = BASE + '/pure_semantics/aggregate/literal.rs'
VECTOR = BASE + '/pure_semantics/aggregate/vector.rs'
CALL = BASE + '/pure_semantics/aggregate/call.rs'
FORMATTER = BASE + '/pure_native/aggregate.rs'
NATIVE_PROJECTION = BASE + '/pure_native/aggregate/projection.rs'
NATIVE_LITERAL = BASE + '/pure_native/aggregate/literal.rs'
NATIVE_VECTOR = BASE + '/pure_native/aggregate/vector.rs'
ITEM = BASE + '/pure_native/item.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
OWNERS = {LIB, VALUE, SEMANTICS, NATIVE, TEST_MODULE, SINK, CHILDREN, ENVIRONMENT}


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'D3'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != OWNERS):
        raise ValueError('aggregate622D3: unreviewed extension or incomplete owner set')


def inverse_regions(source, row, owner):
    """Restore only exact approved regions; mutations outside them survive."""
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('aggregate622D3: current inverse region changed: ' + owner)
        source = source.replace(edit['current'], edit['previous'], 1)
    return source


def predecessor_sources(root, extension):
    authenticate(extension)
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('aggregate622D3: current owner changed: ' + owner)
        source = inverse_regions(source, row, owner)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('aggregate622D3: signed D2 predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('aggregate622D3: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 75:
        raise ValueError('aggregate622D3: all 75 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.shared-aggregate-622d3.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('aggregate622D3: shared aggregate boundary changed: ' + anchor)


def ordered(source, anchors):
    for anchor in anchors:
        require(source, anchor)
    positions = [source.index(x) for x in anchors]
    if positions != sorted(positions):
        raise ValueError('aggregate622D3: first-error ordering changed')


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
        raise ValueError('aggregate622D3: checked native match boundary changed')
    opening = found[0] + len(start)
    if parts[opening] != '{':
        raise ValueError('aggregate622D3: checked native match opening changed')
    depth = 0
    for index in range(opening, len(parts)):
        if parts[index] == '{':
            depth += 1
        elif parts[index] == '}':
            depth -= 1
            if depth == 0:
                return parts[opening:index + 1]
    raise ValueError('aggregate622D3: incomplete native match')


def replace_tokens_once(tokens, before, after):
    found = [i for i in range(len(tokens) - len(before) + 1) if tokens[i:i + len(before)] == before]
    if len(found) != 1:
        raise ValueError('aggregate622D3: issue630 initial Map template slot not unique')
    index = found[0]
    return tokens[:index] + after + tokens[index + len(before):]


def output_assembly_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib, item = text(LIB), text(ITEM)
    facts = extension['output_assembly_630']
    for label, row in facts['lib_regions'].items():
        if lib.count(row['current']) != 1 or previous[LIB].count(row['previous']) != 1:
            raise ValueError('aggregate622D3: issue630 ' + label + ' assembly boundary changed')
    for forbidden in ('RenderError', 'ir::Expr', 'ir::Type', '&Type', 'check_name(',
                      'rust_type(', 'plan_coercion(', 'copy_type(', 'expression_with_calls(',
                      'native_fragment_with_calls('):
        if forbidden in item:
            raise ValueError('aggregate622D3: issue630 item/file formatter acquired semantic decisions')
    if re.search(r'#(?:body|expression|other|result|carrier|arguments|pure_items|items)\b', item):
        raise ValueError('aggregate622D3: issue630 prepared AST children re-entered parser')
    argument = function(item, 'argument')
    for anchor in ('syn::FnArg::Typed(syn::PatType {', 'pat: Box::new(syn::Pat::Ident',
                   'ident: name', 'ty: Box::new(carrier)'):
        require(argument, anchor)
    pure = function(item, 'pure')
    ordered(pure, ('item.sig.ident = name;', 'if internal', 'item.sig.inputs = arguments.into_iter().collect();',
                   'insert_result(&mut item.sig.output, result);', 'let (expression, mut statements) = body.into_views();',
                   'item.block.stmts = if unit_result', 'syn::Item::Fn(item)'))
    require(pure, 'vec![syn::Stmt::Expr(value_tail(expression), None)]')
    require(function(item, 'insert_result'), 'syn::GenericArgument::Type(result)')
    tail = function(item, 'value_tail')
    for anchor in ('syn::Expr::Try(fallible) => *fallible.expr',
                   '*call.args.first_mut().expect("Ok value template slot") = other;', 'syn::Expr::Call(call)'):
        require(tail, anchor)
    if '.clone()' in tail:
        raise ValueError('aggregate622D3: issue630 prepared value-tail move changed')
    file = function(item, 'file')
    ordered(file, ('.extend(pure_items)', 'file.items.extend(types_module)', 'file.items.extend(alias_exports)',
                   'file.items.push(pure_module)', 'file.items.extend(slots_module)', 'file.items.extend(ledger_module)'))
    require(file, 'runtime::RUST_RUNTIME_ABI == #runtime_abi')
    ordered(lib, ('preflight.finish(contract)?;', 'let mut file = pure_native::item::file(',
                  'CompactSignatureLint.visit_file_mut(&mut file);', 'CompactBooleanLiterals.visit_file_mut(&mut file);'))
    reference = text(BASE + '/pure_native/item/old_reference.rs')
    old_item = facts['lib_regions']['pure_item']['previous']
    previous_tokens = rust_tokens(old_item)
    reference_tokens = rust_tokens(reference)
    if sum(reference_tokens[i:i + len(previous_tokens)] == previous_tokens
           for i in range(len(reference_tokens) - len(previous_tokens) + 1)) != 1:
        raise ValueError('aggregate622D3: issue630 original pure-item reference changed')
    old_file = facts['lib_regions']['final_file']['previous']
    if old_file not in text(BASE + '/pure_native/item/tests/file_assembly.rs'):
        raise ValueError('aggregate622D3: issue630 original final-file reference changed')
    native_vector = text(NATIVE_VECTOR)
    native_fold = function(native_vector, 'fold')
    if native_fold != facts['unchanged_fold_function']:
        raise ValueError('aggregate622D3: issue630 accepted Fold formatter changed')
    current_map = function(native_vector, 'map')
    initial_map = facts['initial_map_function']
    if digest(current_map.encode()) != facts['current_map_function_sha256']:
        raise ValueError('aggregate622D3: issue630 current Map formatter changed')
    if digest(native_vector.replace(current_map, initial_map, 1).encode()) != facts['initial_vector_sha256']:
        raise ValueError('aggregate622D3: issue630 Map-only formatter boundary changed')
    old_prefix = initial_map[:initial_map.index('    syn::parse_quote!')]
    new_prefix = current_map[:current_map.index('    let mut block:')]
    if new_prefix != old_prefix:
        raise ValueError('aggregate622D3: issue630 Map checked-plan/setup prefix changed')
    expected = match_body(initial_map, 'syn::parse_quote!(')
    expected = replace_tokens_once(expected, ['#', 'source'], ['__compact_source_placeholder'])
    expected = replace_tokens_once(expected, ['#', 'body'], ['__compact_body_placeholder'])
    if match_body(current_map, 'syn::parse_quote!(') != expected:
        raise ValueError('aggregate622D3: issue630 shallow Map template changed')
    ordered(current_map, ('&mut block.block.stmts[0]', '.expr = source',
                          '&mut block.block.stmts[2]', '&mut mapped_loop.body.stmts[0]',
                          '*push.args.first_mut().expect("Map body argument slot") = body;', 'syn::Expr::Block(block)'))
    require(current_map, '*local')
    if 'Box::new(source)' in current_map:
        raise ValueError('aggregate622D3: issue630 Map initializer box reallocation returned')
    if re.search(r'#(?:source|body)\b', current_map) or 'body.clone()' in current_map:
        raise ValueError('aggregate622D3: issue630 prepared Map child re-entered parser')
    return {'issue': 630, 'scope': 'output-only FnArg/pureItem/finalFile/Map AST assembly; fixed shallow scaffolds only',
            'map_delta': 'exact old shallow template except two placeholders; prepared source/body moved into fixed Local/push slots',
            'fold_formatter_sha256': digest(native_fold.encode()),
            'initial_map_formatter_sha256': digest(initial_map.encode()),
            'old_reference': 'exact original pure Item and final File regions retained for small independent tests',
            'behavior_qualification': 'root authentic2MiB MapBody accepts1..44/refuses45; no cumulative memory guarantee or policy change',
            'nonaccepted_history': 'initialD3, Item-only and Item+File overflows retained separately; no old108 PASS bound to this revised source'}


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib = text(LIB)
    preserved = {}
    for name in ('pure_crypto_expression', 'pure_arithmetic_expression', 'pure_conversion_expression',
                 'pure_leaf_expression', 'pure_control_fragment', 'native_fragment_with_calls', 'expression_with_calls'):
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('aggregate622D3: accepted semantic owner/entry changed: ' + name)
        preserved[name] = digest(function(lib, name).encode())
    adapter = function(lib, 'pure_aggregate_expression')
    if digest(adapter.encode()) != extension['current_aggregate_adapter_sha256']:
        raise ValueError('aggregate622D3: current aggregate adapter changed')
    old = function(previous[LIB], 'pure_aggregate_expression')
    variants = unique_variants(adapter)
    if variants != unique_variants(old) or len(variants) != 8:
        raise ValueError('aggregate622D3: unique complete aggregate operation map changed')
    for anchor in ('fold_aggregate(', 'pure_native::children::NativeChildren::new(parameters, circuits)',
                   'pure_native::leaf::NativeLeafSink::new()'):
        require(adapter, anchor)
    for forbidden in ('parse_quote!', 'check_tuple_traits(', 'copy_type(', 'plan_coercion(',
                      'ident(', 'expression_with_calls('):
        if forbidden in adapter:
            raise ValueError('aggregate622D3: semantic rule remained in native adapter')
    for owner in (SINK, CHILDREN, ENVIRONMENT):
        appendix = extension['owners'][owner]['predecessor_reconstruction'][0]['current']
        source = text(owner)
        if source.count(appendix) != 1 or source.replace(appendix, '', 1) != previous[owner]:
            raise ValueError('aggregate622D3: existing generic protocol/environment changed: ' + owner)
    require(text(SINK), 'pub(crate) trait AggregateSink: LeafSink {')
    scoped = text(CHILDREN)[len(previous[CHILDREN]):]
    for anchor in ('pub(crate) trait ScopedChildren<S: super::sink::AggregateSink>: Children<S>',
                   'source_key: &str', 'ty: &crate::ir::Type', 'binding: &S::Binding'):
        require(scoped, anchor)
    environment = text(ENVIRONMENT)[len(previous[ENVIRONMENT]):]
    ordered(environment, ('let mut scope: HashMap<&str, (&Type, syn::Ident)> = self.bindings.clone();',
                          'scope.insert(source_key, (ty, binding.clone()));',
                          'crate::expression_with_calls(source, &scope, self.declarations)?'))
    if 'self.bindings.insert(' in environment or 'ty.clone()' in environment:
        raise ValueError('aggregate622D3: scoped native binding restore/borrow boundary changed')
    neutral = (AGGREGATE, DECLARATIONS, PROJECTION, LITERAL, VECTOR, CALL, SINK, CHILDREN)
    for owner in neutral:
        for forbidden in ('syn::', 'quote::', 'proc_macro', 'pure_native', 'NativeLeafSink',
                          'expression_with_calls(', 'native_fragment_with_calls(', 'rust_type('):
            if forbidden in text(owner):
                raise ValueError('aggregate622D3: neutral aggregate acquired native checking: ' + owner)
    declarations = text(DECLARATIONS)
    for anchor in ("parameters: &'a [Parameter]", "result: &'a Type", 'self.get(raw_source_name)',
                   'parameters: &circuit.parameters', 'result: &circuit.result'):
        require(declarations, anchor)
    for forbidden in ('.clone()', '.body', 'check_name(', 'RenderError', 'plan_coercion('):
        if forbidden in declarations:
            raise ValueError('aggregate622D3: borrowed raw signature provider acquired checking/clones')
    core = text(AGGREGATE)
    ordered(core, ('let checked = match source {', 'let checked = checked?;', 'output: sink.aggregate(checked.output)'))
    require(core, 'pub(crate) enum CheckedAggregate<V, B, C>')
    for forbidden in ('Option<', 'unwrap(', 'check_tuple_traits(', 'children.visit(', '.clone()', 'DEFAULT ='):
        if forbidden in core:
            raise ValueError('aggregate622D3: arity-safe aggregate dispatch boundary changed')
    call = text(CALL)
    ordered(call, ('.signature(name)', 'if arguments.len() != circuit.parameters.len()',
                   'let value = children.visit(source, sink)?;', 'if value.ty != parameter.ty',
                   'values.push(value.output)', 'let function = check_name(name)?;'))
    require(call, 'expected: parameter.ty.clone()')
    require(call, 'actual: value.ty')
    vector = text(VECTOR)
    mapping = function(vector, 'map')
    ordered(mapping, ('let source = children.visit(source, sink)?;',
                      'let source_plan = plan_coercion(&source.ty, &target)?;',
                      'let name = check_name(&parameter.name)?;', 'let binding = sink.binding(&name);',
                      'children.visit_bound(body, &parameter.name, &parameter.ty, &binding, sink)?',
                      'if body.ty != *result'))
    folding = function(vector, 'fold')
    ordered(folding, ('.signature(name)', 'if circuit.parameters.len() != 2',
                      'let initial = children.visit(initial, sink)?;',
                      'let initial_plan = plan_coercion(&initial.ty, accumulator)?;',
                      'let source = children.visit(source, sink)?;',
                      'let source_plan = plan_coercion(&source.ty, &target)?;',
                      'let function = check_name(name)?;'))
    for anchor in ('circuit.parameters[0].ty != *accumulator', 'circuit.parameters[1].ty != *element',
                   'circuit.result != accumulator', 'expected: accumulator.clone()', 'actual: circuit.result.clone()'):
        require(folding, anchor)
    for forbidden in ('length == 0', 'length != 0', 'BodyBudget', 'DEFAULT =', 'check_tuple_traits('):
        if forbidden in vector:
            raise ValueError('aggregate622D3: map/fold eligibility or resource premise changed')
    field = function(text(PROJECTION), 'field')
    ordered(field, ('let value = children.visit(source, sink)?;', 'let Type::Struct { fields, .. } = value.ty',
                    '.get(index)', '.filter(|declaration| declaration.name == field)',
                    'let field = check_name(field)?;', 'if copy_type(&declaration.ty)'))
    tuple = function(text(PROJECTION), 'tuple')
    ordered(tuple, ('let value = children.visit(source, sink)?;', 'let Type::Tuple { elements } = value.ty',
                    '.get(index)', '.ok_or(RenderError::InvalidTupleIndex(index))?', '.clone()'))
    structure = function(text(LITERAL), 'structure')
    ordered(structure, ('if fields.len() != declarations.len()', 'let value = children.visit(source, sink)?;',
                        'if value.ty != declaration.ty', 'let field = check_name(&declaration.name)?;',
                        'let name = check_name(name)?;'))
    vector_literal = function(text(LITERAL), 'vector')
    ordered(vector_literal, ('let value = children.visit(source, sink)?;', 'if value.ty != *element',
                            'values.push(value.output)', 'if values.is_empty()',
                            'CheckedAggregate::EmptyVector(fold_carrier(element, sink.carriers())?)'))
    for owner in (PROJECTION, LITERAL):
        if 'check_tuple_traits(' in text(owner):
            raise ValueError('aggregate622D3: ordinary projection/literal gained tuple operation cap')
    value = text(VALUE)
    compatibility = function(value, 'retained_value')
    ordered(compatibility, ('if copy_type(ty)', 'Ownership::Copy', 'Ownership::Owned',
                            'retained_value_with_ownership(value, ownership)'))
    require(compatibility, 'copy_type(', 1)
    retaining = function(value, 'retained_value_with_ownership')
    for anchor in ('crate::pure_semantics::ownership::Ownership::Copy => value',
                   'crate::pure_semantics::ownership::Ownership::Owned => syn::parse_quote!((#value).clone())'):
        require(retaining, anchor)
    for forbidden in ('&Type', 'copy_type(', 'RenderError', 'check_tuple_traits('):
        if forbidden in retaining:
            raise ValueError('aggregate622D3: retention formatter acquired semantic decisions')
    old_helper = function(text(BASE + '/pure_native/tests/shared_aggregate/old_retained.rs'), 'retained_value')
    if old_helper != function(previous[VALUE], 'retained_value'):
        raise ValueError('aggregate622D3: literal original retention helper changed')
    native = (FORMATTER, NATIVE_PROJECTION, NATIVE_LITERAL, NATIVE_VECTOR)
    for owner in native:
        for forbidden in ('RenderError', 'ir::Expr', '&Expr', 'ir::Type', '&Type',
                          'children.visit(', 'check_tuple_traits(', 'plan_coercion(', 'copy_type(',
                          'fold_carrier(', 'check_name(', 'expression_with_calls('):
            if forbidden in text(owner):
                raise ValueError('aggregate622D3: native consumer acquired semantic decisions: ' + owner)
    require(text(FORMATTER), 'impl AggregateSink for NativeLeafSink {')
    require(text(NATIVE_PROJECTION), 'retained_value_with_ownership(syn::parse_quote!((#value).#name), ownership)')
    native_tuple = function(text(NATIVE_LITERAL), 'tuple')
    ordered(native_tuple, ('tuple.elems.push_value(value)', 'tuple.elems.push_punct(syn::token::Comma::default())',
                          'if tuple.elems.len() > 1', 'tuple.elems.pop_punct()', 'syn::Expr::Tuple(tuple)'))
    native_fold = function(text(NATIVE_VECTOR), 'fold')
    ordered(native_fold, ('coercion::render(initial_plan, initial, 0)', 'coercion::render(source_plan, source, 0)'))
    ordered(native_fold, ('let #source_name = #source;', 'let mut #acc = #initial;',
                         'for #item in #source_name.into_array()', '#acc = crate::pure_circuits::#function(#acc, #item)?;'))
    assembly = output_assembly_closure(root, extension, previous)
    tests = text(TEST_MODULE)
    require(tests, 'mod shared_aggregate;')
    require(tests, 'mod item_assembly;')
    if tests.replace('mod shared_aggregate;\n', '', 1).replace('mod item_assembly;\n', '', 1) != previous[TEST_MODULE]:
        raise ValueError('aggregate622D3: existing native test module changed')
    if function(text(BASE + '/pure_native/tests/old_oracle/aggregate.rs'), 'pure_aggregate_expression') != old:
        raise ValueError('aggregate622D3: old complete aggregate oracle changed')
    for anchor in ('mod workers;', 'mod runtime_probe;'):
        require(text(BASE + '/pure_native/tests/shared_aggregate.rs'), anchor)
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('aggregate622D3: current source authority changed: ' + owner)
    return {'preserved_functions': preserved, 'unique_source_variants': sorted(variants),
            'output_assembly_630': assembly,
            'retained_helper_sha256': digest(old_helper.encode()),
            'aggregate_owner': 'complete eight-operation generic aggregate with separate scoped visitor and borrowed signatures',
            'signature_policy': 'raw lookup; borrowed parameter/result roots; no callee body/purity checks or provider clones',
            'scope_policy': 'original Children protocol/native visit unchanged; short raw-key borrowed binding overlay restores outer scope',
            'fold_static_order': 'signature then initial child/plan then source child/plan then checked name',
            'fold_runtime_order': 'source initializer before initial initializer; per-item accumulation unchanged',
            'native_output': 'checked names, ownership, carrier and coercion facts only; exact old retention output helper',
            'resource_policy': 'unchanged bounded child/declaration premise/DEFAULT; no new private preflight or cumulative body guarantee',
            'body_syntax_bridge': 'still constructs syntax; generic control/body-binding/full CheckSink pending622',
            'new_retired_routes': [], 'claim': 'aggregate-family groundwork; source inventory is not language coverage'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['aggregate622D3: predecessor checks failed', *prior['errors']]
            return report
        expected = {'reviewed_shared_crypto_extension': 'D2',
                    'reviewed_shared_arithmetic_extension': 'D1c',
                    'reviewed_shared_conversion_extension': 'D1b',
                    'reviewed_shared_leaf_extension': 'D1a', 'reviewed_unit_fragment_extension': 'C1+C2'}
        if (any(prior.get(key, {}).get('issue') != 622 or prior.get(key, {}).get('slice') != stage
                for key, stage in expected.items())
                or prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627):
            raise ValueError('aggregate622D3: mandatory actual D2/D1c/D1b/D1a/C/B predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('aggregate622D3: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('aggregate622D3: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('aggregate622D3: reviewed aggregate closure changed')
        report['reviewed_shared_aggregate_extension'] = {
            'issue': 622, 'slice': 'D3', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': BASELINE_COMMIT, 'historical_manifest_count': 75,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
