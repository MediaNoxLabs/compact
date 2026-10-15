# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Guarded deposit checked admission preserves shared production and prior authority."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_merge import RETIRED as PRIOR_RETIRED

ROUTE = 'selector.guarded-deposit'
REPLACEMENT = 'algebra.guarded-deposit'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
GUARDED = 'tools/compact-rust-backend/src/recorded/typed_plan/guarded_deposit.rs'
VOTING_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs'
RETIRED = [*PRIOR_RETIRED, ROUTE]
# Parent-approved actual source freeze; caller-supplied hashes cannot authorize drift.
MANIFEST_SHA = 'c3f02a85cae2dac6c680f2dc18897cc456b5712dc3b4a7335610b5fcb131a57d'
PRIOR_MERGE = '7160cff044c686cd5474df9209f5a6e4f1391940119c6a8ea731cb43c7d260cd'
PRIOR_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': 'ed6ca69ba8247356eed809790bdbe084174018c393abf8c5a514e817305078f9', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '13cc6c145b02901ed6032ff3e4b2308542ead1aae0c637131058773eb96fd1b7', 'tools/compact-rust-backend/src/recorded/typed_plan/guarded_deposit.rs': 'd7c41e82b00576d5afd2f0857c92cd453e205a29571383ac80fe34a1f0f78c07', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': '51efa96c7008c25fba5abc295d96dd1def43bddd3541e2cb7f1881203c0965f2'}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '4e4f5684aa67f63b5e5d3dd4a3da812bf636c618feb50b51c74724832bb44d14', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '698fcf518d8818d44014f1cc46fcd21c2cce96e4f4d30dc069de296eb884b4a1', 'tools/compact-rust-backend/src/recorded/typed_plan/guarded_deposit.rs': '4b1aad8edbfef0cf727469ce6e13d29067fffdbce3920c00bb9638c29a1f2894', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': 'a557bbb011848ca5ed1637162206f51c40c552ebe64691f9d2ff45fadf847f28'}


def predecessor_sources(root, transition):
    return predecessor_source_snapshot(
        {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}, transition)


def predecessor_source_snapshot(sources, transition):
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        raise ValueError('guarded deposit retirement: unreviewed transition')
    if set(REVIEWED_OWNERS) != {SELECTOR, TYPED, GUARDED, VOTING_TEST} or set(PRIOR_OWNERS) != set(REVIEWED_OWNERS):
        raise ValueError('guarded deposit retirement: incomplete source freeze')
    result = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('guarded deposit retirement: current owner differs: ' + owner)
    for owner, edits in transition['predecessor_reconstruction'].items():
        for edit in edits:
            if not edit['current'] or result[owner].count(edit['current']) != 1:
                raise ValueError('guarded deposit retirement: adaptation not unique: ' + owner)
            result[owner] = result[owner].replace(edit['current'], edit['previous'], 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('guarded deposit retirement: original owner reconstruction differs: ' + owner)
    return result


def validate(baseline, transition, baseline_bytes, merge):
    errors = []
    if canonical(transition) != MANIFEST_SHA:
        errors.append('guarded deposit retirement: unreviewed manifest or reconstruction')
    if merge is None or canonical(merge) != PRIOR_MERGE:
        errors.append('guarded deposit retirement: immutable merge565 predecessor required')
    if baseline_bytes is None or digest(baseline_bytes) != transition.get('baseline_file_sha256'):
        errors.append('guarded deposit retirement: frozen baseline bytes changed')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS):
        errors.append('guarded deposit retirement: unreviewed owner identity')
    if merge is not None:
        prior = merge.get('reviewed_selector', {})
        calls = prior.get('ordered_calls', [])
        if (calls.count('typed_plan::lower_guarded_deposit') != 1
                or transition.get('prior_selector') != prior
                or transition.get('reviewed_selector', {}).get('ordered_calls') != [x for x in calls if x != 'typed_plan::lower_guarded_deposit']):
            errors.append('guarded deposit retirement: unrelated selector order changed')
        expected = sorted(REPLACEMENT if x == ROUTE else x for x in merge.get('instrumented_route_ids', []))
        if transition.get('instrumented_route_ids') != expected:
            errors.append('guarded deposit retirement: only old guarded observation may change')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('guarded deposit retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(rows) != 1 or rows[0]['anchors'] != [{'source': SELECTOR, 'text': 'typed_plan::lower_guarded_deposit(', 'baseline_occurrences': 1}]:
        errors.append('guarded deposit retirement: frozen anchor changed')
    return errors


def check_sources(root, sources, transition, prior_voting_test=None, prior_routing_test=None):
    errors = []
    for owner, source in sources.items():
        if re.search(r'\btyped_plan\s*::\s*lower_guarded_deposit\s*\(', source):
            errors.append('guarded deposit retirement: old root call escaped authority: ' + owner)
        if re.search(r'\buse\s+[^;]*typed_plan\s*::\s*(?:\{[^}]*\b)?lower_guarded_deposit\b', source):
            errors.append('guarded deposit retirement: old root import escaped authority: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        for owner, marker in [(TYPED, "pub(super) fn lower_guarded_deposit<'a>("),
                              (GUARDED, "pub(super) fn lower<'a>(")]:
            text = sources.get(owner, '')
            if text.count('#[cfg(test)]\n' + marker) != 1:
                errors.append('guarded deposit retirement: root must be uniquely test-only: ' + owner)
            if digest(function(text, marker).encode()) != transition['oracle_functions_sha256'][owner]:
                errors.append('guarded deposit retirement: frozen original root changed: ' + owner)
        if digest(function(sources[SELECTOR], 'fn checked_guarded_deposit_domain(').encode()) != transition['checked_domain_sha256']:
            errors.append('guarded deposit retirement: checked interface changed')
        for path, expected in transition['shared_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('guarded deposit retirement: shared admission dependency changed: ' + path)
        for path, expected in transition['test_sources_sha256'].items():
            data = (prior_voting_test.encode() if prior_voting_test is not None and path == VOTING_TEST
                    else prior_routing_test.encode() if prior_routing_test is not None and path ==
                    'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs'
                    else (root / path).read_bytes())
            if digest(data) != expected:
                errors.append('guarded deposit retirement: reviewed routing test changed: ' + path)
    except (OSError, UnicodeError, ValueError, KeyError) as error:
        errors.append('guarded deposit retirement: source preservation failed: ' + str(error))
    return errors
