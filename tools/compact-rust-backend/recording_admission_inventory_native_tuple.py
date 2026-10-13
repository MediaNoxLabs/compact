# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Native operation tuple diagnostics; no recording retirement or coverage claim."""
import copy
import json

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '08a388b2df96765d815ebcbb2a67146eb68e79c0f7fd77e510669b203a66f241'
BASELINE_COMMIT = '1c2489549eae7a75895f7f6721d8993404554ea2'
BASE = 'tools/compact-rust-backend/src'
OWNER = BASE + '/lib.rs'
POLICY = BASE + '/native_representation.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 621 or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []):
        raise ValueError('native621: unreviewed extension')
    if set(extension['owners']) != {OWNER}:
        raise ValueError('native621: complete native owner required')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_text()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('native621: current source changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('native621: inverse is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('native621: signed predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('native621: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 65:
        raise ValueError('native621: all 65 scoped historical manifests required')
    return sources


class _Checkpoint:
    """Authenticate the whole current source before yielding one signed owner."""
    def __init__(self, root, extension):
        self._root = root
        self._sources = predecessor_sources(root, extension)

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    """Historical laws consume an authenticated newest checkpoint, if present."""
    manifest = baseline_path.with_name('recording_admission_inventory.native-tuple-621.json')
    return checkpoint(root, json.loads(manifest.read_text())) if manifest.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('native621: operation/resource boundary changed: ' + anchor)


def source_closure(root, extension):
    source = (root / OWNER).read_text()
    policy = (root / POLICY).read_text()
    require(source, 'mod native_representation;')
    require(source, 'native_representation::check_tuple_traits(', 7)
    require(source, 'let preflight = resource_limits::validate(contract)?;')
    require(source, 'preflight.finish(contract)?;')
    for anchor in ('UnsupportedTupleOperation {',):
        require(source, anchor, 2)
    equality_start = source.index('        Expr::Equal { left, right } | Expr::NotEqual { left, right } => {',
                                  source.index('fn pure_arithmetic_expression('))
    equality = source[equality_start:source.index('        Expr::Compare {', equality_start)]
    require(equality, 'native_representation::Operation::Equal')
    require(equality, 'native_representation::Operation::NotEqual')
    order = ['expression_with_calls(left, parameters, circuits)?',
             'expression_with_calls(right, parameters, circuits)?',
             'if right_ty != left_ty {',
             'native_representation::check_tuple_traits(operation, &left_ty)?;',
             'syn::parse_quote!(#left == #right)']
    if [equality.index(anchor) for anchor in order] != sorted(equality.index(anchor) for anchor in order):
        raise ValueError('native621: equality operand/mismatch/trait/syntax order changed')
    crypto = source[source.index('fn pure_crypto_expression('):source.index('        Expr::DegradeToTransient', source.index('fn pure_crypto_expression('))]
    # HashToCurve follows the unchanged conversion operations.
    crypto += source[source.index('        Expr::HashToCurve { value } => {', source.index('fn pure_crypto_expression(')):source.index('        Expr::JubjubPointX', source.index('fn pure_crypto_expression('))]
    operations = ('TransientHash', 'TransientCommit', 'PersistentHash', 'PersistentCommit', 'Keccak256', 'HashToCurve')
    for op in operations:
        marker = '        Expr::' + op + ' { value'
        begin = crypto.index(marker)
        end = crypto.find('\n        Expr::', begin + len(marker))
        body = crypto[begin:end if end >= 0 else len(crypto)]
        require(body, 'native_representation::Operation::' + op)
        require(body, 'native_representation::check_tuple_traits(')
        if body.index('expression_with_calls(value, parameters, circuits)?') >= body.index('native_representation::check_tuple_traits('):
            raise ValueError('native621: value error ordering changed: ' + op)
        if body.index('native_representation::check_tuple_traits(') >= body.index('syn::parse_quote!('):
            raise ValueError('native621: trait obligation after syntax: ' + op)
        if op.endswith('Commit'):
            order = ['expression_with_calls(value, parameters, circuits)?',
                     'expression_with_calls(opening, parameters, circuits)?',
                     'if actual != ', 'native_representation::check_tuple_traits(']
            if [body.index(anchor) for anchor in order] != sorted(body.index(anchor) for anchor in order):
                raise ValueError('native621: commitment operand/opening/trait order changed: ' + op)
    for anchor in ('Self::Equal | Self::NotEqual => 12,', '_ => 11,',
                   'let nodes = recording_type_nodes(ty)?;', 'let limit = operation.tuple_limit();',
                   'for node in nodes {', 'if let Type::Tuple { elements } = node',
                   '&& elements.len() > limit', 'Self::Equal | Self::NotEqual => "PartialEq",',
                   'Self::Keccak256 => "Into<Value>",', '"HashInputValue"',
                   '_ => "Into<Value> + Aligned",',
                   'Type::OpaqueString | Type::OpaqueBytes', '#[cfg(test)]\nmod tests;'):
        require(policy, anchor)
    for forbidden in ('syn::', 'quote!', 'unsafe', '.clone()', 'usize::MAX'):
        if forbidden in policy:
            raise ValueError('native621: policy mixes syntax, cloning or unbounded traversal: ' + forbidden)
    borrowed = (root / (BASE + '/resource_limits/borrowed_types.rs')).read_text()
    for anchor in ('limits: Limits::DEFAULT,', 'children(visit, &mut self.scan)?;',
                   'check_frames(&self.scan.metrics, self.scan.limits)',
                   'bounded.source(Node::Type(ty)).map_err(render_error)?;'):
        require(borrowed, anchor)
    limits = (root / (BASE + '/resource_limits/policy.rs')).read_text()
    default = limits[limits.index('pub(super) const DEFAULT: Self = Self {'):]
    for anchor in ('nodes: 32768,', 'pending: 4096,', 'syntax_depth: 48,',
                   'declarations: 256,', 'path_units: 1664,'):
        require(default, anchor)
    return {'owner_sha256': digest(source.encode()), 'policy_sha256': digest(policy.encode()),
            'native_guard_sites': 7, 'checked_operations': 8,
            'equality_tuple_limit': 12, 'crypto_tuple_limit': 11,
            'resource_policy': 'existing borrowed DEFAULT preflight, unchanged',
            'diagnostic_policy': 'existing operand/type/opening errors precede tuple trait refusal',
            'claim': 'native operation-specific tuple correction; no recording mechanism or profile change'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior = historical_verify(checkpoint(root, extension))
        if not prior['passed']:
            report['errors'] = ['native621: predecessor checks failed', *prior['errors']]
            return report
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('native621: existing route denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_text())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('native621: recording authority changed')
        closure = source_closure(root, extension)
        if closure != extension['production_closure']:
            raise ValueError('native621: reviewed native closure changed')
        report['reviewed_native_tuple_extension'] = {
            'issue': 621, 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': extension['baseline_commit'],
            'historical_manifest_count': 65, 'original_oracles': 49,
            'production_closure': closure, 'owners': copy.deepcopy(extension['owners'])}
        report['source_changes'] = [
            {'source': owner, 'baseline_sha256': expected,
             'current_sha256': digest((root / owner).read_bytes())}
            for owner, expected in baseline['source_fingerprints'].items()
            if digest((root / owner).read_bytes()) != expected]
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
