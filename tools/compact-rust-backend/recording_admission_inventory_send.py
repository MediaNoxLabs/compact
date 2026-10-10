# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Send retirement into existing composite admission; historical sources stay authenticated."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_membership import RETIRED as PRIOR_RETIRED

ISSUE = 561
CHECKPOINT = '1e7ec883a4646fd58b5d1949db8ce6ab47729946'
ROUTE = 'selector.shielded-send'
REPLACEMENT = 'algebra.composite-values'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
RECORDED = 'tools/compact-rust-backend/src/recorded.rs'
ZSWAP_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs'
HELPER = 'fn checked_composite_attempt('
GUARDS = ["pub(super) fn lower_shielded_send_checked<'a>(", 'fn shielded_send_value_checked(']
RETIRED = [*PRIOR_RETIRED, ROUTE]
# Filled from the parent-approved final source freeze, never from caller input.
MANIFEST_SHA = '660f74f8da0b8b2a35e9feba30966d2047e1094d045c00629aa049ffb09e02a6'
PRIOR_MEMBERSHIP = '59909b13f5a5c48f7288a38c186debdef278d032344298530a69c526f58dc3a1'
PRIOR_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '848b9bb95452de7444ee32b93982e08da21760011275604be1facdc019414ced', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '09ca695b0ae6253d162a51cf3d24162a52ea6fecfb83caad7b52385031aafa03', 'tools/compact-rust-backend/src/recorded.rs': 'd5efbed5f6e77bb982c96700e505a60bafd4ca564ff1d1d8663651d17d438e6b', 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs': 'cd955224fe2028ab88489c849c7fe013123751ddcb7326ad8a048f9caf737a7c'}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '4b583ff73a20b95ba340cef88afa24c065cbbff42f815e67965876b2799a9b6f', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': 'f6ee80c23043a60666da182c19508860412b872e9d88d85df4e8f3d968f11221', 'tools/compact-rust-backend/src/recorded.rs': '19c125b58bc1b284880ad061c55a3a6f3b0b3c21473bef76688ecea7b00fd9c6', 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs': 'cd955224fe2028ab88489c849c7fe013123751ddcb7326ad8a048f9caf737a7c'}


def predecessor_sources(root, transition):
    sources = {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}
    return predecessor_source_snapshot(sources, transition)


def predecessor_source_snapshot(sources, transition):
    """Authenticate production and adapted test owners before exposing historical bytes."""
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        raise ValueError('unreviewed send transition or reconstruction')
    if set(REVIEWED_OWNERS) != {SELECTOR, TYPED, RECORDED, ZSWAP_TEST} or set(PRIOR_OWNERS) != set(REVIEWED_OWNERS):
        raise ValueError('send production freeze is incomplete')
    result = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('current send owner differs from reviewed source: ' + owner)
    for owner, edits in transition['predecessor_reconstruction'].items():
        for edit in edits:
            current, previous = edit['current'], edit['previous']
            if not current or result[owner].count(current) != 1:
                raise ValueError('send adaptation is not unique: ' + owner)
            result[owner] = result[owner].replace(current, previous, 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('reconstructed source differs from frozen #560 predecessor: ' + owner)
    return result


def validate(baseline, transition, baseline_bytes, membership):
    errors = []
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        errors.append('send retirement: unreviewed transition identity or reconstruction')
    if (transition.get('schema_version') != 1 or transition.get('issue') != ISSUE
            or transition.get('reviewed_checkpoint') != CHECKPOINT
            or transition.get('retired_route') != ROUTE
            or transition.get('replacement_route') != REPLACEMENT):
        errors.append('send retirement: unsupported transition or checkpoint')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('send retirement: frozen baseline bytes changed')
    if membership is None or canonical(membership) != PRIOR_MEMBERSHIP:
        errors.append('send retirement: frozen membership560 predecessor changed or unavailable')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS):
        errors.append('send retirement: unreviewed owner identity')
    if membership is not None:
        before = membership.get('reviewed_selector', {})
        ordered = before.get('ordered_calls', [])
        if (not ordered or ordered.count('typed_plan::lower_shielded_send_checked') != 1
                or transition.get('prior_selector') != before
                or transition.get('reviewed_selector', {}).get('ordered_calls') != [call for call in ordered if call != 'typed_plan::lower_shielded_send_checked']):
            errors.append('send retirement: unrelated selector order changed')
        ids = membership.get('instrumented_route_ids', [])
        if (ids.count(ROUTE) != 1 or ids.count(REPLACEMENT) != 1
                or transition.get('instrumented_route_ids') != sorted(route for route in ids if route != ROUTE)):
            errors.append('send retirement: only the send observation may change')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('send retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(rows) != 1 or rows[0]['anchors'] != [{'source': SELECTOR, 'text': 'typed_plan::lower_shielded_send_checked(', 'baseline_occurrences': 1}]:
        errors.append('send retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    for owner, source in sources.items():
        if owner == ZSWAP_TEST:
            continue
        if re.search(r'\btyped_plan\s*::\s*lower_shielded_send_checked\s*\(', source):
            errors.append('send retirement: legacy root escaped test-only authority: ' + owner)
        if re.search(r'\buse\s+[^;]*typed_plan\s*::\s*(?:\{[^}]*\b)?lower_shielded_send_checked\b', source):
            errors.append('send retirement: legacy root import escaped test-only authority: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        typed = sources.get(TYPED, '')
        for marker in GUARDS:
            if typed.count('#[cfg(test)]\n' + marker) != 1:
                errors.append('send retirement: oracle declaration is not uniquely cfg(test): ' + marker)
            expected = transition['oracle_functions_sha256'][marker]
            if digest(function(typed, marker).encode()) != expected:
                errors.append('send retirement: original oracle body changed: ' + marker)
        if digest(function(sources.get(SELECTOR, ''), HELPER).encode()) != transition['helper_sha256']:
            errors.append('send retirement: interface helper changed')
        for path, expected in transition['shared_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('send retirement: retained shared lowering source changed: ' + path)
        for path, expected in transition['test_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('send retirement: reviewed routing tests changed: ' + path)
    except (OSError, UnicodeError, ValueError, KeyError) as error:
        errors.append('send retirement: source preservation failed: ' + str(error))
    return errors
