# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Retire phase-reset authority into the existing, earlier kernel domain."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_voting import SELECTOR, TYPED, RETIRED as PRIOR_RETIRED

ISSUE = 549
CHECKPOINT = '3e61c009494064ad4de115d958865037603bb066'
ROUTE = 'selector.phase-reset'
REPLACEMENT = 'algebra.kernel'
PHASE = 'tools/compact-rust-backend/src/recorded/typed_plan/phase_reset.rs'
PRIOR_VOTING = 'c8161e8f208d788420b2a0b5269d8693fe177c1e8dc1cad357a0ac4a70f54657'
PRIOR_UNIT = '4c0fbb77d697e658b6e25fec96edd0409988b37349de24b4fc70a988f3294d40'
PRIOR_OWNERS = {
    SELECTOR: '187dc9797b9ea9cdefe9e88e6022ca8b70197e1b0c258983a1057f2b6ce0ab6d',
    TYPED: '82ca3e48db05d654e93783ca2231a7269cc7040d6db0c0faad5aae89d1844521',
    PHASE: '782df26e0640a4dbc58924385dafd5d85188d276990bf717f2aa952956768d8c',
}
REVIEWED_OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection.rs': '443f667344342f9361709be3b2e65c96f347c7241287b13e1c2d9727afdd8e4e', 'tools/compact-rust-backend/src/recorded/typed_plan.rs': 'c5ce835cbd7e9a966b668653c4f66d3fc6f033a06f25d2c085172fa3cc7a95c8', 'tools/compact-rust-backend/src/recorded/typed_plan/phase_reset.rs': 'e90a06b5b772e4f494e54ab2b06a9518ceae10c2ffd35c87e725e92462192227'}
SUFFIX_SHA = '15ec397e346c39e45f3ec870bd829d44085d375e9c2b7d6641e5ed73a5f169bf'
RECONSTRUCTION_SHA = '7f1691be98fb8b6ba2577623916f6bda3041465a0ba761eb5c7542fbd10980ad'
MARKER = "pub(super) fn lower_phase_reset<'a>("
GUARDS = {TYPED: [MARKER], PHASE: ['const U64:', "struct Audit<'a>", "impl Audit<'_>", "pub(super) fn lower<'a>("]}
KERNEL = 'fn checked_kernel_domain('
KERNEL_SHA = '9b4062ac7cdb299b7334dc431b2c180eff5bcd59f3ebf9c935f006ac3ddb9b92'
WRAPPER_SHA = '99aedfa027b5202a4773f6e0bb37e4803f0596234c9a5e133cf3f6333ed5cfb2'
SHARED_FUNCTIONS = {
    'pub(super) fn helper_signature(': '19b6dc1f57cc14d9e26f902d96e8d13eef42ce80e0336b6c671bc36d98fdc510',
    'pub(super) fn false_arguments(': '8e12ac395619a0d7282040cd91b161ee5bfacb41be8f91810914d44fcdffcbde',
    'pub(super) fn cell_type(': 'd19b1c22fc32fd2425cbb8e1d0fb479fd6a10c914cb8d20ca749a51f21229f4c',
}
SHARED_CALLERS = {
    'admission.rs': '656254b82f89f77bff1bca3c78b4b3a2bca22680d27550b0e6e1529cf3dc1675',
    'arithmetic.rs': 'e4e1a6f98263a2b5f663c601e28f18c4616e9116503259de23f381af9034be37',
    'observations.rs': '7f77c8e3e0b0babd846e339cac7067150ca4ac920ea6fc1dbb50d187a4285243',
    'actions/cells.rs': '2973b528939db63de51529a2c40af89ce1e7013f87afc3a22042316f8b203d66',
    'reset_payout.rs': '9f3d798a39e08f91aa5d33b2aff2771ab4be93848f6fd5a7f04bbd20fb11efc5',
}
SHARED_CALLERS = {f'tools/compact-rust-backend/src/recorded/typed_plan/{path}': sha for path, sha in SHARED_CALLERS.items()}
RETIRED = [*PRIOR_RETIRED, ROUTE]


def predecessor_sources(root, transition):
    """Authenticate all current owners before undoing the exact reviewed delta."""
    sources = {owner: (root / owner).read_text() for owner in REVIEWED_OWNERS}
    return predecessor_source_snapshot(sources, transition)


def predecessor_source_snapshot(sources, transition):
    """Validate exact supplied owner bytes, including authenticated later history."""
    parts = transition.get('predecessor_reconstruction', {})
    if canonical(parts) != RECONSTRUCTION_SHA:
        raise ValueError('unreviewed phase reconstruction fragments')
    sources = {owner: sources.get(owner, '') for owner in REVIEWED_OWNERS}
    for owner, expected in REVIEWED_OWNERS.items():
        if digest(sources[owner].encode()) != expected:
            raise ValueError('current phase owner differs from reviewed source: ' + owner)
    selector = sources[SELECTOR]
    marker = 'pub(super) fn select('
    if selector.count(marker) != 1:
        raise ValueError('selector anchor is not unique')
    prefix = selector[:selector.index(marker)]
    module = parts['new_test_module']
    if module:
        if prefix.count(module) != 1:
            raise ValueError('new phase test module is not unique')
        prefix = prefix.replace(module, '', 1)
    sources[SELECTOR] = prefix + parts['prior_selector_suffix']
    for owner, markers in GUARDS.items():
        for marker in markers:
            guarded = '#[cfg(test)]\n' + marker
            if sources[owner].count(guarded) != 1:
                raise ValueError('phase oracle guard is not unique: ' + marker)
            sources[owner] = sources[owner].replace(guarded, marker, 1)
    for owner, expected in PRIOR_OWNERS.items():
        if digest(sources[owner].encode()) != expected:
            raise ValueError('reconstructed source differs from frozen predecessor: ' + owner)
    return sources


def validate(baseline, transition, baseline_bytes, voting, unit):
    errors = []
    if (transition.get('schema_version') != 1 or transition.get('issue') != ISSUE
            or transition.get('classification') != 'phase-reset-subsumed-by-existing-kernel-domain'
            or transition.get('reviewed_checkpoint') != CHECKPOINT
            or transition.get('retired_route') != ROUTE or transition.get('replacement_route') != REPLACEMENT):
        errors.append('phase retirement: unsupported transition or checkpoint')
    if baseline_bytes is None or transition.get('baseline_file_sha256') != digest(baseline_bytes):
        errors.append('phase retirement: frozen baseline bytes changed')
    if (voting is None or canonical(voting) != PRIOR_VOTING
            or transition.get('prior_voting_canonical_sha256') != PRIOR_VOTING):
        errors.append('phase retirement: frozen voting predecessor changed or unavailable')
    if (unit is None or canonical(unit) != PRIOR_UNIT
            or transition.get('prior_unit_domain_canonical_sha256') != PRIOR_UNIT):
        errors.append('phase retirement: frozen Unit540 predecessor changed or unavailable')
    if (transition.get('prior_source_fingerprints') != PRIOR_OWNERS
            or transition.get('reviewed_source_fingerprints') != REVIEWED_OWNERS
            or canonical(transition.get('predecessor_reconstruction', {})) != RECONSTRUCTION_SHA):
        errors.append('phase retirement: unreviewed owner identity or reconstruction')
    if (transition.get('oracle_source') != PHASE
            or transition.get('original_oracle_sha256') != PRIOR_OWNERS[PHASE]
            or baseline['source_fingerprints'].get(PHASE) != PRIOR_OWNERS[PHASE]
            or transition.get('guarded_declarations') != GUARDS
            or transition.get('wrapper_sha256') != WRAPPER_SHA):
        errors.append('phase retirement: original oracle or exact guard adaptation changed')
    if (transition.get('shared_functions_sha256') != SHARED_FUNCTIONS
            or transition.get('shared_callers_sha256') != SHARED_CALLERS
            or transition.get('existing_kernel_domain_sha256') != KERNEL_SHA):
        errors.append('phase retirement: retained shared policy or existing kernel domain changed')
    if voting is not None:
        before = voting.get('reviewed_selector', {})
        old = 'typed_plan::lower_phase_reset'
        ordered = before.get('ordered_calls', [])
        after = transition.get('reviewed_selector', {})
        if (transition.get('prior_selector') != before or ordered.count(old) != 1
                or after.get('ordered_calls') != [call for call in ordered if call != old]
                or after.get('function_sha256') != SUFFIX_SHA):
            errors.append('phase retirement: unrelated selector calls/order or position changed')
        ids = voting.get('instrumented_route_ids', [])
        if (ids.count(REPLACEMENT) != 1 or ids.count(ROUTE) != 1
                or transition.get('instrumented_route_ids') != [route for route in ids if route != ROUTE]):
            errors.append('phase retirement: remove only the phase hook; existing kernel hook stays unique')
    if transition.get('retired_route_ids') != RETIRED or transition.get('new_retired_routes') != [ROUTE]:
        errors.append('phase retirement: unreviewed retirement set')
    rows = [row for row in baseline['routes'] if row['id'] == ROUTE]
    anchor = [{'source': SELECTOR, 'text': 'typed_plan::lower_phase_reset(', 'baseline_occurrences': 1}]
    if len(rows) != 1 or rows[0]['anchors'] != anchor:
        errors.append('phase retirement: frozen route anchor changed')
    return errors


def check_sources(root, sources, transition, prior_reset=None):
    errors = []
    # Shared phase_reset imports remain legitimate; only family authority is banned.
    for owner, source in sources.items():
        if owner not in (TYPED, PHASE) and (re.search(r'\blower_phase_reset\b', source)
                or re.search(r'\bphase_reset\s*::\s*(?:lower|Audit|U64)\b', source)
                or re.search(r'\buse\b[^;]*\bphase_reset\b[^;]*\b(?:lower|Audit|U64)\b', source)):
            errors.append('phase retirement: production owner reintroduced old authority: ' + owner)
    try:
        predecessor_source_snapshot(sources, transition)
        typed = sources.get(TYPED, '')
        if (len(re.findall(r'\bmod\s+phase_reset\s*;', typed)) != 1
                or re.search(r'#\[cfg\(test\)\]\s*mod\s+phase_reset\s*;', typed)):
            errors.append('phase retirement: shared module must remain in production')
        wrapper = function(typed, MARKER)
        if digest(wrapper.encode()) != WRAPPER_SHA:
            errors.append('phase retirement: original wrapper body changed')
        outside = typed.replace(wrapper, '', 1)
        if (re.search(r'\blower_phase_reset\b', outside)
                or re.search(r'\bphase_reset\s*::\s*(?:lower|Audit|U64)\b', outside)):
            errors.append('phase retirement: oracle authority escaped guarded wrapper')
        phase = sources.get(PHASE, '')
        for marker, expected in SHARED_FUNCTIONS.items():
            if digest(function(phase, marker).encode()) != expected:
                errors.append('phase retirement: shared helper changed: ' + marker)
        for owner, expected in SHARED_CALLERS.items():
            data = (prior_reset.encode() if prior_reset is not None and owner ==
                    'tools/compact-rust-backend/src/recorded/typed_plan/reset_payout.rs'
                    else (root / owner).read_bytes())
            if digest(data) != expected:
                errors.append('phase retirement: shared caller changed: ' + owner)
        if digest(function(sources.get(SELECTOR, ''), KERNEL).encode()) != KERNEL_SHA:
            errors.append('phase retirement: existing kernel domain changed')
    except (OSError, UnicodeError, ValueError) as error:
        errors.append('phase retirement: source preservation failed: ' + str(error))
    return errors
