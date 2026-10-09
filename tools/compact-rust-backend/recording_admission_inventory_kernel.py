# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Specific #527 kernel-family retirement; the whole original planner stays test-only."""
import re

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_context import RETIRED as PRIOR_RETIRED

ROUTE = 'selector.kernel'
REPLACEMENT = 'algebra.kernel'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
RECORDED = 'tools/compact-rust-backend/src/recorded.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/kernel_plan.rs'
SOURCES = {SELECTOR, RECORDED}
RETIRED = [*PRIOR_RETIRED, ROUTE]


def validate(baseline, transition, baseline_bytes, context_transition, call_extension):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != 527
            or transition.get('retired_route') != ROUTE
            or transition.get('replacement_route') != REPLACEMENT):
        errors.append('kernel retirement: unsupported transition')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('kernel retirement: frozen baseline bytes changed')
    original = baseline['source_fingerprints'].get(ORACLE)
    if transition.get('original_source') != ORACLE or transition.get('original_source_sha256') != original:
        errors.append('kernel retirement: original source is not bound to baseline')
    if transition.get('oracle_source') != ORACLE or transition.get('oracle_source_sha256') != original:
        errors.append('kernel retirement: whole oracle must preserve original source exactly')
    if context_transition is None or transition.get('prior_context_transition_canonical_sha256') != canonical(context_transition):
        errors.append('kernel retirement: frozen context transition changed or unavailable')
        return errors
    if call_extension is None or transition.get('prior_call_extension_canonical_sha256') != canonical(call_extension):
        errors.append('kernel retirement: frozen call extension changed or unavailable')
        return errors
    before = context_transition['reviewed_selector']
    if transition.get('prior_selector') != before:
        errors.append('kernel retirement: prior selector is not bound to frozen evidence')
    old = 'kernel_plan::lower'
    after = transition.get('reviewed_selector', {})
    if before['ordered_calls'].count(old) != 1 or after.get('ordered_calls') != [call for call in before['ordered_calls'] if call != old]:
        errors.append('kernel retirement: unrelated selector calls/order changed')
    if not re.fullmatch('[0-9a-f]{64}', after.get('function_sha256', '')):
        errors.append('kernel retirement: invalid reviewed selector fingerprint')
    expected_prior = {SELECTOR: context_transition['reviewed_source_fingerprints'][SELECTOR],
                      RECORDED: call_extension['reviewed_source_fingerprints'][RECORDED]}
    if transition.get('prior_source_fingerprints') != expected_prior:
        errors.append('kernel retirement: prior observation owners are not bound to frozen evidence')
    fingerprints = transition.get('reviewed_source_fingerprints', {})
    if set(fingerprints) != SOURCES or any(
            not isinstance(sha, str) or not re.fullmatch('[0-9a-f]{64}', sha)
            for sha in fingerprints.values()):
        errors.append('kernel retirement: only reviewed selector and module-registration owners may change')
    ids = sorted(REPLACEMENT if route == ROUTE else route for route in context_transition['instrumented_route_ids'])
    if transition.get('instrumented_route_ids') != ids:
        errors.append('kernel retirement: unrelated observation IDs changed')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('kernel retirement: unreviewed retirement set')
    if transition.get('retained_shared_production_symbols') != []:
        errors.append('kernel retirement: this oracle has no shared production users')
    matches = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': old + '(', 'baseline_occurrences': 1}]
    if len(matches) != 1 or matches[0]['anchors'] != anchor:
        errors.append('kernel retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    if re.search(r'\bkernel_plan\b', sources.get(SELECTOR, '')):
        errors.append('kernel retirement: selector reintroduced old planner import or call')
    recorded = sources.get(RECORDED, '')
    declaration = r'mod\s+kernel_plan\s*;'
    if (len(re.findall(declaration, recorded)) != 1
            or len(re.findall(r'#\[cfg\(test\)\]\s*' + declaration, recorded)) != 1):
        errors.append('kernel retirement: whole oracle module must be uniquely cfg(test)')
    try:
        if digest((root / ORACLE).read_bytes()) != transition['oracle_source_sha256']:
            errors.append('kernel retirement: whole oracle differs from frozen original source')
    except OSError:
        errors.append('kernel retirement: original oracle source unreadable')
    return errors
