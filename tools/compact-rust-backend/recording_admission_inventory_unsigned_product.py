# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Retire guarded unsigned-product shape admission with exact signed-source authority."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

ROUTE = 'guard.guarded-unsigned-product'
REPLACEMENTS = ('algebra.unit-composition',)
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
GUARDED = 'tools/compact-rust-backend/src/recorded/legacy/guarded_pure.rs'
RECORDED = 'tools/compact-rust-backend/src/recorded.rs'
MANIFEST_SHA = 'd1c4ff0f816b38fc15a8acd551fb44d5ee06c5c6ae95c12472c59fd51de495e2'


def predecessor_sources(root, transition):
    if canonical(transition) != MANIFEST_SHA:
        raise ValueError('product591: unreviewed retirement manifest')
    sources = {}
    for owner, row in transition['owners'].items():
        text = (root / owner).read_text()
        if digest(text.encode()) != row['current_sha256']:
            raise ValueError('product591: live complete owner changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or text.count(edit['current']) != 1:
                raise ValueError('product591: nonunique predecessor reconstruction: ' + owner)
            text = text.replace(edit['current'], edit['previous'], 1)
        if digest(text.encode()) != row['previous_sha256']:
            raise ValueError('product591: signed predecessor reconstruction failed: ' + owner)
        # Signed checkpoints added only reviewed test changes after earlier inventories.
        # Authenticate that exact additional stage rather than changing old pins.
        for edit in row.get('historical_reconstruction', []):
            if not edit['current'] or text.count(edit['current']) != 1:
                raise ValueError('product591: nonunique historical test-only reconstruction')
            text = text.replace(edit['current'], edit['previous'], 1)
        if digest(text.encode()) != row.get('historical_sha256', row['previous_sha256']):
            raise ValueError('product591: immutable historical inventory owner differs: ' + owner)
        sources[owner] = text
    for owner, expected in transition['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('product591: actual new/shared dependency changed: ' + owner)
    return sources


class _Checkpoint:
    """Only whole-source-authenticated predecessors; every other path remains live."""
    def __init__(self, root, sources, transition):
        if canonical(transition) != MANIFEST_SHA or set(sources) != set(transition['owners']):
            raise ValueError('product591: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != transition['owners'][owner].get('historical_sha256', transition['owners'][owner]['previous_sha256']):
                raise ValueError('product591: unauthenticated predecessor owner: ' + owner)
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, transition):
    return _Checkpoint(root, predecessor_sources(root, transition), transition)


def verify_transition(root, baseline, transition, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source route inventory; not language or behavioral coverage',
              'route_count': len(baseline.get('routes', [])), 'route_checks': [],
              'retired_routes': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        prior_root = checkpoint(root, transition)
        prior = historical_verify(prior_root)
        report['historical_predecessor'] = prior
        if not prior['passed']:
            report['errors'].append('product591: immutable historical predecessor checks failed')
            report['errors'].extend(prior['errors'])
            return report
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != transition['current_selector']:
            raise ValueError('product591: actual selector priority or fingerprint changed')
        if prior['current_selector'] != transition['previous_selector']:
            raise ValueError('product591: predecessor selector identity differs')
        if prior['current_selector']['ordered_calls'] or actual_selector['ordered_calls']:
            raise ValueError('product591: named-selector retirement must remain complete')
        text = (root / RECORDED).read_text()
        if 'closed_guarded_unsigned_product_steps' in text or ROUTE in text:
            raise ValueError('product591: retired unsigned-product guard remains in production')
        guarded = (root / GUARDED).read_text()
        marker = '#[cfg(test)]\npub(in crate::recorded) fn closed_guarded_unsigned_product_steps('
        if guarded.count(marker) != 1:
            raise ValueError('product591: historical unsigned-product oracle must remain test-only')
        if text.count('"guard.guarded-struct-pure"') != 1 or text.count('closed_guarded_struct_pure_steps(circuit, &parameters, pure_circuits, circuits)?') != 1:
            raise ValueError('product591: guarded-struct sibling changed')
        # Whole owner authority preserves eager sibling placement and continuation handling.
        report['current_selector'] = actual_selector
        report['route_checks'] = copy.deepcopy(prior['route_checks'])
        for row in baseline['routes']:
            actual_row = next(x for x in report['route_checks'] if x['id'] == row['id'])
            for anchor in row['anchors']:
                # Existing retirements remain independently authenticated by the chain.
                expected = 0 if row['id'] in [x['id'] for x in prior['retired_routes']] + [ROUTE] else anchor['baseline_occurrences']
                count = (root / anchor['source']).read_text().count(anchor['text'])
                if count != expected:
                    actual_row['failures'].append({'source':anchor['source'],'anchor':anchor['text'],'expected':expected,'actual':count})
            actual_row['passed'] = not actual_row['failures']
        if any(not x['passed'] for x in report['route_checks']):
            raise ValueError('product591: actual source anchor drift outside the reviewed retirement')
        report['retired_routes'] = copy.deepcopy(prior['retired_routes'])
        report['retired_routes'].append({'id':ROUTE,'status':'retired-from-production-test-oracle-retained','passed':True,'evidence':transition})
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        if ids.count(ROUTE) != 1 or any(ids.count(route) != 1 for route in REPLACEMENTS):
            raise ValueError('product591: unexpected prior instrumentation identity')
        report['reviewed_instrumentation'] = {'issue':591,'instrumented_route_ids':[x for x in ids if x != ROUTE], 'claim':'reviewed source instrumentation; execution of every boundary is not claimed'}
        for owner, expected in baseline['source_fingerprints'].items():
            actual = digest((root / owner).read_bytes())
            if actual != expected:
                report['source_changes'].append({'source':owner,'baseline_sha256':expected,'current_sha256':actual})
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError) as error:
        report['errors'].append(str(error))
    return report
