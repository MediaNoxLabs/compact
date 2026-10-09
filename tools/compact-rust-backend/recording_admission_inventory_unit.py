# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Specific #525 unit-actions retirement; shared legacy predicates stay active."""
import re

from recording_admission_inventory_jubjub import canonical, digest

ROUTE = 'selector.unit-actions'
REPLACEMENT = 'algebra.unit-actions'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
POLICY = 'tools/compact-rust-backend/src/recorded/typed_plan/admission.rs'
ORIGINAL = 'tools/compact-rust-backend/src/recorded/typed_plan/unit_actions.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/typed_plan/unit_actions_reference.rs'
MARKER = "struct Audit<'a> {"
PREDICATES = 'pub(super) fn value_type('
SHARED_SYMBOLS = ['unit_actions::value_type', 'unit_actions::field_structure',
                  'unit_actions::helper_signature', 'AdmissionPolicy.unit_actions']
SHARED_ANCHORS = {
    POLICY: ['pub(super) unit_actions: bool,'],
    TYPED: ['unit_actions: true,', 'unit_actions: domain.singleton_bridge(),',
            'unit_actions::helper_signature(callee)'],
}
RETIRED = ['effectful.effectful', 'selector.jubjub-values', 'selector.terminal-returns', ROUTE]


def validate(baseline, transition, baseline_bytes, terminal_transition):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != 525
            or transition.get('retired_route') != ROUTE):
        errors.append('unit retirement: unsupported transition')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('unit retirement: frozen baseline bytes changed')
    if (transition.get('original_source') != ORIGINAL
            or transition.get('original_source_sha256') != baseline['source_fingerprints'].get(ORIGINAL)):
        errors.append('unit retirement: original source is not bound to baseline')
    if transition.get('oracle_source') != ORACLE or transition.get('oracle_body_start') != MARKER:
        errors.append('unit retirement: unreviewed oracle location')
    for key in ('original_body_sha256', 'oracle_body_sha256', 'retained_predicates_sha256'):
        if not re.fullmatch('[0-9a-f]{64}', transition.get(key, '')):
            errors.append(f'unit retirement: invalid body fingerprint: {key}')
    if transition.get('original_body_sha256') != transition.get('oracle_body_sha256'):
        errors.append('unit retirement: oracle must preserve original admission body exactly')
    if terminal_transition is None or transition.get('prior_terminal_transition_canonical_sha256') != canonical(terminal_transition):
        errors.append('unit retirement: frozen terminal transition changed or unavailable')
        return errors
    before = terminal_transition['reviewed_selector']
    if transition.get('prior_selector') != before:
        errors.append('unit retirement: prior selector is not bound to frozen evidence')
    old = 'typed_plan::lower_unit_actions'
    after = transition.get('reviewed_selector', {})
    if before['ordered_calls'].count(old) != 1 or after.get('ordered_calls') != [call for call in before['ordered_calls'] if call != old]:
        errors.append('unit retirement: unrelated selector calls/order changed')
    if not re.fullmatch('[0-9a-f]{64}', after.get('function_sha256', '')):
        errors.append('unit retirement: invalid reviewed selector fingerprint')
    if transition.get('prior_source_fingerprints') != {SELECTOR: terminal_transition['reviewed_source_fingerprints'][SELECTOR]}:
        errors.append('unit retirement: prior observation owner is not bound to frozen evidence')
    after_sources = transition.get('reviewed_source_fingerprints', {})
    if set(after_sources) != {SELECTOR} or not re.fullmatch('[0-9a-f]{64}', after_sources.get(SELECTOR, '')):
        errors.append('unit retirement: only reviewed selector observation owner may change')
    ids = sorted(REPLACEMENT if route == ROUTE else route for route in terminal_transition['instrumented_route_ids'])
    if transition.get('instrumented_route_ids') != ids:
        errors.append('unit retirement: unrelated observation IDs changed')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('unit retirement: unreviewed retirement set')
    if transition.get('retained_shared_production_symbols') != SHARED_SYMBOLS:
        errors.append('unit retirement: shared production ownership claim changed')
    matches = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': old + '(', 'baseline_occurrences': 1}]
    if len(matches) != 1 or matches[0]['anchors'] != anchor:
        errors.append('unit retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    if re.search(r'\blower_unit_actions\s*\(', sources.get(SELECTOR, '')):
        errors.append('unit retirement: selector reintroduced old admission')
    typed = sources.get(TYPED, '')
    for declaration in (r'mod\s+unit_actions_reference\s*;',
                        r'pub\(super\)\s+fn\s+lower_unit_actions\b'):
        if (len(re.findall(declaration, typed)) != 1
                or len(re.findall(r'#\[cfg\(test\)\]\s*' + declaration, typed)) != 1):
            errors.append('unit retirement: oracle module and wrapper must each be uniquely cfg(test)')
    for source, anchors in SHARED_ANCHORS.items():
        for anchor in anchors:
            if sources.get(source, '').count(anchor) != 1:
                errors.append('unit retirement: retained shared policy or caller changed: ' + anchor)
    shared = sources.get(ORIGINAL, '')
    if re.search(r'\b(?:struct\s+Audit|impl\s+Audit|fn\s+lower\s*\()', shared):
        errors.append('unit retirement: shape admission remains in production predicate owner')
    if (shared.count(PREDICATES) != 1
            or digest((shared[shared.index(PREDICATES):].strip() + '\n').encode()) != transition['retained_predicates_sha256']):
        errors.append('unit retirement: shared predicates differ from original')
    try:
        oracle = (root / ORACLE).read_text()
        if oracle.count(MARKER) != 1:
            errors.append('unit retirement: oracle body anchor changed')
        elif digest((oracle[oracle.index(MARKER):].strip() + '\n').encode()) != transition['oracle_body_sha256']:
            errors.append('unit retirement: oracle no longer equals original admission body')
    except (OSError, UnicodeError):
        errors.append('unit retirement: oracle source unreadable')
    return errors
