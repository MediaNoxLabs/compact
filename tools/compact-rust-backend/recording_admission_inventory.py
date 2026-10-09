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
                             baseline_bytes: bytes | None, evolution: dict | None = None) -> list[str]:
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
            data = (root / relative).read_bytes()
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
           kernel_transition: dict | None = None) -> dict:
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
    if instrumentation is not None:
        errors.extend(validate_instrumentation(root, baseline, instrumentation, baseline_bytes, evolution))
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
            retired = (transition is not None and row['id'] == RETIRED_EFFECTFUL) or (reviewed_jubjub is not None and row['id'] == jubjub.ROUTE) or (reviewed_terminal is not None and row['id'] == terminal.ROUTE) or (reviewed_unit is not None and row['id'] == unit.ROUTE) or (reviewed_context is not None and row['id'] == context.ROUTE) or (reviewed_kernel is not None and row['id'] == kernel.ROUTE)
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
        retirement_errors = unit.check_sources(root, sources, reviewed_unit)
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
    args = parser.parse_args(argv)
    protected = [args.baseline] + ([args.transition] if args.transition else []) + ([args.instrumentation] if args.instrumentation else []) + ([args.jubjub_transition] if args.jubjub_transition else []) + ([args.call_extension] if args.call_extension else []) + ([args.terminal_transition] if args.terminal_transition else []) + ([args.unit_actions_transition] if args.unit_actions_transition else []) + ([args.context_query_transition] if args.context_query_transition else []) + ([args.kernel_transition] if args.kernel_transition else [])
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
        report = verify(args.root, baseline, args.strict_fingerprints, transition, baseline_bytes, instrumentation, jubjub_transition, call_extension, terminal_transition, unit_actions_transition, context_query_transition, kernel_transition)
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
