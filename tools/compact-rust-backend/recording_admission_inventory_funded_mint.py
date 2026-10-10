# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Funded mint subsumption preserves shared production and prior authority."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_shielded_payout import RETIRED as PRIOR_RETIRED

ROUTE = 'selector.funded-mint'
REPLACEMENT = 'algebra.composite-values'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
FUNDED = 'tools/compact-rust-backend/src/recorded/typed_plan/funded_mint.rs'
GUARDED = 'tools/compact-rust-backend/src/recorded/typed_plan/guarded_deposit.rs'
VOTING_DOMAIN = 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/domain.rs'
GUARDED_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs'
VOTING_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs'
RETIRED = [*PRIOR_RETIRED, ROUTE]
# Reviewed source checkpoint; caller-supplied hashes cannot authorize drift.
MANIFEST_SHA = '7f53232d199c81742c35217a2cecce5fdd379da0277d2d4287d25e2fe14428eb'
PRIOR_INEQUALITY = '7ff0e59ef07532f5e58816ae3c0226a28dd454c9961ec38ef3a1bf43a0f3ce9a'
PRIOR_PAYOUT = '147de987a048c220ad54c7bb285818d250a49cfa8c35dfe36a84e3915e23c2d9'
PRIOR_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '49504bf8cf28af98594d93d5bf204da3e1d6e82a8cfba5553865d6e89dcddba8', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '721e3cbecd18b1fa224525eb018082cfca62f9c9acc7e2f59256e78a1a091694', 'tools/compact-rust-backend/src/recorded/typed_plan/funded_mint.rs': 'a7ee91d7c739088adce9ce0edd630f43efa2b007519a92fe835a1a0540758a2e', 'tools/compact-rust-backend/src/recorded/typed_plan/guarded_deposit.rs': '4b1aad8edbfef0cf727469ce6e13d29067fffdbce3920c00bb9638c29a1f2894', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': '0e066d75ff8db4df15063a663cae95f229bf3b46ca1bc632b6e9db07c3346888', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/domain.rs': '0ba9c88e45bf12c694c4c5d187917bf3c3898ce787b534af2e99906b149fe6e3', 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs': '5b539db394c99906de05e26d4a6820f8accb37bbf1ddabb21be4be0de78aef2a'}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '43ce168c20e4c392a8a4d14ce6093eaa18a34ecce95757159742abccb37b56b0', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': 'c47f5eda8e0577c202690cfac5c6c0cb76c64b827b5ac99e63face9ad5b76017', 'tools/compact-rust-backend/src/recorded/typed_plan/funded_mint.rs': '08047d3c6aa05e3dd06d82c44082c4b6049dad25e4cf06f943cbbb6034c246cc', 'tools/compact-rust-backend/src/recorded/typed_plan/guarded_deposit.rs': 'a84159f4209a337b8fb5a916ae1416427ee6551f7baca4bc71e27a1ff594cacc', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': '3afc9c2b8f52cea1305f9986c579dfdb0a03ea62118b60a1e05d461a2c060bd3', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/domain.rs': '85f1d0e5b8611ba15c8f1214f499a1ba6d2626b0269074a137b19d85afb9d2e2', 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs': '77c5979abec69f6a2d0aca46a878322305141571cff178506d7d58e373a1dde0'}


def predecessor_sources(root, transition):
    return predecessor_source_snapshot(
        {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}, transition)


def predecessor_source_snapshot(sources, transition):
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        raise ValueError('funded mint retirement: unreviewed transition')
    if set(REVIEWED_OWNERS) != {SELECTOR, TYPED, FUNDED, GUARDED, VOTING_TEST, VOTING_DOMAIN, GUARDED_TEST} or set(PRIOR_OWNERS) != set(REVIEWED_OWNERS):
        raise ValueError('funded mint retirement: incomplete source freeze')
    result = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('funded mint retirement: current owner differs: ' + owner)
    for owner, edits in transition['predecessor_reconstruction'].items():
        for edit in edits:
            if not edit['current'] or result[owner].count(edit['current']) != 1:
                raise ValueError('funded mint retirement: adaptation not unique: ' + owner)
            result[owner] = result[owner].replace(edit['current'], edit['previous'], 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('funded mint retirement: original owner reconstruction differs: ' + owner)
    return result


def validate(baseline, transition, baseline_bytes, payout, inequality):
    errors = []
    if canonical(transition) != MANIFEST_SHA:
        errors.append('funded mint retirement: unreviewed manifest or reconstruction')
    if payout is None or canonical(payout) != PRIOR_PAYOUT:
        errors.append('funded mint retirement: immutable shielded570 predecessor required')
    if inequality is None or canonical(inequality) != PRIOR_INEQUALITY:
        errors.append('funded mint retirement: immutable inequality571 authority required')
    if baseline_bytes is None or digest(baseline_bytes) != transition.get('baseline_file_sha256'):
        errors.append('funded mint retirement: frozen baseline bytes changed')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS):
        errors.append('funded mint retirement: unreviewed owner identity')
    if payout is not None:
        if transition.get('prior_shared_sources_sha256') != payout.get('shared_sources_sha256'):
            errors.append('funded mint retirement: immutable prior shared-source pins changed')
        prior = payout.get('reviewed_selector', {})
        calls = prior.get('ordered_calls', [])
        if (calls.count('typed_plan::lower_funded_mint') != 1
                or transition.get('prior_selector') != prior
                or transition.get('reviewed_selector', {}).get('ordered_calls') != [x for x in calls if x != 'typed_plan::lower_funded_mint']):
            errors.append('funded mint retirement: unrelated selector order changed')
        expected = [x for x in payout.get('instrumented_route_ids', []) if x != ROUTE]
        if transition.get('instrumented_route_ids') != expected:
            errors.append('funded mint retirement: remove only funded mint observation; existing composite route stays unique')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('funded mint retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(rows) != 1 or rows[0]['anchors'] != [{'source': SELECTOR, 'text': 'typed_plan::lower_funded_mint(', 'baseline_occurrences': 1}]:
        errors.append('funded mint retirement: frozen anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    for owner, source in sources.items():
        if re.search(r'\btyped_plan\s*::\s*lower_funded_mint\s*\(', source):
            errors.append('funded mint retirement: old root call escaped authority: ' + owner)
        if re.search(r'\buse\s+[^;]*typed_plan\s*::\s*(?:\{[^}]*\b)?lower_funded_mint\b', source):
            errors.append('funded mint retirement: old root import escaped authority: ' + owner)
    for owner, source in sources.items():
        if owner not in (TYPED, FUNDED) and re.search(r'\bfunded_mint\s*::\s*lower\b', source):
            errors.append('funded mint retirement: old module root escaped authority: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        for owner, marker in [(TYPED, "pub(super) fn lower_funded_mint<'a>("),
                              (FUNDED, "pub(super) fn lower<'a>(")]:
            text = sources.get(owner, '')
            if text.count('#[cfg(test)]\n' + marker) != 1:
                errors.append('funded mint retirement: root must be uniquely test-only: ' + owner)
            if digest(function(text, marker).encode()) != transition['oracle_functions_sha256'][owner]:
                errors.append('funded mint retirement: frozen original root changed: ' + owner)
        for marker in ['#[derive(Clone, Copy, Default)]\nstruct Flow', 'struct Audit', "impl Audit<'_>", 'pub(super) fn prefix(']:
            if sources[GUARDED].count('#[cfg(test)]\n' + marker) != 1:
                errors.append('funded mint retirement: private prefix audit must remain test-only: ' + marker)
        if digest(function(sources[SELECTOR], 'fn checked_composite_attempt(').encode()) != transition['existing_composite_attempt_sha256']:
            errors.append('funded mint retirement: checked interface changed')
        for path, expected in transition['shared_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('funded mint retirement: shared admission dependency changed: ' + path)
        for path, expected in transition['test_sources_sha256'].items():
            if digest((root / path).read_bytes()) != expected:
                errors.append('funded mint retirement: reviewed routing test changed: ' + path)
    except (OSError, UnicodeError, ValueError, KeyError) as error:
        errors.append('funded mint retirement: source preservation failed: ' + str(error))
    return errors
