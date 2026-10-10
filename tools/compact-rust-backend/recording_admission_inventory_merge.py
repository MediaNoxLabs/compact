# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Merge subsumption preserves shared helper production and historical authority."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_send import RETIRED as PRIOR_RETIRED

ROUTE = 'selector.shielded-merge'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
MERGE = 'tools/compact-rust-backend/src/recorded/typed_plan/shielded_merge.rs'
VOTING_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs'
RETIRED = [*PRIOR_RETIRED, ROUTE]
# Parent-approved actual source freeze; caller-supplied hashes cannot authorize drift.
MANIFEST_SHA = '7160cff044c686cd5474df9209f5a6e4f1391940119c6a8ea731cb43c7d260cd'
PRIOR_SEND = '660f74f8da0b8b2a35e9feba30966d2047e1094d045c00629aa049ffb09e02a6'
PRIOR_OBSERVATION = 'bd0286e994fcc65fd9153a4ba06ea116a4c3b0b4cc2bbf36bab3c709f8300f17'
PRIOR_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '4b583ff73a20b95ba340cef88afa24c065cbbff42f815e67965876b2799a9b6f', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': 'f6ee80c23043a60666da182c19508860412b872e9d88d85df4e8f3d968f11221', 'tools/compact-rust-backend/src/recorded/typed_plan/shielded_merge.rs': '65a451b226b35a308575c10393a07313f73d4f28a18e3da0172807f96e5b4ffc', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': 'a0f695f2d9d4ed3bc3240e487e53affbb5b27c5a58fba6657f642766684bd03d'}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': 'ed6ca69ba8247356eed809790bdbe084174018c393abf8c5a514e817305078f9', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '13cc6c145b02901ed6032ff3e4b2308542ead1aae0c637131058773eb96fd1b7', 'tools/compact-rust-backend/src/recorded/typed_plan/shielded_merge.rs': '6f0bf14cb1d3647c0ced6d10e4a7c8bc4f7cbcbc11054cb8a826f68c5599657d', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': '51efa96c7008c25fba5abc295d96dd1def43bddd3541e2cb7f1881203c0965f2'}


def predecessor_sources(root, transition):
    return predecessor_source_snapshot(
        {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}, transition)


def predecessor_source_snapshot(sources, transition):
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        raise ValueError('merge retirement: unreviewed transition')
    if set(REVIEWED_OWNERS) != {SELECTOR, TYPED, MERGE, VOTING_TEST} or set(PRIOR_OWNERS) != set(REVIEWED_OWNERS):
        raise ValueError('merge retirement: incomplete source freeze')
    result = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('merge retirement: current owner differs: ' + owner)
    for owner, edits in transition['predecessor_reconstruction'].items():
        for edit in edits:
            if not edit['current'] or result[owner].count(edit['current']) != 1:
                raise ValueError('merge retirement: adaptation not unique: ' + owner)
            result[owner] = result[owner].replace(edit['current'], edit['previous'], 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('merge retirement: original owner reconstruction differs: ' + owner)
    return result


def validate(baseline, transition, baseline_bytes, send, observation):
    errors = []
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        errors.append('merge retirement: unreviewed manifest or reconstruction')
    if send is None or canonical(send) != PRIOR_SEND:
        errors.append('merge retirement: immutable send561 predecessor required')
    if observation is None or canonical(observation) != PRIOR_OBSERVATION:
        errors.append('merge retirement: immutable observation564 extension required')
    if baseline_bytes is None or digest(baseline_bytes) != transition.get('baseline_file_sha256'):
        errors.append('merge retirement: frozen baseline bytes changed')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS):
        errors.append('merge retirement: unreviewed owner identity')
    if send is not None:
        prior = send.get('reviewed_selector', {})
        calls = prior.get('ordered_calls', [])
        if (calls.count('typed_plan::lower_shielded_merge') != 1
                or transition.get('prior_selector') != prior
                or transition.get('reviewed_selector', {}).get('ordered_calls') != [x for x in calls if x != 'typed_plan::lower_shielded_merge']):
            errors.append('merge retirement: unrelated selector order changed')
        if transition.get('instrumented_route_ids') != sorted(x for x in send.get('instrumented_route_ids', []) if x != ROUTE):
            errors.append('merge retirement: only old merge observation may change')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('merge retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(rows) != 1 or rows[0]['anchors'] != [{'source': SELECTOR, 'text': 'typed_plan::lower_shielded_merge(', 'baseline_occurrences': 1}]:
        errors.append('merge retirement: frozen anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    for owner, source in sources.items():
        if re.search(r'\btyped_plan\s*::\s*lower_shielded_merge\s*\(', source):
            errors.append('merge retirement: old root call escaped authority: ' + owner)
        if re.search(r'\buse\s+[^;]*typed_plan\s*::\s*(?:\{[^}]*\b)?lower_shielded_merge\b', source):
            errors.append('merge retirement: old root import escaped authority: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        for owner, marker in [(TYPED, "pub(super) fn lower_shielded_merge<'a>("),
                              (MERGE, "pub(super) fn lower<'a>(")]:
            text = sources.get(owner, '')
            if text.count('#[cfg(test)]\n' + marker) != 1:
                errors.append('merge retirement: root must be uniquely test-only: ' + owner)
            if digest(function(text, marker).encode()) != transition['oracle_functions_sha256'][owner]:
                errors.append('merge retirement: frozen original root changed: ' + owner)
        for path, expected in transition['shared_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('merge retirement: shared admission dependency changed: ' + path)
        for path, expected in transition['test_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('merge retirement: reviewed routing test changed: ' + path)
    except (OSError, UnicodeError, ValueError, KeyError) as error:
        errors.append('merge retirement: source preservation failed: ' + str(error))
    return errors
