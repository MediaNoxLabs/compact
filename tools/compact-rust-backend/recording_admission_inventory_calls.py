# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Explicit #519 call-context plumbing review, with no additional retirement."""
import re

from recording_admission_inventory_jubjub import canonical

RECORDED = 'tools/compact-rust-backend/src/recorded.rs'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
PROBE = 'tools/compact-rust-backend/src/recorded/admission_probe.rs'
SOURCES = {RECORDED, SELECTOR, PROBE}
RETIRED = ['effectful.effectful', 'selector.jubjub-values']


def validate(extension, transition, instrumentation, jubjub_transition):
    """Bind the three reviewed edits to their cumulative immutable predecessors.

    Current source hashes and hook IDs are still checked by the inventory's
    observation gate. This only authorizes their specifically reviewed evolution.
    """
    errors = []
    if extension.get('schema_version') != 1 or extension.get('issue') != 519:
        errors.append('call extension: unsupported reviewed manifest')
    if extension.get('classification') != 'stateful-call-context-plumbing-no-retirement':
        errors.append('call extension: unreviewed change classification')
    predecessors = (
        ('prior_effectful_transition_canonical_sha256', transition),
        ('prior_instrumentation_canonical_sha256', instrumentation),
        ('prior_jubjub_transition_canonical_sha256', jubjub_transition),
    )
    for name, predecessor in predecessors:
        if predecessor is None or extension.get(name) != canonical(predecessor):
            errors.append(f'call extension: frozen predecessor changed or unavailable: {name}')
    if any(value is None for _, value in predecessors):
        return errors
    prior_sources = {
        **instrumentation['reviewed_source_fingerprints'],
        **jubjub_transition['reviewed_source_fingerprints'],
    }
    if extension.get('prior_source_fingerprints') != {p: prior_sources[p] for p in SOURCES}:
        errors.append('call extension: prior source fingerprints are not bound to frozen evidence')
    reviewed = extension.get('reviewed_source_fingerprints', {})
    if set(reviewed) != SOURCES or any(
            not isinstance(sha, str) or not re.fullmatch('[0-9a-f]{64}', sha)
            for sha in reviewed.values()):
        errors.append('call extension: exactly three valid reviewed owner fingerprints required')
    if extension.get('prior_selector') != jubjub_transition['reviewed_selector']:
        errors.append('call extension: prior selector is not bound to frozen evidence')
    selector = extension.get('reviewed_selector', {})
    if selector.get('ordered_calls') != jubjub_transition['reviewed_selector']['ordered_calls']:
        errors.append('call extension: legacy selector calls or order changed')
    if not re.fullmatch('[0-9a-f]{64}', selector.get('function_sha256', '')):
        errors.append('call extension: invalid reviewed selector fingerprint')
    if extension.get('instrumented_route_ids') != jubjub_transition['instrumented_route_ids']:
        errors.append('call extension: observation IDs changed')
    if extension.get('retired_route_ids') != RETIRED or extension.get('new_retired_routes') != []:
        errors.append('call extension: existing retirements changed or new retirement claimed')
    return errors
