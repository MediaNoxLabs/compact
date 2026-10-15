# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Retire proved opaque Set sequence permit admission with exact signed-source authority."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

ROUTE = 'guard.proved-opaque-set-sequence'
RETIRED = (ROUTE,)
REPLACEMENTS = ('algebra.membership', 'algebra.unit-composition')
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
RECORDED = 'tools/compact-rust-backend/src/recorded.rs'
MANIFEST_SHA = 'a892e66fda3f7e440dd736598c6916f9e85eca3fd5e0525e2386e324a73a2ec4'


def predecessor_sources(root, transition):
    if canonical(transition) != MANIFEST_SHA:
        raise ValueError('sequence605: unreviewed retirement manifest')
    sources = {}
    for owner, row in transition['owners'].items():
        text = (root / owner).read_text()
        if digest(text.encode()) != row['current_sha256']:
            raise ValueError('sequence605: live complete owner changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or text.count(edit['current']) != 1:
                raise ValueError('sequence605: nonunique predecessor reconstruction: ' + owner)
            text = text.replace(edit['current'], edit['previous'], 1)
        if digest(text.encode()) != row['previous_sha256']:
            raise ValueError('sequence605: signed predecessor reconstruction failed: ' + owner)
        # Signed checkpoints added only reviewed test changes after earlier inventories.
        # Authenticate that exact additional stage rather than changing old pins.
        for edit in row.get('historical_reconstruction', []):
            if not edit['current'] or text.count(edit['current']) != 1:
                raise ValueError('sequence605: nonunique historical test-only reconstruction')
            text = text.replace(edit['current'], edit['previous'], 1)
        if digest(text.encode()) != row.get('historical_sha256', row['previous_sha256']):
            raise ValueError('sequence605: immutable historical inventory owner differs: ' + owner)
        sources[owner] = text
    for owner, expected in transition['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('sequence605: actual new/shared dependency changed: ' + owner)
    return sources


class _Checkpoint:
    """Only whole-source-authenticated predecessors; every other path remains live."""
    def __init__(self, root, sources, transition):
        if canonical(transition) != MANIFEST_SHA or set(sources) != set(transition['owners']):
            raise ValueError('sequence605: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != transition['owners'][owner].get('historical_sha256', transition['owners'][owner]['previous_sha256']):
                raise ValueError('sequence605: unauthenticated predecessor owner: ' + owner)
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
            report['errors'].append('sequence605: immutable historical predecessor checks failed')
            report['errors'].extend(prior['errors'])
            return report
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != transition['current_selector']:
            raise ValueError('sequence605: actual selector priority or fingerprint changed')
        if prior['current_selector'] != transition['previous_selector']:
            raise ValueError('sequence605: predecessor selector identity differs')
        if prior['current_selector']['ordered_calls'] or actual_selector['ordered_calls']:
            raise ValueError('sequence605: named-selector retirement must remain complete')
        text = (root / RECORDED).read_text()
        if ROUTE in text:
            raise ValueError('sequence605: retired proved Set sequence guard remains in production')
        if text.count('#[cfg(test)]\nfn proved_opaque_set_sequence(') != 1:
            raise ValueError('sequence605: historical proved Set oracle must remain test-only')
        if len(prior['retired_routes']) != 33 or prior['route_count'] != 84:
            raise ValueError('sequence605: predecessor inventory counts changed')
        # Whole owner authority preserves generic dispatch and the surviving guard priorities.
        report['current_selector'] = actual_selector
        report['route_checks'] = copy.deepcopy(prior['route_checks'])
        for row in baseline['routes']:
            actual_row = next(x for x in report['route_checks'] if x['id'] == row['id'])
            for anchor in row['anchors']:
                if row['id'] in (ROUTE, 'guard.closed-opaque-map-operation', 'guard.closed-opaque-set-operation', 'guard.closed-opaque-asset-removal'):
                    # This route's baseline anchor is its definition, retained
                    # only under cfg(test). The eager flag and trace are checked
                    # absent above; whole-owner reconstruction authenticates it.
                    oracle = {'guard.proved-opaque-set-sequence':'proved_opaque_set_sequence', 'guard.closed-opaque-map-operation':'closed_opaque_map_operation', 'guard.closed-opaque-set-operation':'closed_opaque_set_operation', 'guard.closed-opaque-asset-removal':'closed_opaque_asset_removal'}[row['id']]
                    if anchor != {'source': RECORDED, 'text': 'fn ' + oracle + '(', 'baseline_occurrences': 1}:
                        raise ValueError('sequence605: unexpected retired oracle anchor')
                    if text.count(anchor['text']) != 1:
                        raise ValueError('sequence605: retired test oracle identity changed')
                    continue
                adapted = [x for x in transition['anchor_adaptations'] if x['route'] == row['id'] and x['previous'] == anchor]
                if len(adapted) > 1:
                    raise ValueError('sequence605: ambiguous reviewed anchor adaptation')
                retired = row['id'] in [x['id'] for x in prior['retired_routes']] + list(RETIRED)
                if adapted and retired:
                    raise ValueError('sequence605: retired route cannot be relabeled as an anchor adaptation')
                current = adapted[0]['current'] if adapted else [{
                    'source': anchor['source'], 'text': anchor['text'],
                    'occurrences': 0 if retired else anchor['baseline_occurrences']}]
                for check in current:
                    count = (root / check['source']).read_text().count(check['text'])
                    if count != check['occurrences']:
                        actual_row['failures'].append({'source':check['source'],'anchor':check['text'],'expected':check['occurrences'],'actual':count})
            actual_row['passed'] = not actual_row['failures']
        if any(not x['passed'] for x in report['route_checks']):
            raise ValueError('sequence605: actual source anchor drift outside the reviewed retirement')
        report['reviewed_anchor_adaptations'] = copy.deepcopy(transition['anchor_adaptations'])
        report['retired_routes'] = copy.deepcopy(prior['retired_routes'])
        for retired_route in RETIRED:
            report['retired_routes'].append({'id':retired_route,'status':'retired-from-production-test-oracle-retained','passed':True,'evidence':transition})
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        if ids.count(ROUTE) != 1 or any(ids.count(route) != 1 for route in REPLACEMENTS):
            raise ValueError('sequence605: unexpected prior instrumentation identity')
        report['reviewed_instrumentation'] = {'issue':605,'instrumented_route_ids':[x for x in ids if x != ROUTE], 'claim':'reviewed source instrumentation; execution of every boundary is not claimed'}
        for owner, expected in baseline['source_fingerprints'].items():
            actual = digest((root / owner).read_bytes())
            if actual != expected:
                report['source_changes'].append({'source':owner,'baseline_sha256':expected,'current_sha256':actual})
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError) as error:
        report['errors'].append(str(error))
    return report
