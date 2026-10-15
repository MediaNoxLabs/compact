# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Whole receive admission retirement; shared shielded helper dispatch stays live."""
import re

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_context import function
from recording_admission_inventory_composite import RETIRED as PRIOR_RETIRED

ISSUE = 536
CHECKPOINT = '3073dcd20efc525f3f03b1a3517cac1bc6969d75'
ROUTE = 'selector.shielded-receive'
REPLACEMENT = 'algebra.shielded-receive'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/typed_plan/shielded_receive_reference.rs'
MARKER = "pub(super) fn lower_shielded_receive<'a>("
ORACLE_MARKER = "pub(super) fn lower<'a>("
# Exact bodies match original baseline2704c624 and checkpoint3073dcd2.
SHARED_FUNCTIONS = {
    'fn shielded_receive_value(': '7f2e18fd3217e161942f22a625627b850df00a992c74c61ca665018c7049c3a1',
    'fn shielded_receive_action(': '1464313e2974deb0fcad51adb81da5036a947fe09ea5bfae6b2160f80b39a5ec',
}
ORIGINAL_FUNCTIONS = {MARKER: '5e14a1f4d6c9c2481f9f13b268b8cb43709160f1ea11562f8b1fe0ca7a476f2e'}
SHARED_MARKER = 'fn shielded_unit_signature('
SHARED_SHA = '66f59340f702aa2d10ebfdb9a6f211e13b57bebdcea0ede11b6c79501b1c18f8'
SHARED_USERS = {
    'singleton_bridge': r'composite_domain\.singleton_bridge\(\)\s*&&\s*shielded_unit_signature\(callee\)',
    'unit_actions_intents': r'composite_domain\.intents\(\)\s*&&\s*shielded_unit_signature\(callee\)',
}

SHARED_CALLERS = {
    'start_funding': '48bae139027a353b2ae462554b7735b839a9a85ba6ce4e4ad376483ceac950b5',
    'voting_commit': '7a16bf5d0552c3ba82e680a32978280b68106f8373f2ea0eca63df4ddc8e1a40',
    'shielded_merge': '65a451b226b35a308575c10393a07313f73d4f28a18e3da0172807f96e5b4ffc',
    'guarded_deposit': 'd7c41e82b00576d5afd2f0857c92cd453e205a29571383ac80fe34a1f0f78c07',
    'immediate_send': '91ff83866f77e2ca57f26e7c7644134887e674e1009ba3949b8f1a84c429fce6',
}
SHARED_CALLERS = {f'tools/compact-rust-backend/src/recorded/typed_plan/{name}.rs': sha
                  for name, sha in SHARED_CALLERS.items()}
RELOCATED_ANCHORS = [{'source': TYPED, 'text': 'unit_actions: true,', 'target': ORACLE,
                      'source_occurrences': 0, 'target_occurrences': 1}]

RETIRED = [*PRIOR_RETIRED, ROUTE]


def validate(baseline, transition, baseline_bytes, composite_transition):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != ISSUE
            or transition.get('retired_route') != ROUTE
            or transition.get('replacement_route') != REPLACEMENT
            or transition.get('reviewed_checkpoint') != CHECKPOINT):
        errors.append('receive retirement: unsupported transition or checkpoint')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('receive retirement: frozen baseline bytes changed')
    if (transition.get('original_source') != TYPED
            or transition.get('original_source_sha256') != baseline['source_fingerprints'].get(TYPED)):
        errors.append('receive retirement: original source is not bound to baseline')
    if (transition.get('oracle_source') != ORACLE
            or transition.get('original_body_start') != MARKER
            or transition.get('oracle_body_start') != ORACLE_MARKER):
        errors.append('receive retirement: unreviewed oracle location or normalization')
    if (transition.get('original_functions_sha256') != ORIGINAL_FUNCTIONS
            or transition.get('oracle_normalized_functions_sha256') != ORIGINAL_FUNCTIONS):
        errors.append('receive retirement: original root admission function must be preserved')
    if (transition.get('retained_shared_functions_sha256') != SHARED_FUNCTIONS
            or transition.get('retained_shared_callers_sha256') != SHARED_CALLERS
            or transition.get('relocated_shared_anchors') != RELOCATED_ANCHORS):
        errors.append('receive retirement: shared predicates/callers or exact policy relocation changed')
    if (transition.get('retained_shared_predicate_sha256') != SHARED_SHA
            or transition.get('retained_shared_predicate_users') != sorted(SHARED_USERS)):
        errors.append('receive retirement: shared predicate and live users must remain bound')
    if composite_transition is None or transition.get('prior_composite_transition_canonical_sha256') != canonical(composite_transition):
        errors.append('receive retirement: frozen composite transition changed or unavailable')
        return errors
    before = composite_transition['reviewed_selector']
    if transition.get('prior_selector') != before:
        errors.append('receive retirement: prior selector is not bound to frozen evidence')
    old = 'typed_plan::lower_shielded_receive'
    after = transition.get('reviewed_selector', {})
    if before['ordered_calls'].count(old) != 1 or after.get('ordered_calls') != [call for call in before['ordered_calls'] if call != old]:
        errors.append('receive retirement: unrelated selector calls/order changed')
    if not re.fullmatch('[0-9a-f]{64}', after.get('function_sha256', '')):
        errors.append('receive retirement: invalid reviewed selector fingerprint')
    if transition.get('prior_source_fingerprints') != {SELECTOR: composite_transition['reviewed_source_fingerprints'][SELECTOR]}:
        errors.append('receive retirement: prior observation owner is not bound to frozen evidence')
    fingerprints = transition.get('reviewed_source_fingerprints', {})
    if set(fingerprints) != {SELECTOR} or not re.fullmatch('[0-9a-f]{64}', fingerprints.get(SELECTOR, '')):
        errors.append('receive retirement: only reviewed selector observation owner may change')
    ids = sorted(REPLACEMENT if route == ROUTE else route for route in composite_transition['instrumented_route_ids'])
    if transition.get('instrumented_route_ids') != ids:
        errors.append('receive retirement: unrelated observation IDs changed')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('receive retirement: unreviewed retirement set')
    matches = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': old + '(', 'baseline_occurrences': 1}]
    if len(matches) != 1 or matches[0]['anchors'] != anchor:
        errors.append('receive retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    # All baseline production owners are checked, not only the main selector.
    for owner, text in sources.items():
        if owner != TYPED and (re.search(r'\b(?:lower_shielded_receive|shielded_receive_reference)\b', text)
                or (owner not in SHARED_CALLERS and re.search(r'\b(?:shielded_receive_value|shielded_receive_action)\b', text))):
            errors.append('receive retirement: production owner reintroduced old admission import or call: ' + owner)
    typed = sources.get(TYPED, '')
    for declaration in (r'mod\s+shielded_receive_reference\s*;',
                        r'pub\(super\)\s+fn\s+lower_shielded_receive\b'):
        if (len(re.findall(declaration, typed)) != 1
                or len(re.findall(r'#\[cfg\(test\)\]\s*' + declaration, typed)) != 1):
            errors.append('receive retirement: oracle module and wrapper must be uniquely cfg(test)')
    if len(re.findall(r'\bfn\s+lower_shielded_receive\b', typed)) != 1:
        errors.append('receive retirement: oracle module and wrapper must be uniquely cfg(test)')
    if len(re.findall(r'\bshielded_receive_reference\s*::\s*lower\s*\(', typed)) != 1:
        errors.append('receive retirement: only test wrapper may call reference lower')
    try:
        wrapper = function(typed, MARKER)
        outside = typed.replace(wrapper, '', 1)
        outside = re.sub(r'mod\s+shielded_receive_reference\s*;', '', outside)
        if (len(re.findall(r'\bshielded_receive_reference\s*::\s*lower\s*\(', wrapper)) != 1
                or re.search(r'\b(?:lower_shielded_receive|shielded_receive_reference)\b', outside)):
            errors.append('receive retirement: reference authority escaped the test-only wrapper')
        if digest(function(typed, SHARED_MARKER).encode()) != SHARED_SHA:
            errors.append('receive retirement: retained shared predicate changed')
        for user, pattern in SHARED_USERS.items():
            if len(re.findall(pattern, typed)) != 1:
                errors.append('receive retirement: retained shared predicate user changed: ' + user)
        for marker, expected in SHARED_FUNCTIONS.items():
            if typed.count(marker) != 1 or digest(function(typed, marker).encode()) != expected:
                errors.append('receive retirement: retained shared admission helper changed: ' + marker)
        for owner, expected in SHARED_CALLERS.items():
            if digest(sources.get(owner, '').encode()) != expected:
                errors.append('receive retirement: retained shared caller changed: ' + owner)
        oracle = (root / ORACLE).read_text()
        for marker, expected in ORIGINAL_FUNCTIONS.items():
            actual_marker = ORACLE_MARKER if marker == MARKER else marker
            body = function(oracle, actual_marker).replace(actual_marker, marker, 1)
            if digest(body.encode()) != expected:
                errors.append('receive retirement: original admission function changed: ' + marker)
    except (OSError, UnicodeError, ValueError) as error:
        errors.append('receive retirement: original oracle/shared predicate unreadable: ' + str(error))
    return errors
