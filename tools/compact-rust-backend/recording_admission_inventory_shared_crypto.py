# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive complete crypto-family authority; full syntax-free body fold pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = 'e0670f70d0cc3e2429ab7ab70b80f6054af4830a7a384b10b2df5a57ba3f5ef1'
BASELINE_COMMIT = 'a9e1d525f2902796e414f3bd006ec2e0b65b26ce'
BASE = 'tools/compact-rust-backend/src'
LIB = BASE + '/lib.rs'
SEMANTICS = BASE + '/pure_semantics.rs'
NATIVE = BASE + '/pure_native.rs'
TEST_MODULE = BASE + '/pure_native/tests.rs'
CHILDREN = BASE + '/pure_semantics/children.rs'
ENVIRONMENT = BASE + '/pure_native/children.rs'
COERCION_FORMATTER = BASE + '/value_lowering/coercion.rs'
SINK = BASE + '/pure_semantics/sink.rs'
SCOPE = BASE + '/pure_semantics/scope.rs'
OWNERSHIP = BASE + '/pure_semantics/ownership.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
HASH = BASE + '/hash_input.rs'
JUBJUB = BASE + '/jubjub_value.rs'
CRYPTO = BASE + '/pure_semantics/crypto.rs'
RULES = BASE + '/pure_semantics/crypto/check.rs'
ENCODING = BASE + '/pure_semantics/hash_encoding.rs'
EXPECTATIONS = BASE + '/pure_semantics/jubjub.rs'
FORMATTER = BASE + '/pure_native/crypto.rs'
OWNERS = {LIB, HASH, JUBJUB, SEMANTICS, NATIVE, TEST_MODULE, SINK}
FAMILIES = ('aggregate',)


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'D2'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != OWNERS):
        raise ValueError('crypto622D2: unreviewed extension or incomplete owner set')


def inverse_regions(source, row, owner):
    """Restore only exact approved regions; mutations outside them survive."""
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('crypto622D2: current inverse region changed: ' + owner)
        source = source.replace(edit['current'], edit['previous'], 1)
    return source


def predecessor_sources(root, extension):
    authenticate(extension)
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('crypto622D2: current owner changed: ' + owner)
        source = inverse_regions(source, row, owner)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('crypto622D2: signed D1c predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('crypto622D2: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 74:
        raise ValueError('crypto622D2: all 74 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.shared-crypto-622d2.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('crypto622D2: shared crypto boundary changed: ' + anchor)


def ordered(source, anchors):
    for anchor in anchors:
        require(source, anchor)
    positions = [source.index(x) for x in anchors]
    if positions != sorted(positions):
        raise ValueError('crypto622D2: first-error ordering changed')


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
        raise ValueError('crypto622D2: checked native match boundary changed')
    opening = found[0] + len(start)
    if parts[opening] != '{':
        raise ValueError('crypto622D2: checked native match opening changed')
    depth = 0
    for index in range(opening, len(parts)):
        if parts[index] == '{':
            depth += 1
        elif parts[index] == '}':
            depth -= 1
            if depth == 0:
                return parts[opening:index + 1]
    raise ValueError('crypto622D2: incomplete native match')


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib = text(LIB)
    preserved = {}
    for family in FAMILIES:
        name = 'pure_' + family + '_expression'
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('crypto622D2: unmigrated semantic family changed: ' + family)
        preserved[name] = digest(function(lib, name).encode())
    for name in ('pure_arithmetic_expression', 'pure_conversion_expression', 'pure_leaf_expression',
                 'pure_control_fragment', 'native_fragment_with_calls', 'expression_with_calls'):
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('crypto622D2: accepted semantic owner/entry changed: ' + name)
        preserved[name] = digest(function(lib, name).encode())
    adapter = function(lib, 'pure_crypto_expression')
    if digest(adapter.encode()) != extension['current_crypto_adapter_sha256']:
        raise ValueError('crypto622D2: current crypto adapter changed')
    old = function(previous[LIB], 'pure_crypto_expression')
    variants = unique_variants(adapter)
    if variants != unique_variants(old) or len(variants) != 16:
        raise ValueError('crypto622D2: unique complete crypto operation map changed')
    for anchor in ('fold_crypto(', 'pure_native::children::NativeChildren::new(parameters, circuits)',
                   'pure_native::leaf::NativeLeafSink::new()'):
        require(adapter, anchor)
    for forbidden in ('parse_quote!', 'check_tuple_traits(', 'contains_opaque(', '.lower(', 'expression_with_calls('):
        if forbidden in adapter:
            raise ValueError('crypto622D2: semantic rule remained in native adapter')
    sink = text(SINK)
    appendix = extension['owners'][SINK]['predecessor_reconstruction'][0]['current']
    if sink.count(appendix) != 1 or sink.replace(appendix, '', 1) != previous[SINK]:
        raise ValueError('crypto622D2: accepted sink domain changed')
    require(sink, 'pub(crate) trait CryptoSink: LeafSink {')
    for owner in (CRYPTO, RULES, ENCODING, EXPECTATIONS, SINK):
        for forbidden in ('syn::', 'quote::', 'proc_macro', 'pure_native', 'NativeLeafSink',
                          'expression_with_calls(', 'native_fragment_with_calls(', 'rust_type('):
            if forbidden in text(owner):
                raise ValueError('crypto622D2: neutral crypto acquired native checking: ' + owner)
    core = text(CRYPTO)
    require(core, 'children.visit(', 3)
    ordered(core, ('let left = children.visit(left, sink)?;', 'let right = children.visit(right, sink)?;',
                   'check::binary(kind, left, right)?', 'output: sink.crypto(checked.output)'))
    ordered(core, ('let value = children.visit(source, sink)?;', 'check::unary(kind, value)?'))
    for anchor in ('pub(crate) enum CheckedCrypto<V>', 'enum Inputs', 'mod check;', '#[cfg(test)]\nmod tests;'):
        require(core, anchor)
    for forbidden in ('Option<', 'unwrap(', 'check_tuple_traits(', 'contains_opaque(', '.clone()', 'DEFAULT ='):
        if forbidden in core:
            raise ValueError('crypto622D2: arity-safe child traversal boundary changed')
    rules = text(RULES)
    require(rules, 'DynamicInputEncoding::for_type(', 3)
    transient = rules[rules.index('Binary::TransientCommit =>'):rules.index('Binary::PersistentCommit =>')]
    ordered(transient, ('expect(right.ty, Type::Field)?;',
                        'check_tuple_traits(Operation::TransientCommit, &left.ty)?;',
                        'DynamicInputEncoding::for_type(&left.ty)'))
    persistent = rules[rules.index('Binary::PersistentCommit =>'):rules.index('Binary::Construct =>')]
    ordered(persistent, ('expect(right.ty, Type::Bytes { length: 32 })?;',
                         'check_tuple_traits(Operation::PersistentCommit, &left.ty)?;'))
    for kind, op in (('TransientHash', 'TransientHash'), ('HashToCurve', 'HashToCurve')):
        start = rules.index('Unary::' + kind + ' =>')
        block = rules[start:rules.index('\n        Unary::', start + 1)]
        ordered(block, ('check_tuple_traits(Operation::' + op + ', &value.ty)?;',
                        'DynamicInputEncoding::for_type(&value.ty)'))
    encoding = text(ENCODING)
    old_predicate = function(previous[HASH], 'contains_opaque')
    predicate = function(encoding, 'contains_opaque')
    if rust_tokens(predicate.replace('pub(crate) ', '', 1)) != rust_tokens(old_predicate):
        raise ValueError('crypto622D2: exact moved opaque predicate changed')
    require(encoding, 'if contains_opaque(ty)')
    hash_source = text(HASH)
    for name in ('private_output', 'witness_method'):
        if function(hash_source, name) != function(previous[HASH], name):
            raise ValueError('crypto622D2: retained transcript helper changed: ' + name)
    require(function(hash_source, 'syntax'), 'render_input(value, DynamicInputEncoding::for_type(ty))')
    rendering = function(hash_source, 'render_input')
    for anchor in ('DynamicInputEncoding::Exact => syn::parse_quote!(runtime::exact_hash_input(#value))',
                   'DynamicInputEncoding::Direct => value'):
        require(rendering, anchor)
    for forbidden in ('&Type', 'contains_opaque(', 'RenderError', 'check_tuple_traits('):
        if forbidden in rendering:
            raise ValueError('crypto622D2: hash renderer acquired semantic decisions')
    expectations = text(EXPECTATIONS)
    for anchor in ('actual: value.ty', 'Ok(value.output)', 'CheckedJubjub::Coordinate(axis, expect(point, Type::JubjubPoint)?)',
                   'CheckedJubjub::Negate(expect(point, Type::JubjubPoint)?)'):
        require(expectations, anchor)
    for anchors in (('let point = expect(point, Type::JubjubPoint)?;', 'let scalar = expect(scalar, Type::Field)?;'),
                    ('let left = expect(left, Type::JubjubPoint)?;', 'let right = expect(right, Type::JubjubPoint)?;'),
                    ('let x = expect(x, Type::Field)?;', 'let y = expect(y, Type::Field)?;')):
        ordered(expectations, anchors)
    if '.clone()' in expectations:
        raise ValueError('crypto622D2: owned Jubjub expectations changed')
    jub = text(JUBJUB)
    for enum in ('Operation', 'Coordinate'):
        marker = 'pub(crate) enum ' + enum + ' {'
        if match_body(jub, marker[:-2]) != match_body(previous[JUBJUB], marker[:-2]):
            raise ValueError('crypto622D2: retained legacy Jubjub API changed')
    for anchor in ('pub(crate) fn lower(self) -> Result<Operand, RenderError>',
                   'pub(crate) fn lower(self, point: Operand) -> Result<Operand, RenderError>',
                   'check_jubjub(operands)?', 'check_jubjub(JubjubOperands::Coordinate(axis, typed(point)))?'):
        require(jub, anchor)
    native = text(FORMATTER)
    require(native, 'impl CryptoSink for NativeLeafSink {')
    require(native, 'render_input(value, encoding)', 3)
    require(native, 'CheckedCrypto::Jubjub(checked) => jubjub_value::render(checked)')
    for output in (native, function(jub, 'render'), function(jub, 'render_coordinate')):
        for forbidden in ('RenderError', 'ir::Expr', '&Expr', 'ir::Type', '&Type',
                          'children.visit(', 'check_tuple_traits(', 'check_jubjub(', 'contains_opaque(', '.lower('):
            if forbidden in output:
                raise ValueError('crypto622D2: native consumer acquired semantic decisions')
    helpers = {}
    for owner, reference in ((HASH, 'old_hash_input'), (JUBJUB, 'old_jubjub')):
        old_source = previous[owner]
        split = old_source.index('#[cfg(test)]')
        if text(owner)[text(owner).index('#[cfg(test)]'):] != old_source[split:]:
            raise ValueError('crypto622D2: historical helper test module changed: ' + owner)
        literal = text(BASE + '/pure_native/tests/shared_crypto/' + reference + '.rs')
        # Standalone rustfmt removes only the separator blank line before cfg.
        if not old_source[:split].endswith('\n\n') or literal + '\n' != old_source[:split]:
            raise ValueError('crypto622D2: literal complete helper oracle changed: ' + reference)
        helpers[reference] = digest(literal.encode())
    tests = text(TEST_MODULE)
    require(tests, 'mod shared_crypto;')
    if tests.replace('mod shared_crypto;\n', '', 1) != previous[TEST_MODULE]:
        raise ValueError('crypto622D2: existing native test module changed')
    if function(text(BASE + '/pure_native/tests/old_oracle/crypto.rs'), 'pure_crypto_expression') != old:
        raise ValueError('crypto622D2: old complete crypto oracle changed')
    require(text(BASE + '/pure_native/tests/shared_crypto.rs'), 'mod workers;')
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('crypto622D2: current source authority changed: ' + owner)
    return {'preserved_functions': preserved, 'unique_source_variants': sorted(variants),
            'complete_helper_oracles': helpers, 'crypto_owner': 'complete sixteen-operation generic crypto; arity-safe children before owned policy',
            'dynamic_encoding': 'exact old opaque predicate; only transient hash/commit and hash-to-curve; no LedgerMap descent',
            'jubjub_owner': 'all eight expectations; legacy four Operation and two Coordinate APIs retained',
            'native_output': 'checked arity-safe values and encoding facts only; no Type inspection or refusals',
            'resource_policy': 'unchanged bounded child/tuple policy/DEFAULT; no cumulative body guarantee',
            'keccak_qualification': 'native-only explicit v3 frontend input; no ledger8 proof/cohort upgrade',
            'body_syntax_bridge': 'still constructs syntax; complete syntax-free body/binding fold pending622',
            'new_retired_routes': [], 'claim': 'crypto-family groundwork; source inventory is not language coverage'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['crypto622D2: predecessor checks failed', *prior['errors']]
            return report
        expected = {'reviewed_shared_arithmetic_extension': 'D1c',
                    'reviewed_shared_conversion_extension': 'D1b',
                    'reviewed_shared_leaf_extension': 'D1a', 'reviewed_unit_fragment_extension': 'C1+C2'}
        if (any(prior.get(key, {}).get('issue') != 622 or prior.get(key, {}).get('slice') != stage
                for key, stage in expected.items())
                or prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627):
            raise ValueError('crypto622D2: mandatory actual D1c/D1b/D1a/C/B predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('crypto622D2: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('crypto622D2: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('crypto622D2: reviewed crypto closure changed')
        report['reviewed_shared_crypto_extension'] = {
            'issue': 622, 'slice': 'D2', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': BASELINE_COMMIT, 'historical_manifest_count': 74,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
