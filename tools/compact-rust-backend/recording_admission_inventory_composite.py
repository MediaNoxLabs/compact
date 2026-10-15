# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Whole composite admission retirement; shared value/effect evaluation stays live."""
import re

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_context import function
from recording_admission_inventory_kernel import RETIRED as PRIOR_RETIRED

ISSUE = 529
ROUTE = 'selector.composite-values'
REPLACEMENT = 'algebra.composite-values'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/typed_plan/composite_reference.rs'
FIELD = 'tools/compact-rust-backend/src/recorded/typed_plan/field_observations.rs'
MARKER = "pub(super) fn lower_composite<'a>("
ORACLE_MARKER = "pub(super) fn lower<'a>("
# Reviewed directly against original baseline2704c624 and checkpoint910497c8.
# The field audit was not a baseline source-owner row; pin its whole original file.
ORIGINAL_FUNCTIONS = {
    'fn composite_type(': 'b91bd787b848cd1d99f3fe27c62b5336541ad0ffdeb4b37baecba9bef2bc7a55',
    'fn composite_value(': '4304278362ae87c5b3bb95c799eb5f22dda0c2a1cc20acf6c841dbef7baf26bf',
    'fn composite_circuit(': 'c94fcf7bcdd956f8e2770994a12ca7958c6882cc330bc3bfbc561f30eae81611',
    MARKER: '70061d85c4133514ad764cf4f5e19d35e3876fd9671c8a9e9428e92264822833',
}
FIELD_SHA = 'c001d565438add4b63833c079bdf6f03c1ba227c3f5122410348457be563fd00'
SHARED_SYMBOLS = ['CompositeDomain', 'Plan::expression', 'Plan::inline_call',
                  'intent_effect::emit', 'coin_shapes']
RETIRED = [*PRIOR_RETIRED, ROUTE]


def validate(baseline, transition, baseline_bytes, kernel_transition):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != ISSUE
            or transition.get('retired_route') != ROUTE
            or transition.get('replacement_route') != REPLACEMENT):
        errors.append('composite retirement: unsupported transition')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('composite retirement: frozen baseline bytes changed')
    if (transition.get('original_source') != TYPED
            or transition.get('original_source_sha256') != baseline['source_fingerprints'].get(TYPED)):
        errors.append('composite retirement: original source is not bound to baseline')
    if (transition.get('oracle_source') != ORACLE
            or transition.get('original_body_start') != MARKER
            or transition.get('oracle_body_start') != ORACLE_MARKER):
        errors.append('composite retirement: unreviewed oracle location or normalization')
    if (transition.get('original_functions_sha256') != ORIGINAL_FUNCTIONS
            or transition.get('oracle_normalized_functions_sha256') != ORIGINAL_FUNCTIONS):
        errors.append('composite retirement: all four original admission functions must be preserved')
    if (transition.get('field_observation_source') != FIELD
            or transition.get('field_observation_source_sha256') != FIELD_SHA):
        errors.append('composite retirement: whole field-observation oracle must preserve original source')
    if kernel_transition is None or transition.get('prior_kernel_transition_canonical_sha256') != canonical(kernel_transition):
        errors.append('composite retirement: frozen kernel transition changed or unavailable')
        return errors
    before = kernel_transition['reviewed_selector']
    if transition.get('prior_selector') != before:
        errors.append('composite retirement: prior selector is not bound to frozen evidence')
    old = 'typed_plan::lower_composite'
    after = transition.get('reviewed_selector', {})
    if before['ordered_calls'].count(old) != 1 or after.get('ordered_calls') != [call for call in before['ordered_calls'] if call != old]:
        errors.append('composite retirement: unrelated selector calls/order changed')
    if not re.fullmatch('[0-9a-f]{64}', after.get('function_sha256', '')):
        errors.append('composite retirement: invalid reviewed selector fingerprint')
    if transition.get('prior_source_fingerprints') != {SELECTOR: kernel_transition['reviewed_source_fingerprints'][SELECTOR]}:
        errors.append('composite retirement: prior observation owner is not bound to frozen evidence')
    fingerprints = transition.get('reviewed_source_fingerprints', {})
    if set(fingerprints) != {SELECTOR} or not re.fullmatch('[0-9a-f]{64}', fingerprints.get(SELECTOR, '')):
        errors.append('composite retirement: only reviewed selector observation owner may change')
    ids = sorted(REPLACEMENT if route == ROUTE else route for route in kernel_transition['instrumented_route_ids'])
    if transition.get('instrumented_route_ids') != ids:
        errors.append('composite retirement: unrelated observation IDs changed')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('composite retirement: unreviewed retirement set')
    if transition.get('retained_shared_production_symbols') != SHARED_SYMBOLS:
        errors.append('composite retirement: shared evaluation must remain separately owned')
    matches = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': old + '(', 'baseline_occurrences': 1}]
    if len(matches) != 1 or matches[0]['anchors'] != anchor:
        errors.append('composite retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    if re.search(r'\b(?:lower_composite|composite_reference|field_observations)\b', sources.get(SELECTOR, '')):
        errors.append('composite retirement: selector reintroduced old admission import or call')
    typed = sources.get(TYPED, '')
    for declaration in (r'mod\s+composite_reference\s*;',
                        r'mod\s+field_observations\s*;',
                        r'pub\(super\)\s+fn\s+lower_composite\b'):
        if (len(re.findall(declaration, typed)) != 1
                or len(re.findall(r'#\[cfg\(test\)\]\s*' + declaration, typed)) != 1):
            errors.append('composite retirement: both oracle modules and wrapper must be uniquely cfg(test)')
    if len(re.findall(r'\bfn\s+lower_composite\b', typed)) != 1:
        errors.append('composite retirement: both oracle modules and wrapper must be uniquely cfg(test)')
    if re.search(r'\bfn\s+composite_(?:type|value|circuit)\b', typed):
        errors.append('composite retirement: admission helpers remain outside the test-only oracle')
    for anchor in ('enum CompositeDomain', 'fn values(self)', 'fn intents(self)',
                   'fn inline_call(', 'fn expression('):
        if typed.count(anchor) != 1:
            errors.append('composite retirement: retained shared evaluator changed: ' + anchor)
    try:
        oracle = (root / ORACLE).read_text()
        for marker, expected in ORIGINAL_FUNCTIONS.items():
            actual_marker = ORACLE_MARKER if marker == MARKER else marker
            body = function(oracle, actual_marker).replace(actual_marker, marker, 1)
            if digest(body.encode()) != expected:
                errors.append('composite retirement: original admission function changed: ' + marker)
        if digest((root / FIELD).read_bytes()) != FIELD_SHA:
            errors.append('composite retirement: whole field-observation oracle changed')
    except (OSError, UnicodeError, ValueError) as error:
        errors.append('composite retirement: original oracle unreadable: ' + str(error))
    return errors
