# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Stateful/constructor tuple obligations; zero recording-route retirements."""
import copy

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = 'e82bb20cd0dc1cf614458c674b49eec950c4dd5fc33eea3eaef8f934549d5657'
BASELINE_COMMIT = 'f88a3a778454d879c6177950597a5d690504be06'
BASE = 'tools/compact-rust-backend/src'
OWNER = BASE + '/stateful/expression.rs'
POLICY = BASE + '/native_representation.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 624 or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []):
        raise ValueError('stateful624: unreviewed extension')
    if set(extension['owners']) != {OWNER}:
        raise ValueError('stateful624: complete expression owner required')
    row = extension['owners'][OWNER]
    source = (root / OWNER).read_bytes().decode()
    if digest(source.encode()) != row['current_sha256']:
        raise ValueError('stateful624: current expression owner changed')
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('stateful624: inverse is not unique')
        source = source.replace(edit['current'], edit['previous'], 1)
    if digest(source.encode()) != row['previous_sha256']:
        raise ValueError('stateful624: complete signed predecessor differs')
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('stateful624: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 66:
        raise ValueError('stateful624: all 66 scoped historical manifests required')
    return {OWNER: source}


class _Checkpoint:
    """Authenticate complete current source before yielding the signed owner."""
    def __init__(self, root, extension):
        self._root = root
        self._sources = predecessor_sources(root, extension)

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('stateful624: operation boundary changed: ' + anchor)


def ordered(source, anchors, label):
    positions = [source.index(anchor) for anchor in anchors]
    if positions != sorted(positions):
        raise ValueError('stateful624: ' + label + ' order changed')


def source_closure(root, extension):
    source = (root / OWNER).read_bytes().decode()
    require(source, 'use crate::native_representation;')
    require(source, 'native_representation::check_tuple_traits(', 6)
    require(source, '#[cfg(test)]\nmod tuple_capabilities;')
    start = source.index('        Expr::Equal { left, right } | Expr::NotEqual { left, right } => {',
                         source.index('fn render_arithmetic_operation('))
    equality = source[start:source.index('        Expr::Compare {', start)]
    for op in ('Equal', 'NotEqual'):
        require(equality, 'native_representation::Operation::' + op)
    ordered(equality, ['let (left, left_ty, left_effect) = render_state_expression(',
                      'statements.push(syn::parse_quote!(let #left_name = #left;));',
                      'let (right, right_ty, right_effect) = render_state_expression(',
                      'if right_ty != left_ty {',
                      'native_representation::check_tuple_traits(operation, &left_ty)?;',
                      'syn::parse_quote!(#left_name == #right)'], 'equality evaluation/mismatch/trait')
    crypto = source[source.index('fn render_crypto_operation('):]
    unary_end = crypto.index('        Expr::TransientCommit {\n')
    unary = crypto[:unary_end]
    ordered(unary, ['let (rendered, actual, effect) = render_state_expression(',
                    'let (operation, result): (syn::Path, Type) = match value {',
                    'native_representation::check_tuple_traits(',
                    'crate::hash_input::syntax(rendered, &actual)',
                    'syn::parse_quote!(#operation(#rendered))'], 'unary operand/trait/invocation')
    operations = {'TransientHash': 'transient_hash', 'PersistentHash': 'persistent_hash',
                  'Keccak256': 'keccak256', 'HashToCurve': 'hash_to_curve'}
    for op, runtime in operations.items():
        begin = unary.index('                Expr::' + op + ' { .. } => {')
        end = unary.find('\n                Expr::', begin + 1)
        body = unary[begin:end if end >= 0 else len(unary)]
        require(body, 'native_representation::Operation::' + op)
        require(body, 'native_representation::check_tuple_traits(')
        ordered(body, ['native_representation::check_tuple_traits(',
                       'syn::parse_quote!(runtime::' + runtime + ')'], op + ' trait/path')
    commit = crypto[unary_end:crypto.index('        Expr::EcMulGenerator { scalar } => {', unary_end)]
    for op in ('TransientCommit', 'PersistentCommit'):
        require(commit, 'native_representation::Operation::' + op)
    ordered(commit, ['let (input, input_type, input_effect) = render_state_expression(',
                     'statements.push(syn::parse_quote!(let #input_name = #input;));',
                     'let (opening, actual, opening_effect) = render_state_expression(',
                     'if actual != expected {',
                     'native_representation::check_tuple_traits(tuple_operation, &input_type)?;',
                     'crate::hash_input::syntax(syn::parse_quote!(#input_name), &input_type)',
                     'syn::parse_quote!(#operation(#input, #opening))'], 'commitment evaluation/opening/trait')
    return {'owner_sha256': digest(source.encode()),
            'shared_policy_sha256': digest((root / POLICY).read_bytes()),
            'native_guard_sites': 6, 'checked_operations': 8,
            'equality_tuple_limit': 12, 'crypto_tuple_limit': 11,
            'resource_policy': 'unchanged #621 borrowed DEFAULT policy',
            'diagnostic_policy': 'operand/mismatch/opening errors precede shared tuple trait refusal',
            'evaluation_policy': 'left/input temporary retained before right/opening evaluation',
            'claim': 'stateful and constructor emitter obligations; no recording mechanism/profile or ledger cohort change'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, extension))
        if not prior['passed']:
            report['errors'] = ['stateful624: predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('stateful624: existing route denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_text())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('stateful624: recording authority changed')
        closure = source_closure(root, extension)
        if closure != extension['production_closure']:
            raise ValueError('stateful624: reviewed emitter closure changed')
        report['reviewed_stateful_tuple_extension'] = {
            'issue': 624, 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': extension['baseline_commit'],
            'historical_manifest_count': 66, 'original_oracles': 49,
            'production_closure': closure, 'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
