# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""First-priority membership retirement; the shared typed renderer stays live."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_zswap import RETIRED as PRIOR_RETIRED

ISSUE = 560
CHECKPOINT = 'e8bb00351f7bb2367c7b7ea5cc19ffa6dbbeb28b'
ROUTE = 'selector.membership-optional'
REPLACEMENT = 'algebra.membership'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
ZSWAP_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs'
HELPER = 'fn checked_membership_domain('
GUARDS = ["pub(super) fn lower<'a>(", 'fn assertion_value(', 'fn assertion_type(']
RETIRED = [*PRIOR_RETIRED, ROUTE]
# Filled from the parent-approved final source freeze, never from caller input.
MANIFEST_SHA = '59909b13f5a5c48f7288a38c186debdef278d032344298530a69c526f58dc3a1'
PRIOR_ZSWAP = 'd0bd62c6624e4c21c2e0926ec010ceb6c343e56291b087605d98e95da1d5a573'
PRIOR_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs': 'fa39f4c136d5a967f7bef644458b80a8c6a61ed18c69512306e285ac81f86a41', 'tools/compact-rust-backend/src/recorded/profile_selection.rs': 'f4a956c7b2e9dcc07e34be37a923068b15cacc45ce8358ffcd8aaee76e88af43', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '3b8683fbf667dc5a04cf9080dfc5437aae0cb116ffc84ed2ec081baee69821a8'}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs': 'cd955224fe2028ab88489c849c7fe013123751ddcb7326ad8a048f9caf737a7c', 'tools/compact-rust-backend/src/recorded/profile_selection.rs': '848b9bb95452de7444ee32b93982e08da21760011275604be1facdc019414ced', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '09ca695b0ae6253d162a51cf3d24162a52ea6fecfb83caad7b52385031aafa03'}


def predecessor_sources(root, transition):
    sources = {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}
    return predecessor_source_snapshot(sources, transition)


def predecessor_source_snapshot(sources, transition):
    """Authenticate production and adapted test owners before exposing historical bytes."""
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        raise ValueError('unreviewed membership transition or reconstruction')
    if set(REVIEWED_OWNERS) != {SELECTOR, TYPED, ZSWAP_TEST} or set(PRIOR_OWNERS) != set(REVIEWED_OWNERS):
        raise ValueError('membership production freeze is incomplete')
    result = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('current membership owner differs from reviewed source: ' + owner)
    for owner, edits in transition['predecessor_reconstruction'].items():
        for edit in edits:
            current, previous = edit['current'], edit['previous']
            if not current or result[owner].count(current) != 1:
                raise ValueError('membership adaptation is not unique: ' + owner)
            result[owner] = result[owner].replace(current, previous, 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('reconstructed source differs from frozen #553 predecessor: ' + owner)
    return result


def validate(baseline, transition, baseline_bytes, zswap):
    errors = []
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        errors.append('membership retirement: unreviewed transition identity or reconstruction')
    if (transition.get('schema_version') != 1 or transition.get('issue') != ISSUE
            or transition.get('reviewed_checkpoint') != CHECKPOINT
            or transition.get('retired_route') != ROUTE
            or transition.get('replacement_route') != REPLACEMENT):
        errors.append('membership retirement: unsupported transition or checkpoint')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('membership retirement: frozen baseline bytes changed')
    if zswap is None or canonical(zswap) != PRIOR_ZSWAP:
        errors.append('membership retirement: frozen Zswap553 predecessor changed or unavailable')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS):
        errors.append('membership retirement: unreviewed owner identity')
    if zswap is not None:
        before = zswap.get('reviewed_selector', {})
        ordered = before.get('ordered_calls', [])
        if (not ordered or ordered[0] != 'typed_plan::lower'
                or transition.get('prior_selector') != before
                or transition.get('reviewed_selector', {}).get('ordered_calls') != ordered[1:]):
            errors.append('membership retirement: first priority or unrelated selector order changed')
        ids = zswap.get('instrumented_route_ids', [])
        if (ids.count(ROUTE) != 1 or REPLACEMENT in ids
                or transition.get('instrumented_route_ids') != sorted(REPLACEMENT if route == ROUTE else route for route in ids)):
            errors.append('membership retirement: only the membership observation may change')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('membership retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(rows) != 1 or rows[0]['anchors'] != [{'source': SELECTOR, 'text': 'typed_plan::lower(', 'baseline_occurrences': 1}]:
        errors.append('membership retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    for owner, source in sources.items():
        if owner == ZSWAP_TEST:
            continue
        if re.search(r'\btyped_plan\s*::\s*lower\s*\(', source):
            errors.append('membership retirement: legacy root escaped test-only authority: ' + owner)
        if re.search(r'\buse\s+[^;]*typed_plan\s*::\s*(?:\{[^}]*\b)?lower\b', source):
            errors.append('membership retirement: legacy root import escaped test-only authority: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        typed = sources.get(TYPED, '')
        for marker in GUARDS:
            if typed.count('#[cfg(test)]\n' + marker) != 1:
                errors.append('membership retirement: oracle declaration is not uniquely cfg(test): ' + marker)
            expected = transition['oracle_functions_sha256'][marker]
            if digest(function(typed, marker).encode()) != expected:
                errors.append('membership retirement: original oracle body changed: ' + marker)
        if digest(function(sources.get(SELECTOR, ''), HELPER).encode()) != transition['helper_sha256']:
            errors.append('membership retirement: interface helper changed')
        for path, expected in transition['shared_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('membership retirement: retained shared lowering source changed: ' + path)
        for path, expected in transition['test_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('membership retirement: reviewed routing tests changed: ' + path)
    except (OSError, UnicodeError, ValueError, KeyError) as error:
        errors.append('membership retirement: source preservation failed: ' + str(error))
    return errors
