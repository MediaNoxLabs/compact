# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Specific #517 retirement obligations; never a generic source-gate bypass."""
import hashlib
import json
import re

ROUTE = 'selector.jubjub-values'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
ORIGINAL = 'tools/compact-rust-backend/src/recorded/typed_plan/jubjub_values.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/typed_plan/jubjub_reference.rs'
MARKER = 'fn value_type('
REPLACEMENT = 'algebra.jubjub-values'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return digest(json.dumps(value, sort_keys=True, separators=(',', ':')).encode())


def validate(baseline, transition, baseline_bytes, instrumentation):
    errors = []
    if transition.get('schema_version') != 1 or transition.get('issue') != 517 or transition.get('retired_route') != ROUTE:
        errors.append('Jubjub retirement: unsupported transition')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('Jubjub retirement: frozen baseline bytes changed')
    if transition.get('original_source') != ORIGINAL or transition.get('original_source_sha256') != baseline['source_fingerprints'].get(ORIGINAL):
        errors.append('Jubjub retirement: original source is not bound to baseline')
    if transition.get('oracle_source') != ORACLE or transition.get('oracle_body_start') != MARKER:
        errors.append('Jubjub retirement: unreviewed oracle location')
    if not re.fullmatch('[0-9a-f]{64}', transition.get('oracle_body_sha256', '')):
        errors.append('Jubjub retirement: invalid oracle body fingerprint')
    if instrumentation is None or transition.get('prior_instrumentation_canonical_sha256') != canonical(instrumentation):
        errors.append('Jubjub retirement: original instrumentation manifest changed or unavailable')
        return errors
    calls = baseline['main_selector']['ordered_calls']
    retired_call = 'typed_plan::lower_jubjub_values'
    expected = [call for call in calls if call != retired_call]
    reviewed = transition.get('reviewed_selector', {})
    if calls.count(retired_call) != 1 or reviewed.get('ordered_calls') != expected:
        errors.append('Jubjub retirement: unrelated selector calls/order changed')
    if not re.fullmatch('[0-9a-f]{64}', reviewed.get('function_sha256', '')):
        errors.append('Jubjub retirement: invalid reviewed selector fingerprint')
    before = transition.get('prior_source_fingerprints')
    if before != {SELECTOR: instrumentation['reviewed_source_fingerprints'][SELECTOR]}:
        errors.append('Jubjub retirement: original selector owner not bound to instrumentation')
    after = transition.get('reviewed_source_fingerprints', {})
    if set(after) != {SELECTOR} or not re.fullmatch('[0-9a-f]{64}', after.get(SELECTOR, '')):
        errors.append('Jubjub retirement: only the reviewed selector observation owner may change')
    ids = sorted(REPLACEMENT if route == ROUTE else route for route in instrumentation['instrumented_route_ids'])
    if transition.get('instrumented_route_ids') != ids:
        errors.append('Jubjub retirement: unrelated observation IDs changed')
    expected_anchor = [{'source': SELECTOR, 'text': retired_call + '(', 'baseline_occurrences': 1}]
    matches = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(matches) != 1 or matches[0]['anchors'] != expected_anchor:
        errors.append('Jubjub retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    selector = sources.get(SELECTOR, '')
    typed = sources.get(TYPED, '')
    original = sources.get(ORIGINAL, '')
    if re.search(r'\blower_jubjub_values\s*\(', selector):
        errors.append('Jubjub retirement: selector reintroduced old admission')
    declarations = (
        r'mod\s+jubjub_reference\s*;',
        r'pub\(super\)\s+fn\s+lower_jubjub_values\b',
    )
    for declaration in declarations:
        if len(re.findall(declaration, typed)) != 1 or len(re.findall(r'#\[cfg\(test\)\]\s*' + declaration, typed)) != 1:
            errors.append('Jubjub retirement: oracle module and wrapper must each be uniquely cfg(test)')
    for declaration in (r'\bfn\s+(?:lower|value_type|value|bindings)\s*\(', r'\bstruct\s+Audit\b', r'\bimpl\s+Audit\b'):
        if re.search(declaration, original):
            errors.append('Jubjub retirement: shape admission remains in production value adapter')
    try:
        oracle = (root / ORACLE).read_text()
        if oracle.count(MARKER) != 1:
            errors.append('Jubjub retirement: oracle body anchor changed')
        elif digest((oracle[oracle.index(MARKER):].strip() + '\n').encode()) != transition['oracle_body_sha256']:
            errors.append('Jubjub retirement: oracle no longer equals original admission body')
    except (OSError, UnicodeError):
        errors.append('Jubjub retirement: oracle source unreadable')
    return errors
