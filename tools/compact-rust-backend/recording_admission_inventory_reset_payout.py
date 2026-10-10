# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Reset payout subsumption preserves shared production and prior authority."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_guarded import RETIRED as PRIOR_RETIRED

ROUTE = 'selector.reset-payout'
REPLACEMENT = 'algebra.composite-values'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
RESET = 'tools/compact-rust-backend/src/recorded/typed_plan/reset_payout.rs'
GUARDED_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs'
VOTING_TEST = 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs'
RETIRED = [*PRIOR_RETIRED, ROUTE]
# Parent-approved actual source freeze; caller-supplied hashes cannot authorize drift.
MANIFEST_SHA = '854c6abd01cd887d925abbbea84e9daacdc7c196f1eede57e0c0e3b4d7482bb4'
PRIOR_ROUTING_TEST = '6a17304fb2cee492340a5ebc331c15b1e2465f3d54087aa3873162a0dddea4ce'
PRIOR_GUARDED = 'c3f02a85cae2dac6c680f2dc18897cc456b5712dc3b4a7335610b5fcb131a57d'
PRIOR_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '4e4f5684aa67f63b5e5d3dd4a3da812bf636c618feb50b51c74724832bb44d14', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '698fcf518d8818d44014f1cc46fcd21c2cce96e4f4d30dc069de296eb884b4a1', 'tools/compact-rust-backend/src/recorded/typed_plan/reset_payout.rs': '9f3d798a39e08f91aa5d33b2aff2771ab4be93848f6fd5a7f04bbd20fb11efc5', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': 'a557bbb011848ca5ed1637162206f51c40c552ebe64691f9d2ff45fadf847f28', 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs': '05bc7912426ce2240152027970513a8ddf58571fa45d25537a0bf9361f9c39a2'}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': 'c4f88e616810677d349143fa9ebc562e024a1a6e8239fefe67eb0d5d64c9274f', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': 'f7a3d7e2c66f22e7b0a5ef92b429a46c0ba4b288d34f581f73184018bfed53af', 'tools/compact-rust-backend/src/recorded/typed_plan/reset_payout.rs': 'bb690aef2530304937070a9982e4b2ba031d1cda2943b024ffb65d8f609eb037', 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': 'b846ec4f245e18f57806a056410b0f70186a89aa3ac47a5f65d3f210cc94e565', 'tools/compact-rust-backend/src/recorded/profile_selection/guarded_deposit_migration_tests/routing.rs': '3cfa1683745e2ca0dabb6dcc4acc1fd6826c5d02c15eac1d79d1dda857dd8dc6'}


def predecessor_sources(root, transition):
    return predecessor_source_snapshot(
        {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}, transition)


def predecessor_source_snapshot(sources, transition):
    if not MANIFEST_SHA or canonical(transition) != MANIFEST_SHA:
        raise ValueError('reset payout retirement: unreviewed transition')
    if set(REVIEWED_OWNERS) != {SELECTOR, TYPED, RESET, VOTING_TEST, GUARDED_TEST} or set(PRIOR_OWNERS) != set(REVIEWED_OWNERS):
        raise ValueError('reset payout retirement: incomplete source freeze')
    result = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('reset payout retirement: current owner differs: ' + owner)
    for owner, edits in transition['predecessor_reconstruction'].items():
        for edit in edits:
            if not edit['current'] or result[owner].count(edit['current']) != 1:
                raise ValueError('reset payout retirement: adaptation not unique: ' + owner)
            result[owner] = result[owner].replace(edit['current'], edit['previous'], 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(result[owner].encode()) != expected:
            raise ValueError('reset payout retirement: original owner reconstruction differs: ' + owner)
    return result


def validate(baseline, transition, baseline_bytes, guarded, routing_test):
    errors = []
    if canonical(transition) != MANIFEST_SHA:
        errors.append('reset payout retirement: unreviewed manifest or reconstruction')
    if guarded is None or canonical(guarded) != PRIOR_GUARDED:
        errors.append('reset payout retirement: immutable guarded566 predecessor required')
    if routing_test is None or canonical(routing_test) != PRIOR_ROUTING_TEST:
        errors.append('reset payout retirement: immutable routing568 authority required')
    if baseline_bytes is None or digest(baseline_bytes) != transition.get('baseline_file_sha256'):
        errors.append('reset payout retirement: frozen baseline bytes changed')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS):
        errors.append('reset payout retirement: unreviewed owner identity')
    if guarded is not None:
        prior = guarded.get('reviewed_selector', {})
        calls = prior.get('ordered_calls', [])
        if (calls.count('typed_plan::lower_reset_payout') != 1
                or transition.get('prior_selector') != prior
                or transition.get('reviewed_selector', {}).get('ordered_calls') != [x for x in calls if x != 'typed_plan::lower_reset_payout']):
            errors.append('reset payout retirement: unrelated selector order changed')
        expected = [x for x in guarded.get('instrumented_route_ids', []) if x != ROUTE]
        if transition.get('instrumented_route_ids') != expected:
            errors.append('reset payout retirement: remove only reset observation; existing composite route stays unique')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('reset payout retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(rows) != 1 or rows[0]['anchors'] != [{'source': SELECTOR, 'text': 'typed_plan::lower_reset_payout(', 'baseline_occurrences': 1}]:
        errors.append('reset payout retirement: frozen anchor changed')
    return errors


def check_sources(root, sources, transition, prior_payout=None, prior_voting_test=None,
                  prior_guarded_test=None, prior_reset_test=None):
    errors = []
    for owner, source in sources.items():
        if owner == 'tools/compact-rust-backend/src/recorded/profile_selection/reset_payout_migration_tests.rs':
            if digest(source.encode()) != transition['test_sources_sha256'].get(owner):
                errors.append('reset payout retirement: frozen routing test owner changed')
            continue
        if re.search(r'\btyped_plan\s*::\s*lower_reset_payout\s*\(', source):
            errors.append('reset payout retirement: old root call escaped authority: ' + owner)
        if re.search(r'\buse\s+[^;]*typed_plan\s*::\s*(?:\{[^}]*\b)?lower_reset_payout\b', source):
            errors.append('reset payout retirement: old root import escaped authority: ' + owner)
    for owner, source in sources.items():
        if owner not in (TYPED, RESET) and re.search(r'\breset_payout\s*::\s*lower\b', source):
            errors.append('reset payout retirement: old module root escaped authority: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        for owner, marker in [(TYPED, "pub(super) fn lower_reset_payout<'a>("),
                              (RESET, "pub(super) fn lower<'a>(")]:
            text = sources.get(owner, '')
            if text.count('#[cfg(test)]\n' + marker) != 1:
                errors.append('reset payout retirement: root must be uniquely test-only: ' + owner)
            if digest(function(text, marker).encode()) != transition['oracle_functions_sha256'][owner]:
                errors.append('reset payout retirement: frozen original root changed: ' + owner)
        if digest(function(sources[SELECTOR], 'fn checked_composite_attempt(').encode()) != transition['existing_composite_attempt_sha256']:
            errors.append('reset payout retirement: checked interface changed')
        for path, expected in transition['shared_sources_sha256'].items():
            data = (prior_payout.encode() if prior_payout is not None and path ==
                    'tools/compact-rust-backend/src/recorded/typed_plan/shielded_payout.rs'
                    else (root / path).read_bytes())
            if digest(data) != expected:
                errors.append('reset payout retirement: shared admission dependency changed: ' + path)
        prior_tests = {VOTING_TEST: prior_voting_test, GUARDED_TEST: prior_guarded_test,
                       'tools/compact-rust-backend/src/recorded/profile_selection/reset_payout_migration_tests.rs': prior_reset_test}
        for path, expected in transition['test_sources_sha256'].items():
            previous = prior_tests.get(path)
            data = previous.encode() if previous is not None else (root / path).read_bytes()
            if digest(data) != expected:
                errors.append('reset payout retirement: reviewed routing test changed: ' + path)
    except (OSError, UnicodeError, ValueError, KeyError) as error:
        errors.append('reset payout retirement: source preservation failed: ' + str(error))
    return errors
