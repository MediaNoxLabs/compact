# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Shielded payout subsumption preserves shared production and prior authority."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_reset_payout import RETIRED as PRIOR_RETIRED

ROUTE = 'selector.shielded-payout'
REPLACEMENT = 'algebra.composite-values'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
PAYOUT = 'tools/compact-rust-backend/src/recorded/typed_plan/shielded_payout.rs'
RESET_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/reset_payout_migration_tests.rs'
GUARDED_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs'
VOTING_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs'
RETIRED = [*PRIOR_RETIRED, ROUTE]
# Reviewed source checkpoint; caller-supplied hashes cannot authorize drift.
MANIFEST_SHA = '147de987a048c220ad54c7bb285818d250a49cfa8c35dfe36a84e3915e23c2d9'
PRIOR_RESET = '854c6abd01cd887d925abbbea84e9daacdc7c196f1eede57e0c0e3b4d7482bb4'
PRIOR_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': 'c4f88e616810677d349143fa9ebc562e024a1a6e8239fefe67eb0d5d64c9274f', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': 'f7a3d7e2c66f22e7b0a5ef92b429a46c0ba4b288d34f581f73184018bfed53af', 'tools/compact-rust-backend/src/recorded/typed_plan/shielded_payout.rs': '7a13d8a6bacba6204cf4853f8fa350d8fb576248cce24b61f0256fcedd594049', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': 'b846ec4f245e18f57806a056410b0f70186a89aa3ac47a5f65d3f210cc94e565', 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs': '3cfa1683745e2ca0dabb6dcc4acc1fd6826c5d02c15eac1d79d1dda857dd8dc6', 'tools/compact-rust-backend/src/recorded/profile_selection/reset_payout_migration_tests.rs': '13124177ec003d36ce97dc1852b3f48cf85f1b4095dffc245f5a64725f37980e'}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '49504bf8cf28af98594d93d5bf204da3e1d6e82a8cfba5553865d6e89dcddba8', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '721e3cbecd18b1fa224525eb018082cfca62f9c9acc7e2f59256e78a1a091694', 'tools/compact-rust-backend/src/recorded/typed_plan/shielded_payout.rs': '0b9b24f53524b1558f8babece86ed4b96cef1470a5904ba4ac614e2b0f4c46cb', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': '0e066d75ff8db4df15063a663cae95f229bf3b46ca1bc632b6e9db07c3346888', 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs': '5b539db394c99906de05e26d4a6820f8accb37bbf1ddabb21be4be0de78aef2a', 'tools/compact-rust-backend/src/recorded/profile_selection/reset_payout_migration_tests.rs': '5314ffd2f76ee3a62afbf82d17433a5acd7aab9c9c36fcc7ff8be514d1244261'}


def predecessor_sources(root, transition):
    return predecessor_source_snapshot(
        {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}, transition)


def predecessor_source_snapshot(sources, transition):
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        raise ValueError('shielded payout retirement: unreviewed transition')
    if set(REVIEWED_OWNERS) != {SELECTOR, TYPED, PAYOUT, VOTING_TEST, GUARDED_TEST, RESET_TEST} or set(PRIOR_OWNERS) != set(REVIEWED_OWNERS):
        raise ValueError('shielded payout retirement: incomplete source freeze')
    result = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('shielded payout retirement: current owner differs: ' + owner)
    for owner, edits in transition['predecessor_reconstruction'].items():
        for edit in edits:
            if not edit['current'] or result[owner].count(edit['current']) != 1:
                raise ValueError('shielded payout retirement: adaptation not unique: ' + owner)
            result[owner] = result[owner].replace(edit['current'], edit['previous'], 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('shielded payout retirement: original owner reconstruction differs: ' + owner)
    return result


def validate(baseline, transition, baseline_bytes, reset):
    errors = []
    if canonical(transition) != MANIFEST_SHA:
        errors.append('shielded payout retirement: unreviewed manifest or reconstruction')
    if reset is None or canonical(reset) != PRIOR_RESET:
        errors.append('shielded payout retirement: immutable reset569 predecessor required')
    if baseline_bytes is None or digest(baseline_bytes) != transition.get('baseline_file_sha256'):
        errors.append('shielded payout retirement: frozen baseline bytes changed')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS):
        errors.append('shielded payout retirement: unreviewed owner identity')
    if reset is not None:
        if transition.get('prior_shared_sources_sha256') != reset.get('shared_sources_sha256'):
            errors.append('shielded payout retirement: immutable prior shared-source pins changed')
        prior = reset.get('reviewed_selector', {})
        calls = prior.get('ordered_calls', [])
        if (calls.count('typed_plan::lower_shielded_payout') != 1
                or transition.get('prior_selector') != prior
                or transition.get('reviewed_selector', {}).get('ordered_calls') != [x for x in calls if x != 'typed_plan::lower_shielded_payout']):
            errors.append('shielded payout retirement: unrelated selector order changed')
        expected = [x for x in reset.get('instrumented_route_ids', []) if x != ROUTE]
        if transition.get('instrumented_route_ids') != expected:
            errors.append('shielded payout retirement: remove only payout observation; existing composite route stays unique')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('shielded payout retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(rows) != 1 or rows[0]['anchors'] != [{'source': SELECTOR, 'text': 'typed_plan::lower_shielded_payout(', 'baseline_occurrences': 1}]:
        errors.append('shielded payout retirement: frozen anchor changed')
    return errors


def check_sources(root, sources, transition, prior_guarded_source=None,
                  prior_voting_test=None, prior_guarded_test=None):
    errors = []
    for owner, source in sources.items():
        if re.search(r'\btyped_plan\s*::\s*lower_shielded_payout\s*\(', source):
            errors.append('shielded payout retirement: old root call escaped authority: ' + owner)
        if re.search(r'\buse\s+[^;]*typed_plan\s*::\s*(?:\{[^}]*\b)?lower_shielded_payout\b', source):
            errors.append('shielded payout retirement: old root import escaped authority: ' + owner)
    for owner, source in sources.items():
        if owner not in (TYPED, PAYOUT) and re.search(r'\bshielded_payout\s*::\s*lower\b', source):
            errors.append('shielded payout retirement: old module root escaped authority: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        for owner, marker in [(TYPED, "pub(super) fn lower_shielded_payout<'a>("),
                              (PAYOUT, "pub(super) fn lower<'a>(")]:
            text = sources.get(owner, '')
            if text.count('#[cfg(test)]\n' + marker) != 1:
                errors.append('shielded payout retirement: root must be uniquely test-only: ' + owner)
            if digest(function(text, marker).encode()) != transition['oracle_functions_sha256'][owner]:
                errors.append('shielded payout retirement: frozen original root changed: ' + owner)
        if sources[PAYOUT].count('#[cfg(test)]\nfn coin_result(') != 1:
            errors.append('shielded payout retirement: private result recognizer must remain test-only')
        if digest(function(sources[PAYOUT], 'fn coin_result(').encode()) != transition['coin_result_sha256']:
            errors.append('shielded payout retirement: frozen result recognizer changed')
        for marker in ['fn add_paths(', 'pub(super) fn actionful_call_paths(']:
            if sources[PAYOUT].count('#[cfg(test)]\n' + marker) != 1:
                errors.append('shielded payout retirement: path recognizer must remain test-only: ' + marker)
        if digest(function(sources[SELECTOR], 'fn checked_composite_attempt(').encode()) != transition['existing_composite_attempt_sha256']:
            errors.append('shielded payout retirement: checked interface changed')
        for path, expected in transition['shared_sources_sha256'].items():
            data = (prior_guarded_source.encode() if prior_guarded_source is not None and path ==
                    'tools/compact-rust-backend/src/recorded/typed_plan/guarded_deposit.rs'
                    else (root / path).read_bytes())
            if digest(data) != expected:
                errors.append('shielded payout retirement: shared admission dependency changed: ' + path)
        prior_tests = {VOTING_TEST: prior_voting_test, GUARDED_TEST: prior_guarded_test}
        for path, expected in transition['test_sources_sha256'].items():
            previous = prior_tests.get(path)
            data = previous.encode() if previous is not None else (root / path).read_bytes()
            if digest(data) != expected:
                errors.append('shielded payout retirement: reviewed routing test changed: ' + path)
    except (OSError, UnicodeError, ValueError, KeyError) as error:
        errors.append('shielded payout retirement: source preservation failed: ' + str(error))
    return errors
