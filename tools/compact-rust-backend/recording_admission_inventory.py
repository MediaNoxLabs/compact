#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Verify the frozen EA-00 admission route inventory; never rewrite its baseline.

This is a source-change gate, not a Rust parser or language coverage measure.
Exact anchor counts and the main selector's ordered calls/function digest are
checked. Other source drift is reported, or rejected by --strict-fingerprints.
Human review must classify new/changed routes before updating a future baseline.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
BASELINE = Path(__file__).with_suffix('.baseline.json')
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
CALL = re.compile(r'\b(?:typed_plan::lower\w*|kernel_plan::lower|zswap_plan::lower)\s*\(')
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


def verify(root: Path, baseline: dict, strict_fingerprints: bool = False) -> dict:
    errors = validate_baseline(baseline)
    report = {'schema_version': 1, 'baseline_commit': baseline.get('baseline_commit'),
              'claim': 'source route inventory; not language or behavioral coverage',
              'route_count': len(baseline.get('routes', [])),
              'source_changes': [], 'route_checks': [], 'errors': errors}
    if errors:
        report['passed'] = False
        return report
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
            if actual != anchor['baseline_occurrences']:
                failures.append({'source': anchor['source'], 'anchor': anchor['text'],
                                 'expected': anchor['baseline_occurrences'], 'actual': actual})
        report['route_checks'].append({'id': row['id'], 'passed': not failures, 'failures': failures})
        if failures:
            errors.append(f"{row['id']}: source anchor count changed")
    if SELECTOR in sources:
        try:
            actual = selector_signature(sources[SELECTOR])
            report['current_selector'] = actual
            if actual != baseline['main_selector']:
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
    args = parser.parse_args(argv)
    if args.output and args.output.resolve() == args.baseline.resolve():
        parser.error('--output cannot overwrite the frozen baseline')
    try:
        baseline = json.loads(args.baseline.read_text())
        report = verify(args.root, baseline, args.strict_fingerprints)
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
