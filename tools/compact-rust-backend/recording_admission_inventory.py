#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Verify the frozen EA-00 admission route inventory; never rewrite its baseline.

This is a source-change gate, not a Rust parser or language coverage measure.
Exact anchor counts and the main selector's ordered calls/function digest are
checked. Other source drift is reported, or rejected by --strict-fingerprints.
A reviewed retirement is applied only with --transition; the original baseline
stays immutable. A transition never suppresses unrelated anchor/selector errors.
Test-only observation hooks require an explicit --instrumentation manifest. It
pins the reviewed source edits and unchanged selector call order without replacing
the original baseline or retirement evidence. This is not semantic equivalence
proof; Rust tests separately check evaluation, refusal, and output preservation.
Call-context plumbing requires --call-extension after these frozen predecessors;
it retires no additional route and cannot change selector order or hook IDs.
Terminal retirement requires --terminal-transition after #519 and preserves the
original denominator, prior retirements, and remaining legacy selector order.
Unit-actions retirement requires --unit-actions-transition after #521; shared
legacy predicates and policy remain active for independent families.
Context-query retirement requires --context-query-transition after #525; shared
context predicates and VotingCommit evaluation remain active.
Whole composite retirement requires --composite-transition after #527.
Whole receive retirement requires --receive-transition after #529.
Ordinary Unit return expansion requires --unit-domain-extension after #536; it retires no route.
Voting retirement requires --voting-transition after #540 and authenticates its exact predecessor.
Phase-reset retirement requires --phase-reset-transition after #547; the kernel owner already exists.
Immediate-send retirement requires --immediate-send-transition after #549, reusing the composite owner.
Zswap retirement requires --zswap-transition after #550 and preserves the final selector priority.
Membership retirement requires --membership-transition after #553; the shared typed renderer stays live.
Send retirement requires --send-transition after #560; existing composite admission owns checked sends.
Kernel retirement requires --kernel-transition after #526; its unchanged planner
is compiled only for tests and cannot be a production selector dependency.
Human review must classify new/changed routes before a future baseline update.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

import recording_admission_inventory_jubjub as jubjub
import recording_admission_inventory_calls as calls
import recording_admission_inventory_terminal as terminal
import recording_admission_inventory_unit as unit
import recording_admission_inventory_context as context
import recording_admission_inventory_kernel as kernel
import recording_admission_inventory_composite as composite
import recording_admission_inventory_receive as receive
import recording_admission_inventory_unit_domain as unit_domain
import recording_admission_inventory_voting as voting
import recording_admission_inventory_phase_reset as phase_reset
import recording_admission_inventory_immediate as immediate
import recording_admission_inventory_zswap as zswap
import recording_admission_inventory_membership as membership
import recording_admission_inventory_send as send
import recording_admission_inventory_observation_test as observation_test
import recording_admission_inventory_merge as merge
import recording_admission_inventory_guarded as guarded
import recording_admission_inventory_routing_test as routing_test
import recording_admission_inventory_reset_payout as reset_payout

ROOT = Path(__file__).resolve().parents[2]
BASELINE = Path(__file__).with_suffix('.baseline.json')
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
CALL = re.compile(r'\b(?:typed_plan::lower\w*|kernel_plan::lower|zswap_plan::lower)\s*\(')
RETIRED_EFFECTFUL = 'effectful.effectful'
RECORDED = 'tools/compact-rust-backend/src/recorded.rs'
TYPED_PLAN = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
ORACLE = 'tools/compact-rust-backend/src/recorded/typed_plan/effectful_reference.rs'
INSTRUMENTATION = Path(__file__).with_name('recording_admission_inventory.instrumentation-515.json')
HOOK_SOURCES = (
    RECORDED, SELECTOR,
    'tools/compact-rust-backend/src/recorded/helpers.rs',
    'tools/compact-rust-backend/src/recorded/admission_probe.rs',
    'tools/compact-rust-backend/src/recorded/admission_probe/trace.rs',
    'tools/compact-rust-backend/src/recorded/admission_probe/attribution.rs',
)
DISPOSITIONS = {'replace-shape', 'preserve-semantic', 'split-semantic-from-shape'}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def selector_signature(source: str) -> dict:
    # The selector is the final top-level function in this small owner module.
    # Its complete suffix digest catches conditions/order and added routes,
    # including calls outside CALL's deliberately narrow baseline naming set.
    marker = 'pub(super) fn select('
    if source.count(marker) != 1:
        raise ValueError('main selector function anchor must occur exactly once')
    body = source[source.index(marker):]
    return {'ordered_calls': [m.group().rstrip(' (\n\t') for m in CALL.finditer(body)],
            'function_sha256': digest(body.encode())}


def validate_baseline(baseline: dict) -> list[str]:
    errors = []
    if baseline.get('schema_version') != 1:
        errors.append('unsupported inventory schema')
    routes = baseline.get('routes', [])
    ids = [row.get('id') for row in routes]
    if len(ids) != len(set(ids)) or not all(isinstance(i, str) and i for i in ids):
        errors.append('route IDs must be unique nonempty strings')
    fingerprints = baseline.get('source_fingerprints', {})
    for source, sha in fingerprints.items():
        if Path(source).is_absolute() or '..' in Path(source).parts:
            errors.append(f'unsafe source path: {source}')
        if not isinstance(sha, str) or not re.fullmatch('[0-9a-f]{64}', sha):
            errors.append(f'invalid source fingerprint: {source}')
    for row in routes:
        name = row.get('id', '<missing>')
        if row.get('disposition') not in DISPOSITIONS:
            errors.append(f'{name}: invalid disposition')
        if row.get('baseline_status') != 'production-active':
            errors.append(f'{name}: invalid baseline status')
        packages = row.get('planned_packages', [])
        if not packages or any(not re.fullmatch(r'EA-(?:0[0-9]|1[0-3])', p) for p in packages):
            errors.append(f'{name}: invalid planned package')
        if not row.get('anchors'):
            errors.append(f'{name}: missing anchors')
        for anchor in row.get('anchors', []):
            if anchor.get('source') not in fingerprints:
                errors.append(f'{name}: anchor source has no fingerprint')
            if not anchor.get('text') or not isinstance(anchor.get('baseline_occurrences'), int) or anchor.get('baseline_occurrences', 0) < 1:
                errors.append(f'{name}: invalid anchor')
    if not re.fullmatch('[0-9a-f]{40}', baseline.get('baseline_commit', '')):
        errors.append('invalid baseline commit')
    return errors


def retired_effectful_checks(sources: dict[str, str], oracle: str, transition: dict) -> list[str]:
    """Specific retirement obligations; intentionally not a generic ignore list."""
    errors = []
    recorded = sources.get(RECORDED, '')
    typed = sources.get(TYPED_PLAN, '')
    if re.search(r'\blower_effectful\s*\(', recorded):
        errors.append('retirement: recorded.rs reintroduced a lower_effectful call')
    for name in ('lower_effectful', 'effectful_value', 'effectful_bindings', 'effectful_action', 'effectful_body'):
        if re.search(r'\bfn\s+' + name + r'\b', typed):
            errors.append(f'retirement: {name} remains defined in production typed_plan.rs')
    declarations = (
        r'mod\s+effectful_reference\s*;',
        r'pub\(super\)\s+use\s+effectful_reference::lower_effectful\s*;',
    )
    for declaration in declarations:
        guarded = r'#\[cfg\(test\)\]\s*' + declaration
        if len(re.findall(declaration, typed)) != 1 or len(re.findall(guarded, typed)) != 1:
            errors.append('retirement: oracle module and re-export must each be uniquely cfg(test)')
    marker = transition['oracle_body_start']
    if oracle.count(marker) != 1:
        errors.append('retirement: oracle body anchor changed')
    else:
        # The moved module needs one explicit visibility adaptation. Both raw
        # and original-normalized body digests are pinned; no other rewrite.
        body = oracle[oracle.index(marker):].strip() + '\n'
        visibility = 'pub(in crate::recorded) fn lower_effectful'
        original_visibility = 'pub(super) fn lower_effectful'
        if body.count(visibility) != 1 or digest(body.encode()) != transition['oracle_body_sha256']:
            errors.append('retirement: test oracle body or visibility changed')
        normalized = body.replace(visibility, original_visibility, 1)
        if digest(normalized.encode()) != transition['original_body_sha256']:
            errors.append('retirement: test oracle differs from frozen original body')
    return errors


def validate_transition(baseline: dict, transition: dict, baseline_bytes: bytes | None) -> list[str]:
    errors = []
    if transition.get('schema_version') != 1 or transition.get('retired_route') != RETIRED_EFFECTFUL:
        errors.append('unsupported retirement transition; only effectful.effectful is reviewed')
    if baseline_bytes is None or digest(baseline_bytes) != transition.get('baseline_file_sha256'):
        errors.append('transition: frozen baseline file fingerprint changed or unavailable')
    elif json.loads(baseline_bytes) != baseline:
        errors.append('transition: decoded baseline differs from supplied frozen bytes')
    if transition.get('baseline_commit') != baseline.get('baseline_commit'):
        errors.append('transition: baseline commit mismatch')
    if transition.get('original_source') != TYPED_PLAN or transition.get('original_source_sha256') != baseline['source_fingerprints'].get(TYPED_PLAN):
        errors.append('transition: original owner fingerprint is not bound to frozen baseline')
    marker = '// Bounded admission is separate from the existing single-slot root-Let profile.'
    if transition.get('oracle_source') != ORACLE or transition.get('oracle_body_start') != marker:
        errors.append('transition: invalid test oracle location or body anchor')
    visibility = {'from': 'pub(super) fn lower_effectful',
                  'to': 'pub(in crate::recorded) fn lower_effectful'}
    if transition.get('visibility_adaptation') != visibility:
        errors.append('transition: unreviewed oracle visibility adaptation')
    if any(not re.fullmatch('[0-9a-f]{64}', transition.get(key, ''))
           for key in ('original_body_sha256', 'oracle_body_sha256')):
        errors.append('transition: invalid original body fingerprint')
    matches = [row for row in baseline['routes'] if row['id'] == RETIRED_EFFECTFUL]
    expected = [{'source': RECORDED, 'text': 'typed_plan::lower_effectful(', 'baseline_occurrences': 1}]
    if len(matches) != 1 or matches[0]['anchors'] != expected:
        errors.append('transition: retired route no longer matches reviewed baseline anchor')
    return errors



def validate_instrumentation(root: Path, baseline: dict, manifest: dict,
                             baseline_bytes: bytes | None, evolution: dict | None = None,
                             prior_probe: str | None = None) -> list[str]:
    """Permit only the reviewed test observation edit, not arbitrary gate drift."""
    errors = []
    if manifest.get('schema_version') != 1 or manifest.get('issue') != 515:
        errors.append('instrumentation: unsupported reviewed observation manifest')
    if baseline_bytes is None or digest(baseline_bytes) != manifest.get('baseline_file_sha256'):
        errors.append('instrumentation: frozen baseline file fingerprint changed or unavailable')
    elif json.loads(baseline_bytes) != baseline:
        errors.append('instrumentation: decoded baseline differs from frozen bytes')
    if manifest.get('baseline_selector') != baseline.get('main_selector'):
        errors.append('instrumentation: original selector is not bound to frozen baseline')
    current = manifest.get('reviewed_selector', {})
    if current.get('ordered_calls') != baseline.get('main_selector', {}).get('ordered_calls'):
        errors.append('instrumentation: selector calls or order changed')
    if not re.fullmatch('[0-9a-f]{64}', current.get('function_sha256', '')):
        errors.append('instrumentation: invalid reviewed selector fingerprint')
    fingerprints = manifest.get('reviewed_source_fingerprints', {})
    if set(fingerprints) != set(HOOK_SOURCES):
        errors.append('instrumentation: reviewed source set differs from observation owners')
        return errors
    if evolution is not None:
        fingerprints = {**fingerprints, **evolution['reviewed_source_fingerprints']}
    hook_ids = set()
    hook_pattern = r'(?:record_option!|record_guard!|trace::handled|trace::action|trace::guard)\(\s*"([^"]+)"'
    return_pattern = r'row\.return_dispatch\s*=\s*Some\("([^"]+)"\)'
    for relative, expected in fingerprints.items():
        try:
            data = (prior_probe.encode() if relative == observation_test.OWNER and prior_probe is not None
                    else (root / relative).read_bytes())
            actual = digest(data)
            hook_ids.update(re.findall(hook_pattern, data.decode()))
            hook_ids.update(re.findall(return_pattern, data.decode()))
        except OSError:
            errors.append(f'instrumentation: unreadable observation owner: {relative}')
            continue
        if actual != expected:
            errors.append(f'instrumentation: reviewed observation owner changed: {relative}')
    routes = (evolution if evolution is not None else manifest).get('instrumented_route_ids', [])
    if not routes or not all(isinstance(row, str) and row for row in routes):
        errors.append('instrumentation: observation IDs must be nonempty strings')
    elif len(set(routes)) != len(routes) or set(routes) != hook_ids:
        errors.append('instrumentation: IDs differ from source observation hooks')
    return errors


def verify(root: Path, baseline: dict, strict_fingerprints: bool = False,
           transition: dict | None = None, baseline_bytes: bytes | None = None,
           instrumentation: dict | None = None, jubjub_transition: dict | None = None,
           call_extension: dict | None = None, terminal_transition: dict | None = None,
           unit_actions_transition: dict | None = None,
           context_query_transition: dict | None = None,
           kernel_transition: dict | None = None,
           composite_transition: dict | None = None,
           receive_transition: dict | None = None,
           unit_domain_extension: dict | None = None,
           voting_transition: dict | None = None,
           phase_reset_transition: dict | None = None,
           immediate_send_transition: dict | None = None,
           zswap_transition: dict | None = None,
           membership_transition: dict | None = None,
           send_transition: dict | None = None,
           observation_test_extension: dict | None = None,
           merge_transition: dict | None = None,
           guarded_deposit_transition: dict | None = None,
           routing_test_extension: dict | None = None,
           reset_payout_transition: dict | None = None) -> dict:
    errors = validate_baseline(baseline)
    report = {'schema_version': 1, 'baseline_commit': baseline.get('baseline_commit'),
              'claim': 'source route inventory; not language or behavioral coverage',
              'route_count': len(baseline.get('routes', [])),
              'source_changes': [], 'route_checks': [], 'retired_routes': [], 'errors': errors}
    if transition is not None and not errors:
        errors.extend(validate_transition(baseline, transition, baseline_bytes))
    if errors:
        report['passed'] = False
        return report
    reviewed_jubjub = None
    if jubjub_transition is not None:
        review_errors = jubjub.validate(baseline, jubjub_transition, baseline_bytes, instrumentation)
        if transition is None:
            review_errors.append('Jubjub retirement: preceding effectful transition required')
        errors.extend(review_errors)
        if not review_errors:
            reviewed_jubjub = jubjub_transition
    reviewed_calls = None
    if call_extension is not None:
        call_errors = calls.validate(call_extension, transition, instrumentation, reviewed_jubjub)
        errors.extend(call_errors)
        if not call_errors:
            reviewed_calls = call_extension
            report['reviewed_call_extension'] = {
                'issue': call_extension['issue'],
                'classification': call_extension['classification'],
                'reviewed_sources': sorted(call_extension['reviewed_source_fingerprints']),
                'new_retired_routes': [],
                'claim': 'reviewed call-context source plumbing; no additional route retirement',
            }
    evolution = reviewed_calls or reviewed_jubjub
    reviewed_terminal = None
    if terminal_transition is not None:
        terminal_errors = terminal.validate(baseline, terminal_transition, baseline_bytes, reviewed_calls, instrumentation)
        errors.extend(terminal_errors)
        if not terminal_errors:
            reviewed_terminal = terminal_transition
            evolution = {**reviewed_calls, **reviewed_terminal,
                         'reviewed_source_fingerprints': {
                             **reviewed_calls['reviewed_source_fingerprints'],
                             **reviewed_terminal['reviewed_source_fingerprints']}}
    reviewed_unit = None
    if unit_actions_transition is not None:
        unit_errors = unit.validate(baseline, unit_actions_transition, baseline_bytes, reviewed_terminal)
        errors.extend(unit_errors)
        if not unit_errors:
            reviewed_unit = unit_actions_transition
            evolution = {**evolution, **reviewed_unit,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **reviewed_unit['reviewed_source_fingerprints']}}
    reviewed_context = None
    if context_query_transition is not None:
        context_errors = context.validate(baseline, context_query_transition, baseline_bytes, reviewed_unit)
        errors.extend(context_errors)
        if not context_errors:
            reviewed_context = context_query_transition
            evolution = {**evolution, **reviewed_context,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **reviewed_context['reviewed_source_fingerprints']}}
    reviewed_kernel = None
    if kernel_transition is not None:
        kernel_errors = kernel.validate(baseline, kernel_transition, baseline_bytes, reviewed_context, reviewed_calls)
        errors.extend(kernel_errors)
        if not kernel_errors:
            reviewed_kernel = kernel_transition
            evolution = {**evolution, **reviewed_kernel,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **reviewed_kernel['reviewed_source_fingerprints']}}
    reviewed_composite = None
    if composite_transition is not None:
        composite_errors = composite.validate(baseline, composite_transition, baseline_bytes, reviewed_kernel)
        errors.extend(composite_errors)
        if not composite_errors:
            reviewed_composite = composite_transition
            evolution = {**evolution, **reviewed_composite,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **reviewed_composite['reviewed_source_fingerprints']}}
    reviewed_receive = None
    if receive_transition is not None:
        receive_errors = receive.validate(baseline, receive_transition, baseline_bytes, reviewed_composite)
        errors.extend(receive_errors)
        if not receive_errors:
            reviewed_receive = receive_transition
            evolution = {**evolution, **reviewed_receive,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **reviewed_receive['reviewed_source_fingerprints']}}
    prior_routing_test = None
    if routing_test_extension is not None:
        try:
            # This exact single-owner snapshot supports ancestor reconstruction.
            # Final acceptance below still requires the complete reviewed chain.
            prior_routing_test = routing_test.validate(
                root, routing_test_extension, zswap_transition, membership_transition,
                send_transition, guarded_deposit_transition)
        except (OSError, UnicodeError, ValueError, KeyError) as error:
            errors.append(str(error))
    routing_snapshot = ({routing_test.OWNER: prior_routing_test}
                        if prior_routing_test is not None else {})
    proposed_reset_payout = None
    reset_payout_snapshot = None
    if reset_payout_transition is not None:
        reset_errors = reset_payout.validate(
            baseline, reset_payout_transition, baseline_bytes,
            guarded_deposit_transition, routing_test_extension)
        if not reset_errors:
            try:
                reset_payout_snapshot = reset_payout.predecessor_sources(root, reset_payout_transition)
            except (OSError, UnicodeError, ValueError, KeyError) as error:
                reset_errors.append('reset payout retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(reset_errors)
        if not reset_errors:
            proposed_reset_payout = reset_payout_transition
    proposed_guarded = None
    guarded_snapshot = None
    if guarded_deposit_transition is not None:
        guarded_errors = guarded.validate(baseline, guarded_deposit_transition, baseline_bytes, merge_transition)
        if not guarded_errors:
            try:
                if reset_payout_snapshot is not None:
                    guarded_sources = {owner: (root / owner).read_text() for owner in guarded.REVIEWED_OWNERS}
                    guarded_snapshot = guarded.predecessor_source_snapshot(
                        {**guarded_sources, **reset_payout_snapshot}, guarded_deposit_transition)
                else:
                    guarded_snapshot = guarded.predecessor_sources(root, guarded_deposit_transition)
            except (OSError, UnicodeError, ValueError) as error:
                guarded_errors.append('guarded deposit retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(guarded_errors)
        if not guarded_errors:
            proposed_guarded = guarded_deposit_transition
    proposed_merge = None
    merge_snapshot = None
    if merge_transition is not None:
        merge_errors = merge.validate(baseline, merge_transition, baseline_bytes,
                                      send_transition, observation_test_extension)
        if not merge_errors:
            try:
                if guarded_snapshot is not None:
                    merge_sources = {owner: (root / owner).read_text() for owner in merge.REVIEWED_OWNERS}
                    merge_snapshot = merge.predecessor_source_snapshot(
                        {**merge_sources, **guarded_snapshot}, merge_transition)
                else:
                    merge_snapshot = merge.predecessor_sources(root, merge_transition)
            except (OSError, UnicodeError, ValueError) as error:
                merge_errors.append('merge retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(merge_errors)
        if not merge_errors:
            proposed_merge = merge_transition
    reviewed_send = None
    send_snapshot = None
    if send_transition is not None:
        send_errors = send.validate(baseline, send_transition, baseline_bytes, membership_transition)
        if not send_errors:
            try:
                if merge_snapshot is not None:
                    send_sources = {owner: (root / owner).read_text() for owner in send.REVIEWED_OWNERS}
                    send_snapshot = send.predecessor_source_snapshot(
                        {**send_sources, **merge_snapshot, **routing_snapshot}, send_transition)
                else:
                    send_sources = {owner: (root / owner).read_text() for owner in send.REVIEWED_OWNERS}
                    send_snapshot = send.predecessor_source_snapshot(
                        {**send_sources, **routing_snapshot}, send_transition)
            except (OSError, UnicodeError, ValueError) as error:
                send_errors.append('send retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(send_errors)
        if not send_errors:
            reviewed_send = send_transition
    reviewed_membership = None
    membership_snapshot = None
    if membership_transition is not None:
        membership_errors = membership.validate(baseline, membership_transition, baseline_bytes,
                                                 zswap_transition)
        if not membership_errors:
            try:
                membership_snapshot = (membership.predecessor_source_snapshot(send_snapshot, membership_transition)
                                       if send_snapshot is not None else
                                       membership.predecessor_sources(root, membership_transition))
            except (OSError, UnicodeError, ValueError) as error:
                membership_errors.append('membership retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(membership_errors)
        if not membership_errors:
            reviewed_membership = membership_transition
    reviewed_zswap = None
    zswap_snapshot = None
    if zswap_transition is not None:
        zswap_errors = zswap.validate(baseline, zswap_transition, baseline_bytes,
                                     immediate_send_transition, phase_reset_transition,
                                     voting_transition, unit_domain_extension)
        if not zswap_errors:
            try:
                if membership_snapshot is not None:
                    zswap_sources = {owner: (root / owner).read_text() for owner in zswap.REVIEWED_OWNERS}
                    zswap_snapshot = zswap.predecessor_source_snapshot(
                        {**zswap_sources, **(send_snapshot or {}), **membership_snapshot}, zswap_transition)
                else:
                    zswap_snapshot = zswap.predecessor_sources(root, zswap_transition)
            except (OSError, UnicodeError, ValueError) as error:
                zswap_errors.append('Zswap retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(zswap_errors)
        if not zswap_errors:
            reviewed_zswap = zswap_transition
    reviewed_immediate = None
    immediate_snapshot = None
    if immediate_send_transition is not None:
        immediate_errors = immediate.validate(baseline, immediate_send_transition, baseline_bytes,
                                              phase_reset_transition, voting_transition, unit_domain_extension)
        if not immediate_errors:
            try:
                if zswap_snapshot is not None:
                    immediate_sources = {owner: (root / owner).read_text() for owner in immediate.REVIEWED_OWNERS}
                    if membership_snapshot is not None:
                        immediate_sources.update(membership_snapshot)
                    immediate_sources[SELECTOR] = zswap_snapshot[SELECTOR]
                    immediate_snapshot = immediate.predecessor_source_snapshot(immediate_sources, immediate_send_transition)
                else:
                    immediate_snapshot = immediate.predecessor_sources(root, immediate_send_transition)
            except (OSError, UnicodeError, ValueError) as error:
                immediate_errors.append('immediate retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(immediate_errors)
        if not immediate_errors:
            reviewed_immediate = immediate_send_transition
    reviewed_phase_reset = None
    phase_snapshot = None
    if phase_reset_transition is not None:
        phase_errors = phase_reset.validate(baseline, phase_reset_transition, baseline_bytes,
                                            voting_transition, unit_domain_extension)
        if not phase_errors:
            try:
                if immediate_snapshot is not None:
                    phase_sources = {owner: (root / owner).read_text() for owner in phase_reset.REVIEWED_OWNERS}
                    phase_snapshot = phase_reset.predecessor_source_snapshot(
                        {**phase_sources, **immediate_snapshot}, phase_reset_transition)
                else:
                    phase_snapshot = phase_reset.predecessor_sources(root, phase_reset_transition)
            except (OSError, UnicodeError, ValueError) as error:
                phase_errors.append('phase retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(phase_errors)
        if not phase_errors:
            reviewed_phase_reset = phase_reset_transition
    reviewed_voting = None
    voting_snapshot = None
    if voting_transition is not None:
        voting_errors = voting.validate(baseline, voting_transition, baseline_bytes,
                                        reviewed_receive, unit_domain_extension)
        if not voting_errors:
            try:
                voting_snapshot = voting.predecessor_selector(
                    phase_snapshot[SELECTOR] if phase_snapshot is not None else (root / SELECTOR).read_text(),
                    voting_transition)
            except (OSError, UnicodeError, ValueError) as error:
                voting_errors.append('voting retirement: predecessor reconstruction failed: ' + str(error))
        errors.extend(voting_errors)
        if not voting_errors:
            reviewed_voting = voting_transition
    if unit_domain_extension is not None:
        domain_errors = (unit_domain.validate_source(voting_snapshot, unit_domain_extension, reviewed_receive)
                         if voting_snapshot is not None else
                         unit_domain.validate(root, unit_domain_extension, reviewed_receive))
        errors.extend(domain_errors)
        if not domain_errors:
            evolution = {**evolution, **unit_domain_extension,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **unit_domain_extension['reviewed_source_fingerprints']}}
            report['reviewed_unit_domain_extension'] = {
                'issue': 540, 'new_retired_routes': [],
                'claim': 'ordinary Unit return admission; unchanged selector and route inventory',
            }
    # Authenticating a source snapshot does not authorize retirement unless its
    # historical voting transition also passed the full preceding review chain.
    if reviewed_voting is None:
        reviewed_phase_reset = None
    if reviewed_phase_reset is None:
        reviewed_immediate = None
    if reviewed_immediate is None:
        reviewed_zswap = None
    if reviewed_zswap is None:
        reviewed_membership = None
    if reviewed_membership is None:
        reviewed_send = None
    if reviewed_voting is not None:
        evolution = {**evolution, **reviewed_voting,
                     'reviewed_source_fingerprints': {
                         **evolution['reviewed_source_fingerprints'],
                         **reviewed_voting['reviewed_source_fingerprints']}}
    if reviewed_phase_reset is not None and reviewed_voting is not None:
        evolution = {**evolution, **reviewed_phase_reset,
                     'reviewed_source_fingerprints': {
                         **evolution['reviewed_source_fingerprints'],
                         **reviewed_phase_reset['reviewed_source_fingerprints']}}
    if reviewed_immediate is not None:
        evolution = {**evolution, **reviewed_immediate,
                     'reviewed_source_fingerprints': {
                         **evolution['reviewed_source_fingerprints'],
                         **reviewed_immediate['reviewed_source_fingerprints']}}
    if reviewed_zswap is not None:
        evolution = {**evolution, **reviewed_zswap,
                     'reviewed_source_fingerprints': {
                         **evolution['reviewed_source_fingerprints'],
                         **reviewed_zswap['reviewed_source_fingerprints']}}
    if reviewed_membership is not None:
        evolution = {**evolution, **reviewed_membership,
                     'reviewed_source_fingerprints': {
                         **evolution['reviewed_source_fingerprints'],
                         **reviewed_membership['reviewed_source_fingerprints']}}
    if reviewed_send is not None:
        evolution = {**evolution, **reviewed_send,
                     'reviewed_source_fingerprints': {
                         **evolution['reviewed_source_fingerprints'],
                         **reviewed_send['reviewed_source_fingerprints']}}
    prior_probe = None
    if observation_test_extension is not None:
        try:
            if reviewed_send is None:
                raise ValueError('observation test: complete reviewed send561 chain is required')
            prior_probe = observation_test.validate(root, observation_test_extension,
                                                    instrumentation, call_extension, send_transition)
            report['reviewed_observation_test_extension'] = {
                'issue': 564, 'owner': observation_test.OWNER, 'new_retired_routes': [],
                'claim': 'one authenticated test expectation update; unchanged instrumentation and routes'}
        except (OSError, UnicodeError, ValueError, KeyError) as error:
            errors.append(str(error))
    reviewed_merge = None
    if proposed_merge is not None:
        if reviewed_send is None or prior_probe is None:
            errors.append('merge retirement: complete send561 and observation564 authority required')
        else:
            reviewed_merge = proposed_merge
            evolution = {**evolution, **reviewed_merge,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **reviewed_merge['reviewed_source_fingerprints']}}
    reviewed_guarded = None
    if proposed_guarded is not None:
        if reviewed_merge is None:
            errors.append('guarded deposit retirement: complete merge565 authority required')
        else:
            reviewed_guarded = proposed_guarded
            evolution = {**evolution, **reviewed_guarded,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **reviewed_guarded['reviewed_source_fingerprints']}}
    if routing_test_extension is not None:
        if prior_routing_test is None or reviewed_guarded is None:
            errors.append('routing test: complete guarded566 authority and authenticated current test required')
        else:
            report['reviewed_routing_test_extension'] = {
                'issue': 568, 'owner': routing_test.OWNER, 'new_retired_routes': [],
                'claim': 'one authenticated test operand/import update; unchanged selectors and routes'}
            evolution = {**evolution, 'reviewed_source_fingerprints': {
                **evolution['reviewed_source_fingerprints'],
                routing_test.OWNER: routing_test.CURRENT_SHA}}
    reviewed_reset_payout = None
    if proposed_reset_payout is not None:
        if reviewed_guarded is None or 'reviewed_routing_test_extension' not in report:
            errors.append('reset payout retirement: complete guarded566 and routing568 authority required')
        else:
            reviewed_reset_payout = proposed_reset_payout
            evolution = {**evolution, **reviewed_reset_payout,
                         'reviewed_source_fingerprints': {
                             **evolution['reviewed_source_fingerprints'],
                             **reviewed_reset_payout['reviewed_source_fingerprints']}}
    if instrumentation is not None:
        errors.extend(validate_instrumentation(root, baseline, instrumentation, baseline_bytes, evolution, prior_probe))
        report['reviewed_instrumentation'] = {
            'issue': instrumentation.get('issue'),
            'instrumented_route_ids': (evolution or instrumentation).get('instrumented_route_ids', []),
            'claim': 'instrumented boundary IDs; execution of every boundary is not claimed',
        }
    sources = {}
    for relative, expected in baseline['source_fingerprints'].items():
        path = root / relative
        try:
            data = path.read_bytes()
            sources[relative] = data.decode()
        except (OSError, UnicodeError) as error:
            errors.append(f'{relative}: unreadable source ({type(error).__name__})')
            continue
        actual = digest(data)
        if actual != expected:
            report['source_changes'].append({'source': relative, 'baseline_sha256': expected,
                                             'current_sha256': actual})
            if strict_fingerprints:
                errors.append(f'{relative}: source fingerprint changed')
    for row in baseline['routes']:
        failures = []
        for anchor in row['anchors']:
            actual = sources.get(anchor['source'], '').count(anchor['text'])
            retired = (transition is not None and row['id'] == RETIRED_EFFECTFUL) or (reviewed_jubjub is not None and row['id'] == jubjub.ROUTE) or (reviewed_terminal is not None and row['id'] == terminal.ROUTE) or (reviewed_unit is not None and row['id'] == unit.ROUTE) or (reviewed_context is not None and row['id'] == context.ROUTE) or (reviewed_kernel is not None and row['id'] == kernel.ROUTE) or (reviewed_composite is not None and row['id'] == composite.ROUTE) or (reviewed_receive is not None and row['id'] == receive.ROUTE) or (reviewed_voting is not None and row['id'] == voting.ROUTE) or (reviewed_phase_reset is not None and row['id'] == phase_reset.ROUTE) or (reviewed_immediate is not None and row['id'] == immediate.ROUTE) or (reviewed_zswap is not None and row['id'] == zswap.ROUTE) or (reviewed_membership is not None and row['id'] == membership.ROUTE) or (reviewed_send is not None and row['id'] == send.ROUTE) or (reviewed_merge is not None and row['id'] == merge.ROUTE) or (reviewed_guarded is not None and row['id'] == guarded.ROUTE) or (reviewed_reset_payout is not None and row['id'] == reset_payout.ROUTE)
            expected = 0 if retired else anchor['baseline_occurrences']
            if actual != expected:
                failures.append({'source': anchor['source'], 'anchor': anchor['text'],
                                 'expected': expected, 'actual': actual})
        report['route_checks'].append({'id': row['id'], 'passed': not failures, 'failures': failures})
        if failures:
            errors.append(f"{row['id']}: source anchor count changed")
    if transition is not None:
        try:
            oracle = (root / ORACLE).read_text()
            retirement_errors = retired_effectful_checks(sources, oracle, transition)
        except (OSError, UnicodeError) as error:
            retirement_errors = [f'retirement: unreadable test oracle ({type(error).__name__})']
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': RETIRED_EFFECTFUL,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == RETIRED_EFFECTFUL),
            'evidence': transition,
        })
    if reviewed_jubjub is not None:
        retirement_errors = jubjub.check_sources(root, sources, reviewed_jubjub)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': jubjub.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == jubjub.ROUTE),
            'evidence': reviewed_jubjub,
        })
    if reviewed_terminal is not None:
        retirement_errors = terminal.check_sources(root, sources, reviewed_terminal)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': terminal.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == terminal.ROUTE),
            'evidence': reviewed_terminal,
        })
    if reviewed_unit is not None:
        retirement_errors = unit.check_sources(root, sources, reviewed_unit,
                                                reviewed_receive['relocated_shared_anchors'] if reviewed_receive else None)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': unit.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == unit.ROUTE),
            'evidence': reviewed_unit,
        })
    if reviewed_context is not None:
        retirement_errors = context.check_sources(root, sources, reviewed_context)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': context.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == context.ROUTE),
            'evidence': reviewed_context,
        })
    if reviewed_kernel is not None:
        retirement_errors = kernel.check_sources(root, sources, reviewed_kernel)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': kernel.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == kernel.ROUTE),
            'evidence': reviewed_kernel,
        })
    if reviewed_composite is not None:
        retirement_errors = composite.check_sources(root, sources, reviewed_composite)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': composite.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == composite.ROUTE),
            'evidence': reviewed_composite,
        })
    if reviewed_receive is not None:
        receive_sources = ({**sources, merge.MERGE: merge_snapshot[merge.MERGE]}
                           if reviewed_merge is not None else sources)
        if reviewed_guarded is not None:
            receive_sources = {**receive_sources, guarded.GUARDED: guarded_snapshot[guarded.GUARDED]}
        if immediate_snapshot is not None:
            receive_sources = {**receive_sources, immediate.IMMEDIATE: immediate_snapshot[immediate.IMMEDIATE]}
        retirement_errors = receive.check_sources(root, receive_sources, reviewed_receive)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': receive.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == receive.ROUTE),
            'evidence': reviewed_receive,
        })
    if reviewed_voting is not None:
        retirement_errors = voting.check_sources(root, {**sources, **(phase_snapshot or {})}, reviewed_voting)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': voting.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == voting.ROUTE),
            'evidence': reviewed_voting,
        })
    if reviewed_phase_reset is not None:
        retirement_errors = phase_reset.check_sources(
            root, {**sources, **(immediate_snapshot or {})}, reviewed_phase_reset,
            reset_payout_snapshot[reset_payout.RESET] if reviewed_reset_payout is not None else None)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': phase_reset.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == phase_reset.ROUTE),
            'evidence': reviewed_phase_reset,
        })
    if reviewed_immediate is not None:
        # profile_attempt predates the frozen baseline source list, but #550
        # authenticates it as a fourth owner. Supply its actual bytes explicitly.
        try:
            sources[immediate.ATTEMPT] = (root / immediate.ATTEMPT).read_text()
        except (OSError, UnicodeError) as error:
            errors.append('immediate retirement: unreadable attempt owner: ' + str(error))
        immediate_sources = ({**sources, **(send_snapshot or {}), **(membership_snapshot or {}), SELECTOR: zswap_snapshot[SELECTOR]}
                             if zswap_snapshot is not None else sources)
        retirement_errors = immediate.check_sources(
            root, immediate_sources, reviewed_immediate,
            merge_snapshot[merge.MERGE] if reviewed_merge is not None else None,
            guarded_snapshot[guarded.GUARDED] if reviewed_guarded is not None else None,
            reset_payout_snapshot[reset_payout.RESET] if reviewed_reset_payout is not None else None)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': immediate.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == immediate.ROUTE),
            'evidence': reviewed_immediate,
        })
    if reviewed_zswap is not None:
        retirement_errors = zswap.check_sources(root, {**sources, **(send_snapshot or {}), **(membership_snapshot or {})}, reviewed_zswap)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': zswap.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == zswap.ROUTE),
            'evidence': reviewed_zswap,
        })
    if reviewed_membership is not None:
        # The earlier routing test module is itself an authenticated adapted owner.
        try:
            sources[membership.ZSWAP_TEST] = (root / membership.ZSWAP_TEST).read_text()
        except (OSError, UnicodeError) as error:
            errors.append('membership retirement: unreadable adapted routing tests: ' + str(error))
        retirement_errors = membership.check_sources(root, {**sources, **(send_snapshot or {}), **routing_snapshot}, reviewed_membership)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': membership.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == membership.ROUTE),
            'evidence': reviewed_membership,
        })
    if reviewed_send is not None:
        retirement_errors = send.check_sources(root, {**sources, **(merge_snapshot or {}), **routing_snapshot}, reviewed_send)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': send.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == send.ROUTE),
            'evidence': reviewed_send,
        })
    if reviewed_merge is not None:
        try:
            sources[merge.VOTING_TEST] = (root / merge.VOTING_TEST).read_text()
        except (OSError, UnicodeError) as error:
            errors.append('merge retirement: unreadable adapted voting test: ' + str(error))
        retirement_errors = merge.check_sources(
            root, {**sources, **(guarded_snapshot or {})}, reviewed_merge,
            guarded_snapshot[guarded.VOTING_TEST] if reviewed_guarded is not None else None)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': merge.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == merge.ROUTE),
            'evidence': reviewed_merge,
        })
    if reviewed_guarded is not None:
        try:
            sources[guarded.VOTING_TEST] = (root / guarded.VOTING_TEST).read_text()
        except (OSError, UnicodeError) as error:
            errors.append('guarded deposit retirement: unreadable adapted voting test: ' + str(error))
        retirement_errors = guarded.check_sources(
            root, {**sources, **(reset_payout_snapshot or {})}, reviewed_guarded,
            reset_payout_snapshot[reset_payout.VOTING_TEST] if reviewed_reset_payout is not None else None,
            reset_payout_snapshot[reset_payout.GUARDED_TEST] if reviewed_reset_payout is not None else None)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': guarded.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == guarded.ROUTE),
            'evidence': reviewed_guarded,
        })
    if reviewed_reset_payout is not None:
        for owner in reset_payout.REVIEWED_OWNERS:
            try:
                sources[owner] = (root / owner).read_text()
            except (OSError, UnicodeError) as error:
                errors.append('reset payout retirement: unreadable current owner: ' + str(error))
        retirement_errors = reset_payout.check_sources(root, sources, reviewed_reset_payout)
        errors.extend(retirement_errors)
        report['retired_routes'].append({
            'id': reset_payout.ROUTE,
            'status': 'retired-from-production-test-oracle-retained',
            'passed': not retirement_errors and not next(
                row['failures'] for row in report['route_checks'] if row['id'] == reset_payout.ROUTE),
            'evidence': reviewed_reset_payout,
        })
    if SELECTOR in sources:
        try:
            actual = selector_signature(sources[SELECTOR])
            report['current_selector'] = actual
            expected = (evolution or instrumentation).get('reviewed_selector', {}) if instrumentation is not None else baseline['main_selector']
            if actual != expected:
                errors.append('main selector changed: review ordered calls and complete function fingerprint')
        except ValueError as error:
            errors.append(str(error))
    report['passed'] = not errors
    return report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--baseline', type=Path, default=BASELINE)
    parser.add_argument('--output', type=Path, help='current verification JSON; must differ from baseline')
    parser.add_argument('--strict-fingerprints', action='store_true')
    parser.add_argument('--transition', type=Path, help='explicit reviewed retirement JSON; baseline stays frozen')
    parser.add_argument('--instrumentation', type=Path, help='explicit reviewed test-hook manifest; original selector stays frozen')
    parser.add_argument('--jubjub-transition', type=Path, help='explicit #517 retirement following #511 and #515 evidence')
    parser.add_argument('--call-extension', type=Path, help='explicit #519 call-context plumbing review; no new retirement')
    parser.add_argument('--terminal-transition', type=Path, help='explicit #521 terminal retirement following #519 evidence')
    parser.add_argument('--unit-actions-transition', type=Path, help='explicit #525 unit-actions retirement following #521 evidence')
    parser.add_argument('--context-query-transition', type=Path, help='explicit #526 context-query retirement following #525 evidence')
    parser.add_argument('--kernel-transition', type=Path, help='explicit #527 kernel retirement following #526 evidence')
    parser.add_argument('--composite-transition', type=Path, help='explicit #529 whole composite retirement following #527 evidence')
    parser.add_argument('--receive-transition', type=Path, help='explicit #536 whole receive retirement following #529 evidence')
    parser.add_argument('--unit-domain-extension', type=Path, help='explicit #540 ordinary Unit return review; no retirement')
    parser.add_argument('--voting-transition', type=Path, help='explicit #547 voting retirement following #540 evidence')
    parser.add_argument('--phase-reset-transition', type=Path, help='explicit #549 retirement into existing kernel domain after #547')
    parser.add_argument('--immediate-send-transition', type=Path, help='explicit #550 retirement into existing composite domain after #549')
    parser.add_argument('--zswap-transition', type=Path, help='explicit #553 final-priority Zswap retirement after #550')
    parser.add_argument('--membership-transition', type=Path, help='explicit #560 first-priority membership retirement after #553')
    parser.add_argument('--send-transition', type=Path, help='explicit #561 retirement into existing composite domain after #560')
    parser.add_argument('--observation-test-extension', type=Path, help='explicit #564 test-only expectation adaptation; no retirement')
    parser.add_argument('--merge-transition', type=Path, help='explicit #565 merge subsumption after #561 and #564')
    parser.add_argument('--guarded-deposit-transition', type=Path, help='explicit #566 guarded-deposit checked admission after #565')
    parser.add_argument('--routing-test-extension', type=Path, help='explicit #568 test-only negative operand adaptation; no retirement')
    parser.add_argument('--reset-payout-transition', type=Path, help='explicit #569 reset payout subsumption after #566 and #568')
    args = parser.parse_args(argv)
    protected = [args.baseline] + ([args.transition] if args.transition else []) + ([args.instrumentation] if args.instrumentation else []) + ([args.jubjub_transition] if args.jubjub_transition else []) + ([args.call_extension] if args.call_extension else []) + ([args.terminal_transition] if args.terminal_transition else []) + ([args.unit_actions_transition] if args.unit_actions_transition else []) + ([args.context_query_transition] if args.context_query_transition else []) + ([args.kernel_transition] if args.kernel_transition else []) + ([args.composite_transition] if args.composite_transition else []) + ([args.receive_transition] if args.receive_transition else [])
    protected += [args.unit_domain_extension] if args.unit_domain_extension else []
    protected += [args.voting_transition] if args.voting_transition else []
    protected += [args.phase_reset_transition] if args.phase_reset_transition else []
    protected += [args.immediate_send_transition] if args.immediate_send_transition else []
    protected += [args.zswap_transition] if args.zswap_transition else []
    protected += [args.membership_transition] if args.membership_transition else []
    protected += [args.send_transition] if args.send_transition else []
    protected += [args.observation_test_extension] if args.observation_test_extension else []
    protected += [args.merge_transition] if args.merge_transition else []
    protected += [args.guarded_deposit_transition] if args.guarded_deposit_transition else []
    protected += [args.routing_test_extension] if args.routing_test_extension else []
    protected += [args.reset_payout_transition] if args.reset_payout_transition else []
    if args.output and any(args.output.resolve() == path.resolve() for path in protected):
        parser.error('--output cannot overwrite baseline, transition, instrumentation, or call-extension evidence')
    try:
        baseline_bytes = args.baseline.read_bytes()
        baseline = json.loads(baseline_bytes)
        transition = json.loads(args.transition.read_text()) if args.transition else None
        instrumentation = json.loads(args.instrumentation.read_text()) if args.instrumentation else None
        jubjub_transition = json.loads(args.jubjub_transition.read_text()) if args.jubjub_transition else None
        call_extension = json.loads(args.call_extension.read_text()) if args.call_extension else None
        terminal_transition = json.loads(args.terminal_transition.read_text()) if args.terminal_transition else None
        unit_actions_transition = json.loads(args.unit_actions_transition.read_text()) if args.unit_actions_transition else None
        context_query_transition = json.loads(args.context_query_transition.read_text()) if args.context_query_transition else None
        kernel_transition = json.loads(args.kernel_transition.read_text()) if args.kernel_transition else None
        composite_transition = json.loads(args.composite_transition.read_text()) if args.composite_transition else None
        receive_transition = json.loads(args.receive_transition.read_text()) if args.receive_transition else None
        unit_domain_extension = json.loads(args.unit_domain_extension.read_text()) if args.unit_domain_extension else None
        voting_transition = json.loads(args.voting_transition.read_text()) if args.voting_transition else None
        phase_reset_transition = json.loads(args.phase_reset_transition.read_text()) if args.phase_reset_transition else None
        immediate_send_transition = json.loads(args.immediate_send_transition.read_text()) if args.immediate_send_transition else None
        zswap_transition = json.loads(args.zswap_transition.read_text()) if args.zswap_transition else None
        membership_transition = json.loads(args.membership_transition.read_text()) if args.membership_transition else None
        send_transition = json.loads(args.send_transition.read_text()) if args.send_transition else None
        observation_test_extension = json.loads(args.observation_test_extension.read_text()) if args.observation_test_extension else None
        merge_transition = json.loads(args.merge_transition.read_text()) if args.merge_transition else None
        guarded_deposit_transition = json.loads(args.guarded_deposit_transition.read_text()) if args.guarded_deposit_transition else None
        routing_test_extension = json.loads(args.routing_test_extension.read_text()) if args.routing_test_extension else None
        reset_payout_transition = json.loads(args.reset_payout_transition.read_text()) if args.reset_payout_transition else None
        report = verify(args.root, baseline, args.strict_fingerprints, transition, baseline_bytes, instrumentation, jubjub_transition, call_extension, terminal_transition, unit_actions_transition, context_query_transition, kernel_transition, composite_transition, receive_transition, unit_domain_extension, voting_transition, phase_reset_transition, immediate_send_transition, zswap_transition, membership_transition, send_transition, observation_test_extension, merge_transition, guarded_deposit_transition, routing_test_extension, reset_payout_transition)
    except (OSError, ValueError, TypeError, KeyError) as error:
        parser.exit(2, f'inventory error: {error}\n')
    rendered = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    else:
        print(rendered, end='')
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
