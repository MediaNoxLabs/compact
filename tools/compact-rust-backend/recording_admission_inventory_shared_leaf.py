# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive complete leaf-family authority; full syntax-free body fold pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = '2683ff605442d817df3a7c1c22d3f068bb75d4d276f399140b13c1091b8c6696'
BASELINE_COMMIT = '3242095adc0fc3f8d0ab70df0740d2f3ce1375aa'
BASE = 'tools/compact-rust-backend/src'
LIB = BASE + '/lib.rs'
VALUE = BASE + '/value_lowering.rs'
SEMANTICS = BASE + '/pure_semantics.rs'
NATIVE = BASE + '/pure_native.rs'
TEST_MODULE = BASE + '/pure_native/tests.rs'
LEAF = BASE + '/pure_semantics/leaf.rs'
SINK = BASE + '/pure_semantics/sink.rs'
SCOPE = BASE + '/pure_semantics/scope.rs'
OWNERSHIP = BASE + '/pure_semantics/ownership.rs'
FORMATTER = BASE + '/pure_native/leaf.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
OWNERS = {LIB, VALUE, SEMANTICS, NATIVE, TEST_MODULE}
FAMILIES = ('aggregate', 'conversion', 'arithmetic', 'crypto')


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'D1a'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != OWNERS):
        raise ValueError('leaf622D1a: unreviewed extension or incomplete owner set')


def inverse_regions(source, row, owner):
    """Restore only exact approved regions; mutations outside them survive."""
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('leaf622D1a: current inverse region changed: ' + owner)
        source = source.replace(edit['current'], edit['previous'], 1)
    return source


def predecessor_sources(root, extension):
    authenticate(extension)
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('leaf622D1a: current owner changed: ' + owner)
        source = inverse_regions(source, row, owner)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('leaf622D1a: signed C predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('leaf622D1a: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 71:
        raise ValueError('leaf622D1a: all 71 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.shared-leaf-622d1a.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('leaf622D1a: shared leaf boundary changed: ' + anchor)


def ordered(source, anchors):
    for anchor in anchors:
        require(source, anchor)
    positions = [source.index(x) for x in anchors]
    if positions != sorted(positions):
        raise ValueError('leaf622D1a: first-error ordering changed')


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib, value = text(LIB), text(VALUE)
    preserved = {}
    for family in FAMILIES:
        name = 'pure_' + family + '_expression'
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('leaf622D1a: unmigrated semantic family changed: ' + family)
        preserved[name] = digest(function(lib, name).encode())
    adapter = function(lib, 'pure_leaf_expression')
    if digest(adapter.encode()) != extension['current_leaf_adapter_sha256']:
        raise ValueError('leaf622D1a: current leaf adapter changed')
    old_leaf = function(previous[LIB], 'pure_leaf_expression')
    if re.findall(r'Expr::(\w+)', adapter) != re.findall(r'Expr::(\w+)', old_leaf):
        raise ValueError('leaf622D1a: complete leaf source variant/refusal map changed')
    require(adapter, 'let typed = fold_leaf(')
    require(adapter, '&mut pure_native::leaf::NativeLeafSink::new(),')
    for forbidden in ('parse_quote!', 'field_literal_bytes(', 'unsigned_maximum(', 'rust_type(', 'copy_type('):
        if forbidden in adapter:
            raise ValueError('leaf622D1a: legacy leaf semantic decision in adapter: ' + forbidden)
    ownership = text(OWNERSHIP)
    if function(ownership, 'copy_type') != function(previous[VALUE], 'copy_type'):
        raise ValueError('leaf622D1a: original ownership policy changed')
    if function(value, 'copy_type') != ('pub(crate) fn copy_type(ty: &Type) -> bool {\n'
                                      '    crate::pure_semantics::ownership::copy_type(ty)\n}'):
        raise ValueError('leaf622D1a: ownership forwarding changed')
    neutral = {owner: text(owner) for owner in (LEAF, SINK, SCOPE, OWNERSHIP)}
    for owner, source in neutral.items():
        for forbidden in ('syn::', 'quote::', 'proc_macro', 'pure_native', 'NativeLeafSink',
                          'rust_type(', 'expression_with_calls(', 'native_fragment_with_calls('):
            if forbidden in source:
                raise ValueError('leaf622D1a: neutral semantic owner acquired native checking: ' + owner)
    leaf = neutral[LEAF]
    require(leaf, 'fold_carrier(ty, sink.carriers())?;', 2)
    require(leaf, 'output: sink.leaf(facts),')
    require(leaf, 'LeafSource::Effectful => return Err(RenderError::EffectfulExpression),')
    require(leaf, '#[cfg(test)]\nmod tests;')
    ordered(leaf, ('let (ty, binding) = bindings',
                   '.ok_or_else(|| RenderError::UnknownParameter(name.to_owned()))?;',
                   'drop(recording_type_nodes(ty)?);', 'let ownership = if copy_type(ty)',
                   '(CheckedLeaf::Parameter { binding, ownership }, ty.clone())'))
    enum_arm = leaf[leaf.index('LeafSource::EnumVariant { ty, variant } =>'):]
    ordered(enum_arm, ('let Type::Enum { variants, .. } = ty else',
                       'if !variants.iter().any(|member| member == variant)',
                       'let carrier = fold_carrier(ty, sink.carriers())?;',
                       'let variant = check_name(variant)?;'))
    ordered(function(leaf, 'unsigned_literal'), ('let maximum = unsigned_maximum(max)?;',
            'let bytes = field_literal_bytes(value).map_err(|_| invalid())?;',
            'if (high, low) > maximum_limbs', 'Ok(UnsignedLiteralFacts::new(maximum, value, max, bytes))'))
    field_arm = leaf[leaf.index('LeafSource::FieldLiteral(value) =>'):leaf.index('LeafSource::BytesLiteral(bytes) =>')]
    ordered(field_arm, ('let bytes = field_literal_bytes(value)?;', 'value.parse::<u128>()'))
    scope = neutral[SCOPE]
    require(scope, 'impl<B> Bindings<B> for HashMap<&str, (&Type, B)>')
    require(scope, 'HashMap::get(self, source_name).map(|(ty, binding)| (*ty, binding))')
    if '.clone()' in scope or 'check_name(' in scope:
        raise ValueError('leaf622D1a: borrowed source binding scope changed')
    formatter = text(FORMATTER)
    for forbidden in ('RenderError', 'ir::Expr', 'ir::Type', '&Type', 'fold_leaf(',
                      'fold_carrier(', 'rust_type(', 'unsigned_maximum(', 'field_literal_bytes(',
                      'check_name(', 'copy_type(', 'recording_type_nodes(', '.parse::<'):
        if forbidden in formatter:
            raise ValueError('leaf622D1a: native formatter acquired semantic checking: ' + forbidden)
    require(formatter, 'type Value = syn::Expr;')
    require(formatter, 'type Binding = syn::Ident;')
    require(formatter, 'fn leaf(&mut self, facts: CheckedLeaf<')
    require(formatter, 'byte_literals(&facts.bytes()[..31])')
    require(text(TEST_MODULE), 'mod shared_leaf;')
    if text(TEST_MODULE).replace('mod shared_leaf;\n', '', 1) != previous[TEST_MODULE]:
        raise ValueError('leaf622D1a: existing native test module changed')
    # Hash every new core/consumer/test after the structural role checks. This
    # also catches syntax-free but altered semantic rules or output spelling.
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('leaf622D1a: current source authority changed: ' + owner)
    return {'preserved_semantic_functions': preserved,
            'leaf_source_variants': len(re.findall(r'Expr::(\w+)', adapter)),
            'leaf_owner': 'one generic shared leaf checker; checked facts consumed by native sink',
            'ownership': 'exact old copy policy; forwarding wrapper; borrowed source-key bindings',
            'private_parameter_policy': 'lookup first; resource-only annotation bounds before ownership/result clone; bounded malformed annotations preserved',
            'body_syntax_bridge': 'still constructs syntax; complete syntax-free body/binding fold pending622',
            'new_retired_routes': [],
            'claim': 'complete leaf-family groundwork; source inventory is not language coverage'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['leaf622D1a: predecessor checks failed', *prior['errors']]
            return report
        stage = prior.get('reviewed_unit_fragment_extension', {})
        if (stage.get('issue') != 622 or stage.get('slice') != 'C1+C2'
                or prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627):
            raise ValueError('leaf622D1a: mandatory actual C/B predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('leaf622D1a: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('leaf622D1a: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('leaf622D1a: reviewed shared leaf closure changed')
        report['reviewed_shared_leaf_extension'] = {
            'issue': 622, 'slice': 'D1a', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': BASELINE_COMMIT, 'historical_manifest_count': 71,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
