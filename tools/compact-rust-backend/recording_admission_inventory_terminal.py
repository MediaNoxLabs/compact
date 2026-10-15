# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Specific #521 terminal retirement, layered over immutable #519 evidence."""
import re

from recording_admission_inventory_jubjub import canonical, digest

ROUTE = 'selector.terminal-returns'
REPLACEMENT = 'algebra.terminal-returns'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TRACE = 'tools/compact-rust-backend/src/recorded/admission_probe/trace.rs'
SOURCES = {SELECTOR, TRACE}
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
ORIGINAL = 'tools/compact-rust-backend/src/recorded/typed_plan/terminal_returns.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/typed_plan/terminal_returns_reference.rs'
MARKER = 'fn scalar(ty: &Type) -> bool {'
ADAPTER = 'pub(super) fn adapt('
RETIRED = ['effectful.effectful', 'selector.jubjub-values', ROUTE]


def validate(baseline, transition, baseline_bytes, call_extension, instrumentation):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != 521
            or transition.get('retired_route') != ROUTE):
        errors.append('terminal retirement: unsupported transition')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('terminal retirement: frozen baseline bytes changed')
    if (transition.get('original_source') != ORIGINAL
            or transition.get('original_source_sha256') != baseline['source_fingerprints'].get(ORIGINAL)):
        errors.append('terminal retirement: original source is not bound to baseline')
    if transition.get('oracle_source') != ORACLE or transition.get('oracle_body_start') != MARKER:
        errors.append('terminal retirement: unreviewed oracle location')
    for key in ('original_body_sha256', 'oracle_body_sha256', 'retained_adapter_sha256'):
        if not re.fullmatch('[0-9a-f]{64}', transition.get(key, '')):
            errors.append(f'terminal retirement: invalid body fingerprint: {key}')
    if transition.get('original_body_sha256') != transition.get('oracle_body_sha256'):
        errors.append('terminal retirement: oracle must preserve original admission body exactly')
    if call_extension is None or transition.get('prior_call_extension_canonical_sha256') != canonical(call_extension):
        errors.append('terminal retirement: frozen call extension changed or unavailable')
        return errors
    if instrumentation is None:
        errors.append('terminal retirement: prior observation manifest unavailable')
        return errors
    before = call_extension['reviewed_selector']
    if transition.get('prior_selector') != before:
        errors.append('terminal retirement: prior selector is not bound to frozen evidence')
    old = 'typed_plan::lower_terminal_returns'
    after = transition.get('reviewed_selector', {})
    if before['ordered_calls'].count(old) != 1 or after.get('ordered_calls') != [c for c in before['ordered_calls'] if c != old]:
        errors.append('terminal retirement: unrelated selector calls/order changed')
    if not re.fullmatch('[0-9a-f]{64}', after.get('function_sha256', '')):
        errors.append('terminal retirement: invalid reviewed selector fingerprint')
    if transition.get('prior_source_fingerprints') != {SELECTOR: call_extension['reviewed_source_fingerprints'][SELECTOR],
                                                          TRACE: instrumentation['reviewed_source_fingerprints'][TRACE]}:
        errors.append('terminal retirement: prior observation owner is not bound to frozen evidence')
    fingerprints = transition.get('reviewed_source_fingerprints', {})
    if set(fingerprints) != SOURCES or any(
            not isinstance(sha, str) or not re.fullmatch('[0-9a-f]{64}', sha)
            for sha in fingerprints.values()):
        errors.append('terminal retirement: only reviewed selector and test capture owners may change')
    ids = sorted(REPLACEMENT if route == ROUTE else route for route in call_extension['instrumented_route_ids'])
    if transition.get('instrumented_route_ids') != ids:
        errors.append('terminal retirement: unrelated observation IDs changed')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('terminal retirement: unreviewed retirement set')
    matches = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': old + '(', 'baseline_occurrences': 1}]
    if len(matches) != 1 or matches[0]['anchors'] != anchor:
        errors.append('terminal retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    if re.search(r'\blower_terminal_returns\s*\(', sources.get(SELECTOR, '')):
        errors.append('terminal retirement: selector reintroduced old admission')
    typed = sources.get(TYPED, '')
    for declaration in (r'mod\s+terminal_returns_reference\s*;',
                        r'pub\(super\)\s+fn\s+lower_terminal_returns\b'):
        if (len(re.findall(declaration, typed)) != 1
                or len(re.findall(r'#\[cfg\(test\)\]\s*' + declaration, typed)) != 1):
            errors.append('terminal retirement: oracle module and wrapper must each be uniquely cfg(test)')
    adapter = sources.get(ORIGINAL, '')
    if re.search(r'\b(?:struct\s+Audit|impl\s+Audit|fn\s+(?:scalar|lower)\s*\()', adapter):
        errors.append('terminal retirement: shape admission remains in production adapter')
    if (adapter.count(ADAPTER) != 1
            or digest((adapter[adapter.index(ADAPTER):].strip() + '\n').encode()) != transition['retained_adapter_sha256']):
        errors.append('terminal retirement: retained lexical adapter differs from original')
    try:
        oracle = (root / ORACLE).read_text()
        if oracle.count(MARKER) != 1:
            errors.append('terminal retirement: oracle body anchor changed')
        elif digest((oracle[oracle.index(MARKER):].strip() + '\n').encode()) != transition['oracle_body_sha256']:
            errors.append('terminal retirement: oracle no longer equals original admission body')
    except (OSError, UnicodeError):
        errors.append('terminal retirement: oracle source unreadable')
    return errors
