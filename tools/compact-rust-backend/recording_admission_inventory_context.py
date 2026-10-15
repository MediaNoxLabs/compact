# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Specific #526 context-query retirement; shared VotingCommit support stays live."""
import re

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_unit import RETIRED as PRIOR_RETIRED

ROUTE = 'selector.context-query'
REPLACEMENT = 'algebra.context-query'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/typed_plan/context_query_reference.rs'
IR = 'tools/compact-rust-backend/src/ir.rs'
MARKER = "pub(super) fn lower_context_query<'a>("
ORACLE_MARKER = "pub(super) fn lower<'a>("
PREDICATES = ['fn context_query_type(', 'fn context_query_value(']
SHARED_SYMBOLS = ['context_query_type', 'context_query_value',
                  'EvaluationState.context_query', 'ir::contract_address_type']
SHARED_ANCHORS = {
    TYPED: ['let old = self.evaluation.context_query;',
            'self.evaluation.context_query = context;',
            'self.evaluation.context_query = old;'],
    'tools/compact-rust-backend/src/recorded/typed_plan/voting_commit.rs':
        ['context_query_value('],
    'tools/compact-rust-backend/src/recorded/typed_plan/evaluation.rs':
        ['pub(super) context_query: bool,'],
}
RETIRED = [*PRIOR_RETIRED, ROUTE]


def function(source, marker):
    """Exact top-level function text; reviewed source format, not a Rust parser."""
    if source.count(marker) != 1:
        raise ValueError('function anchor is not unique: ' + marker)
    start = source.index(marker)
    end = source.find('\n}\n', start)
    if end < 0:
        raise ValueError('function closing anchor missing: ' + marker)
    return source[start:end + 3]


def validate(baseline, transition, baseline_bytes, unit_transition):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != 526
            or transition.get('retired_route') != ROUTE
            or transition.get('replacement_route') != REPLACEMENT):
        errors.append('context retirement: unsupported transition')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('context retirement: frozen baseline bytes changed')
    if (transition.get('original_source') != TYPED
            or transition.get('original_source_sha256') != baseline['source_fingerprints'].get(TYPED)):
        errors.append('context retirement: original source is not bound to baseline')
    if (transition.get('oracle_source') != ORACLE
            or transition.get('original_body_start') != MARKER
            or transition.get('oracle_body_start') != ORACLE_MARKER):
        errors.append('context retirement: unreviewed oracle location or normalization')
    for key in ('original_body_sha256', 'oracle_normalized_body_sha256',
                'canonical_descriptor_body_sha256'):
        if not re.fullmatch('[0-9a-f]{64}', transition.get(key, '')):
            errors.append(f'context retirement: invalid body fingerprint: {key}')
    if transition.get('original_body_sha256') != transition.get('oracle_normalized_body_sha256'):
        errors.append('context retirement: oracle must preserve original admission body after name-only normalization')
    predicates = transition.get('retained_predicates_sha256', {})
    if set(predicates) != set(PREDICATES) or any(not re.fullmatch('[0-9a-f]{64}', sha) for sha in predicates.values()):
        errors.append('context retirement: invalid retained predicate fingerprints')
    if unit_transition is None or transition.get('prior_unit_transition_canonical_sha256') != canonical(unit_transition):
        errors.append('context retirement: frozen unit transition changed or unavailable')
        return errors
    before = unit_transition['reviewed_selector']
    if transition.get('prior_selector') != before:
        errors.append('context retirement: prior selector is not bound to frozen evidence')
    old = 'typed_plan::lower_context_query'
    after = transition.get('reviewed_selector', {})
    if before['ordered_calls'].count(old) != 1 or after.get('ordered_calls') != [call for call in before['ordered_calls'] if call != old]:
        errors.append('context retirement: unrelated selector calls/order changed')
    if not re.fullmatch('[0-9a-f]{64}', after.get('function_sha256', '')):
        errors.append('context retirement: invalid reviewed selector fingerprint')
    if transition.get('prior_source_fingerprints') != {SELECTOR: unit_transition['reviewed_source_fingerprints'][SELECTOR]}:
        errors.append('context retirement: prior observation owner is not bound to frozen evidence')
    after_sources = transition.get('reviewed_source_fingerprints', {})
    if set(after_sources) != {SELECTOR} or not re.fullmatch('[0-9a-f]{64}', after_sources.get(SELECTOR, '')):
        errors.append('context retirement: only reviewed selector observation owner may change')
    ids = sorted(REPLACEMENT if route == ROUTE else route for route in unit_transition['instrumented_route_ids'])
    if transition.get('instrumented_route_ids') != ids:
        errors.append('context retirement: unrelated observation IDs changed')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('context retirement: unreviewed retirement set')
    if transition.get('retained_shared_production_symbols') != SHARED_SYMBOLS:
        errors.append('context retirement: shared production ownership claim changed')
    matches = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': old + '(', 'baseline_occurrences': 1}]
    if len(matches) != 1 or matches[0]['anchors'] != anchor:
        errors.append('context retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    if re.search(r'\blower_context_query\s*\(', sources.get(SELECTOR, '')):
        errors.append('context retirement: selector reintroduced old admission')
    typed = sources.get(TYPED, '')
    for declaration in (r'mod\s+context_query_reference\s*;',
                        r'pub\(super\)\s+fn\s+lower_context_query\b'):
        if (len(re.findall(declaration, typed)) != 1
                or len(re.findall(r'#\[cfg\(test\)\]\s*' + declaration, typed)) != 1):
            errors.append('context retirement: oracle module and wrapper must each be uniquely cfg(test)')
    if len(re.findall(r'\bfn\s+lower_context_query\b', typed)) != 1:
        errors.append('context retirement: oracle module and wrapper must each be uniquely cfg(test)')
    for source, anchors in SHARED_ANCHORS.items():
        try:
            shared = sources[source] if source in sources else (root / source).read_text()
        except (OSError, UnicodeError):
            shared = ''
        for anchor in anchors:
            if shared.count(anchor) != 1:
                errors.append('context retirement: retained shared policy or caller changed: ' + anchor)
    for marker in PREDICATES:
        try:
            if digest(function(typed, marker).encode()) != transition['retained_predicates_sha256'][marker]:
                errors.append('context retirement: shared predicate differs from original: ' + marker)
        except ValueError as error:
            errors.append('context retirement: ' + str(error))
    try:
        oracle = (root / ORACLE).read_text()
        body = function(oracle, ORACLE_MARKER).replace(ORACLE_MARKER, MARKER, 1)
        if digest(body.encode()) != transition['oracle_normalized_body_sha256']:
            errors.append('context retirement: oracle no longer equals original admission body')
        ir = (root / IR).read_text()
        descriptor = function(ir, 'pub(crate) fn contract_address_type(')
        descriptor = descriptor.replace('pub(crate) fn', 'fn', 1)
        if digest(descriptor.encode()) != transition['canonical_descriptor_body_sha256']:
            errors.append('context retirement: shared canonical descriptor differs from original')
        if ('use crate::ir::{KernelClaimKind, ReturnPlan, contract_address_type};' not in typed
                or 'fn contract_address_type(' in typed):
            errors.append('context retirement: legacy evaluator must use the shared canonical descriptor')
    except (OSError, UnicodeError, ValueError) as error:
        errors.append('context retirement: oracle or shared descriptor unreadable: ' + str(error))
    return errors
