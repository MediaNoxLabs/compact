# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Final-priority Zswap admission retirement with an immutable whole test oracle."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_immediate import RETIRED as PRIOR_RETIRED

ISSUE = 553
CHECKPOINT = 'a04385a1e994a26715432119ce501dbad2903252'
ROUTE = 'selector.zswap'
REPLACEMENT = 'algebra.zswap'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
RECORDED = 'tools/compact-rust-backend/src/recorded.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/zswap_plan.rs'
ORACLE_SHA = '8a45fbd7203c3f4f533e8e2ae0516eecbeb6722624e39413446447f9840af37d'
PRIOR_IMMEDIATE = 'c9f6bb0c919dd1957b5e42ecb55ef4482b449380140add063a5cc3067dee28ca'
PRIOR_PHASE = 'd1eea74f0ddcb84f6bc3349997a40f66f6e24c9d0267d2493d03a192d660c16e'
PRIOR_VOTING = 'c8161e8f208d788420b2a0b5269d8693fe177c1e8dc1cad357a0ac4a70f54657'
PRIOR_UNIT = '4c0fbb77d697e658b6e25fec96edd0409988b37349de24b4fc70a988f3294d40'
PRIOR_OWNERS = {
    SELECTOR: '73f79bf0d1c48e3b3a4eaa418c3685afbd75283275a6e96a6dadf01fe3c2af13',
    RECORDED: 'fc0a4b58a3e2551520166dd493976795547a52f8040a6a55c50d24022fe74096',
}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded.rs': 'd5efbed5f6e77bb982c96700e505a60bafd4ca564ff1d1d8663651d17d438e6b', 'tools/compact-rust-backend/src/recorded/profile_selection.rs': 'f4a956c7b2e9dcc07e34be37a923068b15cacc45ce8358ffcd8aaee76e88af43'}
SUFFIX_SHA = '5f4a1e2847d4167832c2874e2ed13157ab3adcccbb4c298dcff0a326e6aac411'
RECONSTRUCTION_SHA = '39ea35d344cab7065a0055476b95b6baa0875f08a75987b2ce73f623c5794141'
TEST_SOURCE = 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs'
TEST_SHA = 'fa39f4c136d5a967f7bef644458b80a8c6a61ed18c69512306e285ac81f86a41'
TEST_REGISTRATION = '#[cfg(test)]\n#[path = "profile_selection/zswap_migration_tests.rs"]\nmod zswap_migration_tests;\n'
HELPER = 'fn checked_zswap_domain('
HELPER_SHA = 'bf21154fcf80ffc3982b75d94007612dacc7b91847d328558c22c772aa8b8018'
OLD_IMPORT = 'use super::{effect_plan, zswap_plan};'
NEW_IMPORT = 'use super::effect_plan;'
IMPORT_ANCHOR = 'use super::typed_plan::{self, CompositionRejection, TypedPlan};'
MODULE = 'mod zswap_plan;'
GUARDED_MODULE = '#[cfg(test)]\n' + MODULE
RETIRED = [*PRIOR_RETIRED, ROUTE]


def predecessor_sources(root, transition):
    sources = {owner: (root / owner).read_text() for owner in PRIOR_OWNERS}
    return predecessor_source_snapshot(sources, transition)


def predecessor_source_snapshot(sources, transition):
    """Return exact #550 bytes only after authenticating all current owners."""
    if set(REVIEWED_OWNERS) != set(PRIOR_OWNERS):
        raise ValueError('Zswap production freeze is incomplete')
    parts = transition.get('predecessor_reconstruction', {})
    if canonical(parts) != RECONSTRUCTION_SHA:
        raise ValueError('unreviewed Zswap reconstruction fragments')
    sources = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(sources[owner].encode()) != expected:
            raise ValueError('current Zswap owner differs from reviewed source: ' + owner)
    selector = sources[SELECTOR]
    marker = 'pub(super) fn select('
    if selector.count(marker) != 1:
        raise ValueError('Zswap selector anchor is not unique')
    selector = selector[:selector.index(marker)] + parts['prior_selector_suffix']
    for current, previous in [(parts['added_helper'], ''), (parts['added_test_registration'], ''), (NEW_IMPORT + '\n', '')]:
        if selector.count(current) != 1:
            raise ValueError('Zswap selector adaptation is not unique')
        selector = selector.replace(current, previous, 1)
    if selector.count(IMPORT_ANCHOR) != 1:
        raise ValueError('Zswap prior import position is not unique')
    sources[SELECTOR] = selector.replace(IMPORT_ANCHOR, IMPORT_ANCHOR + '\n' + OLD_IMPORT, 1)
    if sources[RECORDED].count(GUARDED_MODULE) != 1:
        raise ValueError('Zswap whole-oracle guard is not unique')
    sources[RECORDED] = sources[RECORDED].replace(GUARDED_MODULE, MODULE, 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(sources[owner].encode()) != expected:
            raise ValueError('reconstructed source differs from frozen #550 predecessor: ' + owner)
    return sources


def validate(baseline, transition, baseline_bytes, immediate, phase, voting, unit):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != ISSUE
            or transition.get('classification') != 'whole-zswap-admission-retirement'
            or transition.get('reviewed_checkpoint') != CHECKPOINT
            or transition.get('retired_route') != ROUTE or transition.get('replacement_route') != REPLACEMENT):
        errors.append('Zswap retirement: unsupported transition or checkpoint')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('Zswap retirement: frozen baseline bytes changed')
    for prior, expected, key, label in [
        (immediate, PRIOR_IMMEDIATE, 'prior_immediate_canonical_sha256', 'immediate550'),
        (phase, PRIOR_PHASE, 'prior_phase_canonical_sha256', 'phase549'),
        (voting, PRIOR_VOTING, 'prior_voting_canonical_sha256', 'voting547'),
        (unit, PRIOR_UNIT, 'prior_unit_domain_canonical_sha256', 'Unit540'),
    ]:
        if prior is None or canonical(prior) != expected or transition.get(key) != expected:
            errors.append('Zswap retirement: frozen ' + label + ' predecessor changed or unavailable')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS
            or canonical(transition.get('predecessor_reconstruction', {})) != RECONSTRUCTION_SHA):
        errors.append('Zswap retirement: unreviewed owner identity or reconstruction')
    if (transition.get('oracle_source') != ORACLE or transition.get('oracle_source_sha256') != ORACLE_SHA
            or baseline['source_fingerprints'].get(ORACLE) != ORACLE_SHA
            or transition.get('module_guard') != {'from': MODULE, 'to': GUARDED_MODULE}
            or transition.get('import_adaptation') != {'from': OLD_IMPORT, 'to': NEW_IMPORT, 'original_preceding_import': IMPORT_ANCHOR}
            or transition.get('helper') != HELPER or transition.get('helper_sha256') != HELPER_SHA
            or transition.get('retained_shared_production_symbols') != []
            or transition.get('test_module') != {'source': TEST_SOURCE, 'sha256': TEST_SHA, 'registration': TEST_REGISTRATION}):
        errors.append('Zswap retirement: oracle, interface helper or exact source adaptation changed')
    if immediate is not None:
        before = immediate.get('reviewed_selector', {})
        ordered = before.get('ordered_calls', [])
        after = transition.get('reviewed_selector', {})
        old = 'zswap_plan::lower'
        if (transition.get('prior_selector') != before or ordered.count(old) != 1
                or not ordered or ordered[-1] != old
                or after.get('ordered_calls') != ordered[:-1]
                or after.get('function_sha256') != SUFFIX_SHA):
            errors.append('Zswap retirement: unrelated selector calls/order or final priority changed')
        ids = immediate.get('instrumented_route_ids', [])
        if (ids.count(ROUTE) != 1 or REPLACEMENT in ids
                or transition.get('instrumented_route_ids') != sorted(REPLACEMENT if route == ROUTE else route for route in ids)):
            errors.append('Zswap retirement: replace only the final Zswap observation hook')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('Zswap retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': 'zswap_plan::lower(', 'baseline_occurrences': 1}]
    if len(rows) != 1 or rows[0]['anchors'] != anchor:
        errors.append('Zswap retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition):
    errors = []
    # Existing tests reside outside the frozen production-owner inventory. The
    # only permitted reference in these production owners is the guarded module.
    for owner, source in sources.items():
        if owner == ORACLE:
            continue
        if owner == RECORDED:
            source = source.replace(GUARDED_MODULE, '', 1)
        if re.search(r'\bzswap_plan\b', source):
            errors.append('Zswap retirement: legacy authority escaped test module: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        recorded = sources.get(RECORDED, '')
        if recorded.count(MODULE) != 1 or recorded.count(GUARDED_MODULE) != 1:
            errors.append('Zswap retirement: whole oracle module must be uniquely cfg(test)')
        if digest((root / ORACLE).read_bytes()) != ORACLE_SHA:
            errors.append('Zswap retirement: whole oracle differs from frozen original source')
        if digest((root / TEST_SOURCE).read_bytes()) != TEST_SHA:
            errors.append('Zswap retirement: reviewed routing test module changed')
        if digest(function(sources.get(SELECTOR, ''), HELPER).encode()) != HELPER_SHA:
            errors.append('Zswap retirement: declared interface helper changed')
    except (OSError, UnicodeError, ValueError) as error:
        errors.append('Zswap retirement: source preservation failed: ' + str(error))
    return errors
