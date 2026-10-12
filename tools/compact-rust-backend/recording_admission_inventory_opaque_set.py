# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Retire guarded opaque Set shape admission with exact signed-source authority."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

ROUTE = 'guard.opaque-set-mutation'
REPLACEMENTS = ('algebra.unit-composition',)
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
GUARDED = 'tools/compact-rust-backend/src/recorded/legacy/opaque_set.rs'
LEGACY = 'tools/compact-rust-backend/src/recorded/legacy.rs'
RECORDED = 'tools/compact-rust-backend/src/recorded.rs'
MANIFEST_SHA = '24fed79911df615c26334b458f47809bc35fcef49600b4c55010f894f5852327'


def predecessor_sources(root, transition):
    if canonical(transition) != MANIFEST_SHA:
        raise ValueError('set592: unreviewed retirement manifest')
    sources = {}
    for owner, row in transition['owners'].items():
        text = (root / owner).read_text()
        if digest(text.encode()) != row['current_sha256']:
            raise ValueError('set592: live complete owner changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or text.count(edit['current']) != 1:
                raise ValueError('set592: nonunique predecessor reconstruction: ' + owner)
            text = text.replace(edit['current'], edit['previous'], 1)
        if digest(text.encode()) != row['previous_sha256']:
            raise ValueError('set592: signed predecessor reconstruction failed: ' + owner)
        # Signed checkpoints added only reviewed test changes after earlier inventories.
        # Authenticate that exact additional stage rather than changing old pins.
        for edit in row.get('historical_reconstruction', []):
            if not edit['current'] or text.count(edit['current']) != 1:
                raise ValueError('set592: nonunique historical test-only reconstruction')
            text = text.replace(edit['current'], edit['previous'], 1)
        if digest(text.encode()) != row.get('historical_sha256', row['previous_sha256']):
            raise ValueError('set592: immutable historical inventory owner differs: ' + owner)
        sources[owner] = text
    for owner, expected in transition['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('set592: actual new/shared dependency changed: ' + owner)
    return sources


class _Checkpoint:
    """Only whole-source-authenticated predecessors; every other path remains live."""
    def __init__(self, root, sources, transition):
        if canonical(transition) != MANIFEST_SHA or set(sources) != set(transition['owners']):
            raise ValueError('set592: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != transition['owners'][owner].get('historical_sha256', transition['owners'][owner]['previous_sha256']):
                raise ValueError('set592: unauthenticated predecessor owner: ' + owner)
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
            report['errors'].append('set592: immutable historical predecessor checks failed')
            report['errors'].extend(prior['errors'])
            return report
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != transition['current_selector']:
            raise ValueError('set592: actual selector priority or fingerprint changed')
        if prior['current_selector'] != transition['previous_selector']:
            raise ValueError('set592: predecessor selector identity differs')
        if prior['current_selector']['ordered_calls'] or actual_selector['ordered_calls']:
            raise ValueError('set592: named-selector retirement must remain complete')
        text = (root / RECORDED).read_text()
        if 'closed_guarded_opaque_set_mutation_steps' in text or 'guarded_set_mutation' in text or ROUTE in text:
            raise ValueError('set592: retired opaque Set guard remains in production')
        legacy = (root / LEGACY).read_text()
        if legacy.count('#[cfg(test)]\npub(super) mod opaque_set;') != 1:
            raise ValueError('set592: historical opaque Set oracle must remain test-only')
        for route in ['guard.guarded-map-read', 'guard.opaque-map-write', 'guard.guarded-struct-pure']:
            if text.count('"' + route + '"') != 1:
                raise ValueError('set592: unaffected sibling guard changed')
        # Whole owner authority preserves generic dispatch and the surviving guard priorities.
        report['current_selector'] = actual_selector
        report['route_checks'] = copy.deepcopy(prior['route_checks'])
        for row in baseline['routes']:
            actual_row = next(x for x in report['route_checks'] if x['id'] == row['id'])
            for anchor in row['anchors']:
                adapted = [x for x in transition['anchor_adaptations'] if x['route'] == row['id'] and x['previous'] == anchor]
                if len(adapted) > 1:
                    raise ValueError('set592: ambiguous reviewed anchor adaptation')
                retired = row['id'] in [x['id'] for x in prior['retired_routes']] + [ROUTE]
                if adapted and retired:
                    raise ValueError('set592: retired route cannot be relabeled as an anchor adaptation')
                current = adapted[0]['current'] if adapted else [{
                    'source': anchor['source'], 'text': anchor['text'],
                    'occurrences': 0 if retired else anchor['baseline_occurrences']}]
                for check in current:
                    count = (root / check['source']).read_text().count(check['text'])
                    if count != check['occurrences']:
                        actual_row['failures'].append({'source':check['source'],'anchor':check['text'],'expected':check['occurrences'],'actual':count})
            actual_row['passed'] = not actual_row['failures']
        if any(not x['passed'] for x in report['route_checks']):
            raise ValueError('set592: actual source anchor drift outside the reviewed retirement')
        report['reviewed_anchor_adaptations'] = copy.deepcopy(transition['anchor_adaptations'])
        report['retired_routes'] = copy.deepcopy(prior['retired_routes'])
        report['retired_routes'].append({'id':ROUTE,'status':'retired-from-production-test-oracle-retained','passed':True,'evidence':transition})
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        if ids.count(ROUTE) != 1 or any(ids.count(route) != 1 for route in REPLACEMENTS):
            raise ValueError('set592: unexpected prior instrumentation identity')
        report['reviewed_instrumentation'] = {'issue':592,'instrumented_route_ids':[x for x in ids if x != ROUTE], 'claim':'reviewed source instrumentation; execution of every boundary is not claimed'}
        for owner, expected in baseline['source_fingerprints'].items():
            actual = digest((root / owner).read_bytes())
            if actual != expected:
                report['source_changes'].append({'source':owner,'baseline_sha256':expected,'current_sha256':actual})
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError) as error:
        report['errors'].append(str(error))
    return report
