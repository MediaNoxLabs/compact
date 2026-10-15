# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Voting authority retirement with authenticated Unit540 predecessor recovery.

This is an exact source transition gate, not a semantic proof or Rust parser.
Historical JSON evidence and source identities cannot be rebound by a manifest.
"""
import re

from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_receive import SELECTOR, TYPED, RETIRED as PRIOR_RETIRED

ISSUE = 547
CHECKPOINT = 'd824d50b408261ab03729e0656231376227b409c'
ROUTE = 'selector.voting-commit'
REPLACEMENT = 'algebra.voting-commit'
ORACLE = 'tools/compact-rust-backend/src/recorded/typed_plan/voting_commit.rs'
ORACLE_SHA = '7a16bf5d0552c3ba82e680a32978280b68106f8373f2ea0eca63df4ddc8e1a40'
PRIOR_RECEIVE = '7650b6f40bd07439284b56c50399017ed2d921c1954683e9b56d110224007835'
PRIOR_UNIT = '4c0fbb77d697e658b6e25fec96edd0409988b37349de24b4fc70a988f3294d40'
PRIOR_OWNERS = {
    SELECTOR: '44cffbfa454deebcddf4f7b548430fc63862f75641eecc9e28ee3e9008a1893d',
    TYPED: '2b66a1b1ecf0580f1b01e2aed19a3b17762fc248c11b72d31c8af6695bd827ac',
}
# Exact reviewed source identities; changing a manifest cannot rebaseline them.
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '187dc9797b9ea9cdefe9e88e6022ca8b70197e1b0c258983a1057f2b6ce0ab6d', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '82ca3e48db05d654e93783ca2231a7269cc7040d6db0c0faad5aae89d1844521'}
WRAPPER_SHA = '824a79a5edc529bc3431776ea533ee3b4fd9b7cb09684a038eda0cafbf2256b4'
HELPER_SHA = 'eb73c6e8da07e6b55ab44bd66d81419efa6f54a741f18c2e131aa64523b776ec'
SUFFIX_SHA = '08119c170c54b409f675dd1ee08de15a7948f30b4d9b9c7e4a2a215437380df9'
RECONSTRUCTION_SHA = 'cf53fd14c6ac94c9653e7a699190592a6512ba433bd35d2f78b4290a47ae2d7e'
MARKER = "pub(super) fn lower_voting_commit<'a>("
HELPER = 'fn checked_voting_domain('
SELECT = 'pub(super) fn select('
MODULE = 'mod voting_commit;'
RETIRED = [*PRIOR_RETIRED, ROUTE]


def predecessor_selector(source, transition):
    """Undo only the reviewed new helper/module and complete selector suffix.

    Every fragment is pinned together; the result must match the immutable whole
    Unit540 owner. This does not allow a manifest to authorize arbitrary history.
    """
    if digest(source.encode()) != REVIEWED_OWNERS[SELECTOR]:
        raise ValueError('current selector owner differs from reviewed source')
    parts = transition.get('predecessor_reconstruction', {})
    if canonical(parts) != RECONSTRUCTION_SHA:
        raise ValueError('unreviewed reconstruction fragments')
    if source.count(SELECT) != 1:
        raise ValueError('selector function anchor is not unique')
    prefix = source[:source.index(SELECT)]
    for key in ('new_helper_region', 'new_test_module'):
        region = parts[key]
        if not region or prefix.count(region) != 1:
            raise ValueError('reconstruction region is not unique: ' + key)
        prefix = prefix.replace(region, '', 1)
    original = prefix + parts['prior_selector_suffix']
    if digest(original.encode()) != PRIOR_OWNERS[SELECTOR]:
        raise ValueError('reconstructed selector is not the frozen Unit540 owner')
    return original


def predecessor_typed(source):
    """Historical tests remove exactly the two reviewed cfg(test) guards."""
    if digest(source.encode()) != REVIEWED_OWNERS[TYPED]:
        raise ValueError('current typed owner differs from reviewed source')
    for declaration in (MODULE, MARKER):
        guarded = '#[cfg(test)]\n' + declaration
        if source.count(guarded) != 1:
            raise ValueError('typed reconstruction guard is not unique')
        source = source.replace(guarded, declaration, 1)
    if digest(source.encode()) != PRIOR_OWNERS[TYPED]:
        raise ValueError('reconstructed typed owner is not the frozen predecessor')
    return source


def validate(baseline, transition, baseline_bytes, receive, unit):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != ISSUE
            or transition.get('classification') != 'whole-voting-admission-retirement'
            or transition.get('reviewed_checkpoint') != CHECKPOINT
            or transition.get('retired_route') != ROUTE
            or transition.get('replacement_route') != REPLACEMENT):
        errors.append('voting retirement: unsupported transition or checkpoint')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('voting retirement: frozen baseline bytes changed')
    if (receive is None or canonical(receive) != PRIOR_RECEIVE
            or transition.get('prior_receive_transition_canonical_sha256') != PRIOR_RECEIVE):
        errors.append('voting retirement: frozen receive predecessor changed or unavailable')
    if (unit is None or canonical(unit) != PRIOR_UNIT
            or transition.get('prior_unit_domain_canonical_sha256') != PRIOR_UNIT):
        errors.append('voting retirement: frozen Unit540 predecessor changed or unavailable')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS):
        errors.append('voting retirement: reviewed predecessor/owner identities changed')
    if (transition.get('oracle_source') != ORACLE
            or transition.get('oracle_source_sha256') != ORACLE_SHA
            or transition.get('wrapper') != MARKER
            or transition.get('wrapper_sha256') != WRAPPER_SHA):
        errors.append('voting retirement: original oracle/wrapper identity changed')
    if (transition.get('helper') != HELPER or transition.get('helper_sha256') != HELPER_SHA
            or canonical(transition.get('predecessor_reconstruction', {})) != RECONSTRUCTION_SHA):
        errors.append('voting retirement: unreviewed domain or reconstruction')
    if unit is not None:
        before = unit.get('reviewed_selector', {})
        if transition.get('prior_selector') != before:
            errors.append('voting retirement: prior selector is not bound to Unit540 evidence')
        old = 'typed_plan::lower_voting_commit'
        after = transition.get('reviewed_selector', {})
        ordered = before.get('ordered_calls', [])
        if (ordered.count(old) != 1
                or after.get('ordered_calls') != [call for call in ordered if call != old]
                or after.get('function_sha256') != SUFFIX_SHA):
            errors.append('voting retirement: unrelated selector calls/order or position changed')
        ids = sorted(REPLACEMENT if route == ROUTE else route
                     for route in unit.get('instrumented_route_ids', []))
        if transition.get('instrumented_route_ids') != ids:
            errors.append('voting retirement: unrelated observation IDs changed')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('voting retirement: unreviewed retirement set')
    anchor = [{'source': SELECTOR, 'text': 'typed_plan::lower_voting_commit(', 'baseline_occurrences': 1}]
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    if len(rows) != 1 or rows[0]['anchors'] != anchor:
        errors.append('voting retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    for owner, source in sources.items():
        if owner not in (TYPED, ORACLE) and re.search(r'\b(?:lower_voting_commit|voting_commit)\b', source):
            errors.append('voting retirement: production owner reintroduced old admission import or call: ' + owner)
    typed = sources.get(TYPED, '')
    for declaration in (r'mod\s+voting_commit\s*;', r'pub\(super\)\s+fn\s+lower_voting_commit\b'):
        if (len(re.findall(declaration, typed)) != 1
                or len(re.findall(r'#\[cfg\(test\)\]\s*' + declaration, typed)) != 1):
            errors.append('voting retirement: oracle module and wrapper must be uniquely cfg(test)')
    try:
        wrapper = function(typed, MARKER)
        if digest(wrapper.encode()) != WRAPPER_SHA:
            errors.append('voting retirement: frozen wrapper body changed')
        outside = typed.replace(wrapper, '', 1)
        outside = re.sub(r'mod\s+voting_commit\s*;', '', outside)
        if (len(re.findall(r'\bvoting_commit\s*::\s*lower\s*\(', wrapper)) != 1
                or re.search(r'\b(?:voting_commit|lower_voting_commit)\b', outside)):
            errors.append('voting retirement: reference authority escaped the test-only wrapper')
        if digest((root / ORACLE).read_bytes()) != ORACLE_SHA:
            errors.append('voting retirement: frozen oracle source changed')
        selector = sources.get(SELECTOR, '')
        if selector.count(HELPER) != 1 or digest(function(selector, HELPER).encode()) != HELPER_SHA:
            errors.append('voting retirement: reviewed interface domain changed')
        for owner, expected in REVIEWED_OWNERS.items():
            if digest(sources.get(owner, '').encode()) != expected:
                errors.append('voting retirement: reviewed owner changed: ' + owner)
    except (OSError, UnicodeError, ValueError) as error:
        errors.append('voting retirement: unreadable oracle/helper: ' + str(error))
    return errors
