# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Immediate-send retirement into the existing composite domain, with exact history."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_phase_reset import SELECTOR, TYPED, RETIRED as PRIOR_RETIRED

ISSUE = 550
CHECKPOINT = '33bf69099af1adaae3c8832cb715948165282139'
ROUTE = 'selector.immediate-send'
REPLACEMENT = 'algebra.composite-values'
IMMEDIATE = 'tools/compact-rust-backend/src/recorded/typed_plan/immediate_send.rs'
ATTEMPT = 'tools/compact-rust-backend/src/recorded/profile_attempt.rs'
PRIOR_PHASE = 'd1eea74f0ddcb84f6bc3349997a40f66f6e24c9d0267d2493d03a192d660c16e'
PRIOR_VOTING = 'c8161e8f208d788420b2a0b5269d8693fe177c1e8dc1cad357a0ac4a70f54657'
PRIOR_UNIT = '4c0fbb77d697e658b6e25fec96edd0409988b37349de24b4fc70a988f3294d40'
PRIOR_OWNERS = {
    SELECTOR: '443f667344342f9361709be3b2e65c96f347c7241287b13e1c2d9727afdd8e4e',
    TYPED: 'c5ce835cbd7e9a966b668653c4f66d3fc6f033a06f25d2c085172fa3cc7a95c8',
    IMMEDIATE: '91ff83866f77e2ca57f26e7c7644134887e674e1009ba3949b8f1a84c429fce6',
    ATTEMPT: 'e93f89ad2932c2f5db07a83d094900a060b5af7924516fb7c980d7e0cfcfff29',
}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '73f79bf0d1c48e3b3a4eaa418c3685afbd75283275a6e96a6dadf01fe3c2af13', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': '3b8683fbf667dc5a04cf9080dfc5437aae0cb116ffc84ed2ec081baee69821a8', 'tools/compact-rust-backend/src/recorded/typed_plan/immediate_send.rs': '02c5b5935b8cd9e141fea22abfd1ef72478107662f64e3481a0d61448ee8e7ae', 'tools/compact-rust-backend/src/recorded/profile_attempt.rs': '20943518e2036dd37f88de56e1bd98296072d6e961cb24464e4e196a1249c4d0'}
SUFFIX_SHA = 'd76643c34ca602c2bf41e48635f6608a0eadb44e7b8f2f0146e6abd9ecd791d3'
RECONSTRUCTION_SHA = '64a1eb5c00f7141490009e7cf1279158450428755b130863b0f6355682b94153'
ROOT_WRAPPER = "pub(super) fn lower_immediate_shielded_send<'a>("
SEND_WRAPPER = "pub(super) fn lower_shielded_send<'a>("
CHECKED_SEND = "pub(super) fn lower_shielded_send_checked<'a>("
GUARDS = {TYPED: [ROOT_WRAPPER, SEND_WRAPPER, 'fn shielded_send_value('],
          ATTEMPT: ['    pub(super) fn into_option('], IMMEDIATE: ['pub(super) fn audit_bridge(', "pub(super) fn lower<'a>("]}
VARIANT = '    ImmediateShieldedSend,'
GUARDED_VARIANT = '    #[cfg_attr(not(test), allow(dead_code))]\n' + VARIANT
OLD_IMPORT = 'use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};'
NEW_IMPORT = '#[cfg(test)]\nuse crate::coin_shapes::shielded_recipient_type;\nuse crate::coin_shapes::{qualified_coin_type, shielded_coin_type};'
COMPOSITE = 'fn checked_composite_domain('
COMPOSITE_SHA = 'ce16ffbf1b53347bad1e75592e1adbd1a0a9fc208cb6cd9b32117a827bb6a3f2'
WRAPPERS = {
    ROOT_WRAPPER: '093be9ec1593b825e077ed66d4db6a131d08270f11449a6709750c497f62d1cb',
    SEND_WRAPPER: '4c1986185ed53be7b9f064911f261c4c4f5669dbde8463e33f5ccd1da7597abf',
}
CHECKED_SEND_SHA = '2ed987d84b72774b196ad729a926549e242b0b85ebebaf49420c8d0bb1ebba60'
SHARED_FUNCTIONS = {
    'pub(super) fn uncoerced(': 'aef69b9ece919b65de3082a51e37a073e77b27485f7121ce67ef2ff3d84a8c4a',
    'pub(super) fn parameter(': '408ee657bde2fb7a179aca5f4d4ab400b9448f31c2e6fe01a6acc7c3baa406bd',
    'fn member(': '3d135d7bbb898869808ada271f2d3b9da2f9c3eebdf80e320a1afa9e989aaab9',
    'pub(super) fn singleton_bridge(': 'ec996910502d5737f0f891d983374ab8443234f24b855f15c021138590d1dc43',
}
SHARED_CALLERS = {
    'shielded_merge.rs': '65a451b226b35a308575c10393a07313f73d4f28a18e3da0172807f96e5b4ffc',
    'guarded_deposit.rs': 'd7c41e82b00576d5afd2f0857c92cd453e205a29571383ac80fe34a1f0f78c07',
    'start_funding.rs': '48bae139027a353b2ae462554b7735b839a9a85ba6ce4e4ad376483ceac950b5',
    'reset_payout.rs': '9f3d798a39e08f91aa5d33b2aff2771ab4be93848f6fd5a7f04bbd20fb11efc5',
    'voting_commit.rs': '7a16bf5d0552c3ba82e680a32978280b68106f8373f2ea0eca63df4ddc8e1a40',
}
SHARED_CALLERS = {f'tools/compact-rust-backend/src/recorded/typed_plan/{path}': sha for path, sha in SHARED_CALLERS.items()}
# Existing inline cfg(test) payout negatives call the now test-only Option shim.
# Pin the whole reviewed owner; an added production call or lost guard cannot be
# hidden by exempting its filename from the authority scan.
PAYOUT_TEST_OWNER = 'tools/compact-rust-backend/src/recorded/typed_plan/shielded_payout.rs'
PAYOUT_TEST_SHA = '7a13d8a6bacba6204cf4853f8fa350d8fb576248cce24b61f0256fcedd594049'
RETIRED = [*PRIOR_RETIRED, ROUTE]


def guarded(marker):
    indent = marker[:len(marker) - len(marker.lstrip())]
    return indent + '#[cfg(test)]\n' + marker


def predecessor_sources(root, transition):
    """Recover exact #549 owners, never a caller-supplied or rehashed history."""
    sources = {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}
    return predecessor_source_snapshot(sources, transition)


def predecessor_source_snapshot(sources, transition):
    """Validate exactly the #550 owners, including an authenticated later overlay."""
    sources = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    parts = transition.get('predecessor_reconstruction', {})
    if canonical(parts) != RECONSTRUCTION_SHA:
        raise ValueError('unreviewed immediate reconstruction fragments')
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(sources[owner].encode()) != expected:
            raise ValueError('current immediate owner differs from reviewed source: ' + owner)
    selector = sources[SELECTOR]
    marker = 'pub(super) fn select('
    if selector.count(marker) != 1:
        raise ValueError('selector anchor is not unique')
    sources[SELECTOR] = selector[:selector.index(marker)] + parts['prior_selector_suffix']
    for owner, markers in GUARDS.items():
        for marker in markers:
            guard = guarded(marker)
            if sources[owner].count(guard) != 1:
                raise ValueError('immediate oracle guard is not unique: ' + marker)
            sources[owner] = sources[owner].replace(guard, marker, 1)
    if sources[IMMEDIATE].count(NEW_IMPORT) != 1:
        raise ValueError('reviewed test-only import adaptation is not unique')
    sources[IMMEDIATE] = sources[IMMEDIATE].replace(NEW_IMPORT, OLD_IMPORT, 1)
    if sources[TYPED].count(GUARDED_VARIANT) != 1:
        raise ValueError('reviewed variant annotation is not unique')
    sources[TYPED] = sources[TYPED].replace(GUARDED_VARIANT, VARIANT, 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(sources[owner].encode()) != expected:
            raise ValueError('reconstructed source differs from frozen predecessor: ' + owner)
    return sources


def validate(baseline, transition, baseline_bytes, phase, voting, unit):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != ISSUE
            or transition.get('classification') != 'immediate-send-subsumed-by-existing-composite-domain'
            or transition.get('reviewed_checkpoint') != CHECKPOINT
            or transition.get('retired_route') != ROUTE or transition.get('replacement_route') != REPLACEMENT):
        errors.append('immediate retirement: unsupported transition or checkpoint')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('immediate retirement: frozen baseline bytes changed')
    for prior, expected, key, label in [
        (phase, PRIOR_PHASE, 'prior_phase_canonical_sha256', 'phase549'),
        (voting, PRIOR_VOTING, 'prior_voting_canonical_sha256', 'voting547'),
        (unit, PRIOR_UNIT, 'prior_unit_domain_canonical_sha256', 'Unit540'),
    ]:
        if prior is None or canonical(prior) != expected or transition.get(key) != expected:
            errors.append('immediate retirement: frozen ' + label + ' predecessor changed or unavailable')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS
            or canonical(transition.get('predecessor_reconstruction', {})) != RECONSTRUCTION_SHA):
        errors.append('immediate retirement: unreviewed owner identity or reconstruction')
    if (transition.get('oracle_source') != IMMEDIATE
            or transition.get('original_oracle_sha256') != PRIOR_OWNERS[IMMEDIATE]
            or baseline['source_fingerprints'].get(IMMEDIATE) != PRIOR_OWNERS[IMMEDIATE]
            or transition.get('guarded_declarations') != GUARDS
            or transition.get('variant_annotation') != {'from': VARIANT, 'to': GUARDED_VARIANT}
            or transition.get('import_adaptation') != {'from': OLD_IMPORT, 'to': NEW_IMPORT}
            or transition.get('wrapper_functions_sha256') != WRAPPERS):
        errors.append('immediate retirement: original oracle or exact guard adaptation changed')
    if (transition.get('shared_functions_sha256') != SHARED_FUNCTIONS
            or transition.get('shared_callers_sha256') != SHARED_CALLERS
            or transition.get('checked_send_sha256') != CHECKED_SEND_SHA
            or transition.get('existing_composite_domain_sha256') != COMPOSITE_SHA):
        errors.append('immediate retirement: retained shared policy or existing composite domain changed')
    if phase is not None:
        before = phase.get('reviewed_selector', {})
        ordered = before.get('ordered_calls', [])
        after = transition.get('reviewed_selector', {})
        old = 'typed_plan::lower_immediate_shielded_send'
        if (transition.get('prior_selector') != before or ordered.count(old) != 1
                or after.get('ordered_calls') != [call for call in ordered if call != old]
                or after.get('function_sha256') != SUFFIX_SHA):
            errors.append('immediate retirement: unrelated selector calls/order or position changed')
        ids = phase.get('instrumented_route_ids', [])
        if (ids.count(REPLACEMENT) != 1 or ids.count(ROUTE) != 1
                or transition.get('instrumented_route_ids') != [route for route in ids if route != ROUTE]):
            errors.append('immediate retirement: remove only immediate hook; existing composite hook stays unique')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('immediate retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': 'typed_plan::lower_immediate_shielded_send(', 'baseline_occurrences': 1}]
    if len(rows) != 1 or rows[0]['anchors'] != anchor:
        errors.append('immediate retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition, prior_merge=None, prior_guarded=None):
    """Later authenticated retirements may supply exact former shared-caller owners."""
    errors = []
    test_users = {TYPED, IMMEDIATE, 'tools/compact-rust-backend/src/recorded/typed_plan/voting_commit.rs'}
    for owner, source in sources.items():
        if owner == PAYOUT_TEST_OWNER:
            if digest(source.encode()) != PAYOUT_TEST_SHA:
                errors.append('immediate retirement: frozen payout test-owner source changed')
            continue
        if owner not in test_users and (re.search(r'\b(?:lower_immediate_shielded_send|lower_shielded_send)\b', source)
                or re.search(r'\bimmediate_send\s*::\s*(?:lower|audit_bridge)\b', source)
                or re.search(r'\buse\b[^;]*\bimmediate_send\b[^;]*\b(?:lower|audit_bridge)\b', source)):
            errors.append('immediate retirement: production owner reintroduced old authority: ' + owner)
    if PAYOUT_TEST_OWNER not in sources:
        errors.append('immediate retirement: frozen payout test-owner source missing')
    try:
        predecessor_source_snapshot(sources, transition)
        typed = sources.get(TYPED, '')
        if (len(re.findall(r'\bmod\s+immediate_send\s*;', typed)) != 1
                or re.search(r'#\[cfg\(test\)\]\s*mod\s+immediate_send\s*;', typed)):
            errors.append('immediate retirement: shared module must remain in production')
        outside = typed
        for marker, expected in WRAPPERS.items():
            body = function(typed, marker)
            if digest(body.encode()) != expected:
                errors.append('immediate retirement: original wrapper changed: ' + marker)
            outside = outside.replace(body, '', 1)
        if (re.search(r'\b(?:lower_immediate_shielded_send|lower_shielded_send)\b', outside)
                or re.search(r'\bimmediate_send\s*::\s*(?:lower|audit_bridge)\b', outside)):
            errors.append('immediate retirement: oracle authority escaped guarded wrappers')
        if digest(function(typed, CHECKED_SEND).encode()) != CHECKED_SEND_SHA:
            errors.append('immediate retirement: production checked send changed')
        if re.search(r'#\[cfg\(test\)\]\s*' + re.escape(CHECKED_SEND), typed):
            errors.append('immediate retirement: checked send must remain in production')
        module = sources.get(IMMEDIATE, '')
        for marker, expected in SHARED_FUNCTIONS.items():
            if digest(function(module, marker).encode()) != expected:
                errors.append('immediate retirement: shared helper changed: ' + marker)
        for owner, expected in SHARED_CALLERS.items():
            data = (prior_merge.encode() if prior_merge is not None and owner ==
                    'tools/compact-rust-backend/src/recorded/typed_plan/shielded_merge.rs'
                    else prior_guarded.encode() if prior_guarded is not None and owner ==
                    'tools/compact-rust-backend/src/recorded/typed_plan/guarded_deposit.rs'
                    else (root / owner).read_bytes())
            if digest(data) != expected:
                errors.append('immediate retirement: shared caller changed: ' + owner)
        if digest(function(sources.get(SELECTOR, ''), COMPOSITE).encode()) != COMPOSITE_SHA:
            errors.append('immediate retirement: existing composite domain changed')
    except (OSError, UnicodeError, ValueError) as error:
        errors.append('immediate retirement: source preservation failed: ' + str(error))
    return errors
