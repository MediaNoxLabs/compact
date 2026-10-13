# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive syntax-free production pure body checker; cumulative-work qualification pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = '9cb45f349f4043f8f39590880e8cf89fa8b6ca7cd83b134bb4034c31f131b460'
BASELINE_COMMIT = 'e18bd1606a0a4ea413f0567ec1ad2a7ba1e3eb28'
BASE = 'tools/compact-rust-backend/src'
SEMANTICS = BASE + '/pure_semantics.rs'
CERTIFIER = BASE + '/recorded/effect_plan/pure_certification.rs'
RETIRED = BASE + '/recorded/effect_plan/pure_certification/native.rs'
CHECKER = BASE + '/recorded/effect_plan/pure_certification/checker.rs'
BODY = BASE + '/pure_semantics/body_check.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
OWNERS = {SEMANTICS, CERTIFIER}


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'D4c'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != OWNERS
            or set(extension.get('retired_sources', {})) != {RETIRED}):
        raise ValueError('body622D4c: unreviewed extension or incomplete owner set')


def inverse_regions(source, row, owner):
    """Restore only exact approved regions; mutations outside them survive."""
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('body622D4c: current inverse region changed: ' + owner)
        source = source.replace(edit['current'], edit['previous'], 1)
    return source


def retired_source_absent(root):
    # Overlay snapshots support reads, not Path.exists. Resurrection must fail
    # even if an empty or historically authentic bridge file is supplied.
    try:
        (root / RETIRED).read_bytes()
    except FileNotFoundError:
        return
    raise ValueError('body622D4c: retired production native bridge returned')


def predecessor_sources(root, extension):
    authenticate(extension)
    current = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('body622D4c: current owner changed: ' + owner)
        current[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('body622D4c: ' + label + ' changed: ' + owner)
    retired_source_absent(root)
    row = extension['retired_sources'][RETIRED]
    if digest(row['previous_source'].encode()) != row['previous_sha256'] or row['current_state'] != 'absent':
        raise ValueError('body622D4c: retired signed bridge authority changed')
    if len(extension['historical_manifests']) != 78:
        raise ValueError('body622D4c: all 78 scoped historical manifests required')
    # All actual checker/sink/owners and deletion authenticated before projection.
    previous = {owner: inverse_regions(source, extension['owners'][owner], owner)
                for owner, source in current.items()}
    for owner, source in previous.items():
        if digest(source.encode()) != extension['owners'][owner]['previous_sha256']:
            raise ValueError('body622D4c: signed D4b predecessor differs: ' + owner)
    previous[RETIRED] = row['previous_source']
    source_closure(root, extension, previous)
    return previous


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.body-checker-622d4c.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('body622D4c: formal body checker boundary changed: ' + anchor)


def ordered(source, anchors):
    for anchor in anchors:
        require(source, anchor)
    positions = [source.index(x) for x in anchors]
    if positions != sorted(positions):
        raise ValueError('body622D4c: first-error ordering changed')


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
        raise ValueError('body622D4c: checked native match boundary changed')
    opening = found[0] + len(start)
    if parts[opening] != '{':
        raise ValueError('body622D4c: checked native match opening changed')
    depth = 0
    for index in range(opening, len(parts)):
        if parts[index] == '{':
            depth += 1
        elif parts[index] == '}':
            depth -= 1
            if depth == 0:
                return parts[opening:index + 1]
    raise ValueError('body622D4c: incomplete native match')


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    retired_source_absent(root)
    checker = text(CHECKER)
    old = previous[RETIRED]
    guards = {}
    for name in ('check_defaults', 'check_named_structs'):
        current, signed = function(checker, name), function(old, name)
        if current != signed:
            raise ValueError('body622D4c: exact moved representation guard changed: ' + name)
        guards[name] = digest(current.encode())
    marker = '/// The native emitter accepts Default syntax'
    if checker[checker.index(marker):checker.index('#[cfg(test)]')] != old[old.index(marker):] + '\n':
        raise ValueError('body622D4c: complete Default/named guard bytes changed')
    check = function(checker, 'check')
    ordered(check, ('check_name(&source.name)?;', 'for parameter in &source.parameters',
                    'check_name(&parameter.name)?;', 'check_carrier(&parameter.ty)?;',
                    '.insert(parameter.name.as_str(), (&parameter.ty, ()))',
                    'RenderError::DuplicateParameter(parameter.name.clone())',
                    'check_carrier(&source.result)?;',
                    'fold_expression(&source.body, &scope, pure, &mut BodyCheckSink::new())?',
                    'if actual != source.result'))
    for anchor in ('expected: source.result.clone()', 'actual,', 'let mut scope = HashMap::new();'):
        require(check, anchor)
    for forbidden in ('parameter_idents(', 'native_fragment_with_calls(', 'expression_with_calls(',
                      'syn::', 'quote::', 'proc_macro', 'NativeSink', 'rust_type(', 'checked_ident('):
        if forbidden in checker:
            raise ValueError('body622D4c: native output entered formal checker entry')
    body = text(BODY)
    for anchor in ('type Value = ();', 'type Binding = ();', 'type Carriers = CheckSink;',
                   'fn leaf(&mut self, _: CheckedLeaf<', 'fn ordinary(&mut self, _: OrdinaryView, _: ()) {}'):
        require(body, anchor)
    for trait in ('LeafSink', 'ConversionSink', 'ArithmeticSink', 'CryptoSink', 'AggregateSink', 'ControlSink', 'ExpressionSink'):
        require(body, 'impl ' + trait + ' for BodyCheckSink {')
    for forbidden in ('syn::', 'quote::', 'proc_macro', 'NativeSink', 'NativeLeafSink', 'NativeChildren',
                      'pure_native', 'RenderError', 'check_name(', 'check_tuple_traits(', 'rust_type(',
                      'allocate(', 'checked_ident(', 'parameter_idents(', 'fold_expression(',
                      'expression_with_calls(', 'native_fragment_with_calls(', 'match ', 'if '):
        if forbidden in body:
            raise ValueError('body622D4c: unit output callback acquired checking/native syntax')
    certifier = text(CERTIFIER)
    require(certifier, 'mod checker;')
    for forbidden in ('mod native;', 'native::check', 'native_fragment_with_calls(', 'NativeSink'):
        if forbidden in certifier:
            raise ValueError('body622D4c: retired native certification fallback returned')
    if inverse_regions(certifier, extension['owners'][CERTIFIER], CERTIFIER) != previous[CERTIFIER]:
        raise ValueError('body622D4c: Certifier executable/cache/binding authority changed')
    if inverse_regions(text(SEMANTICS), extension['owners'][SEMANTICS], SEMANTICS) != previous[SEMANTICS]:
        raise ValueError('body622D4c: existing shared expression registry changed')
    reference = text(BASE + '/recorded/effect_plan/pure_certification/checker/tests/old_bridge.rs')
    if function(reference, 'check') != function(old, 'check'):
        raise ValueError('body622D4c: independent signed old bridge check body changed')
    require(checker, '#[cfg(test)]\nmod tests;')
    require(text(BASE + '/recorded/effect_plan/pure_certification/checker/tests.rs'), 'mod old_bridge;')
    functions = {}
    for owner, names in extension['neutral_function_authorities'].items():
        for name, expected in names.items():
            actual = function(text(owner), name)
            if digest(actual.encode()) != expected:
                raise ValueError('body622D4c: allowed specific neutral function changed: ' + name)
            for forbidden in ('syn::', 'quote::', 'proc_macro', 'checked_ident(', 'parameter_idents(', 'rust_type('):
                if forbidden in actual:
                    raise ValueError('body622D4c: allowed neutral function acquired native output: ' + name)
            functions[owner + '::' + name] = expected
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('body622D4c: protected shared/native/transitive source changed: ' + owner)
    return {'moved_representation_guards': guards, 'allowed_specific_neutral_functions': functions,
            'formal_entry': 'name; each parameter name/carrier/raw insertion/duplicate; result carrier; same complete fold; mismatch',
            'monomorphized_consumer': 'BodyCheckSink Value=()/Binding=()/Carriers=CheckSink; callbacks discard checked facts without traversal/checking/output',
            'production_bridge': 'retired native.rs absent and unregistered; same shared expression/visitor/family/control owners now produce no Syntax for certificate bodies',
            'callpath_qualification': 'function/receiver-associated unit substitution closure; naming semantic_identifier and tuple capability policy allowed; unrelated module native constructors unreachable',
            'resource_policy': 'existing allocation/semantic plans/maps may remain; cumulative body work/memory and EA13 still pending',
            'goal_scope': 'production pure body syntax bridge removed; not formal proof/full-spec/TS parity/proof availability or goal508 closure',
            'new_retired_routes': [], 'claim': 'syntax-free production body source authority, not language/formal coverage'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['body622D4c: predecessor checks failed', *prior['errors']]
            return report
        expected = {'reviewed_native_control_fold_extension': 'D4b',
                    'reviewed_shared_aggregate_extension': 'D3',
                    'reviewed_shared_crypto_extension': 'D2',
                    'reviewed_shared_arithmetic_extension': 'D1c',
                    'reviewed_shared_conversion_extension': 'D1b',
                    'reviewed_shared_leaf_extension': 'D1a', 'reviewed_unit_fragment_extension': 'C1+C2'}
        if (any(prior.get(key, {}).get('issue') != 622 or prior.get(key, {}).get('slice') != stage
                for key, stage in expected.items())
                or prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627
                or prior.get('reviewed_lexical_symbols_extension', {}).get('issue') != 629
                or prior.get('reviewed_lexical_symbols_extension', {}).get('slice') != 'D4a'):
            raise ValueError('body622D4c: mandatory actual D4b/D4a/D3/D2/D1c/D1b/D1a/C/B predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('body622D4c: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('body622D4c: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('body622D4c: reviewed body checker closure changed')
        report['reviewed_body_checker_extension'] = {
            'issue': 622, 'slice': 'D4c', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': BASELINE_COMMIT, 'historical_manifest_count': 78,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
