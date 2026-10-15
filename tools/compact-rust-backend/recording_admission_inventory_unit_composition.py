# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Retire Unit admission only after live authority and exact historical reconstruction."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

ROUTE = 'selector.unit-composition'
REPLACEMENT = 'algebra.unit-composition'
SELECTOR = 'tools/compact-rust-backend/src/recorded/profile_selection.rs'
TYPED = 'tools/compact-rust-backend/src/recorded/typed_plan.rs'
COMPOSITION = 'tools/compact-rust-backend/src/recorded/typed_plan/composition.rs'
MANIFEST_SHA = 'dd614d49cbece9c193d3c116322a25e2bd2ca1c45e296e2bb81e027ea6372d06'


def predecessor_sources(root, transition):
    if canonical(transition) != MANIFEST_SHA:
        raise ValueError('unit580: unreviewed retirement manifest')
    sources = {}
    for owner, row in transition['owners'].items():
        text = (root / owner).read_text()
        if digest(text.encode()) != row['current_sha256']:
            raise ValueError('unit580: live complete owner changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or text.count(edit['current']) != 1:
                raise ValueError('unit580: nonunique predecessor reconstruction: ' + owner)
            text = text.replace(edit['current'], edit['previous'], 1)
        if digest(text.encode()) != row['previous_sha256']:
            raise ValueError('unit580: signed predecessor reconstruction failed: ' + owner)
        # Signed checkpoints added only reviewed test changes after earlier inventories.
        # Authenticate that exact additional stage rather than changing old pins.
        for edit in row.get('historical_reconstruction', []):
            if not edit['current'] or text.count(edit['current']) != 1:
                raise ValueError('unit580: nonunique historical test-only reconstruction')
            text = text.replace(edit['current'], edit['previous'], 1)
        if digest(text.encode()) != row.get('historical_sha256', row['previous_sha256']):
            raise ValueError('unit580: immutable historical inventory owner differs: ' + owner)
        sources[owner] = text
    for owner, expected in transition['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('unit580: actual new/shared dependency changed: ' + owner)
    return sources


class _Checkpoint:
    """Only whole-source-authenticated predecessors; every other path remains live."""
    def __init__(self, root, sources, transition):
        if canonical(transition) != MANIFEST_SHA or set(sources) != set(transition['owners']):
            raise ValueError('unit580: unauthenticated predecessor set')
        for owner, source in sources.items():
            if digest(source.encode()) != transition['owners'][owner].get('historical_sha256', transition['owners'][owner]['previous_sha256']):
                raise ValueError('unit580: unauthenticated predecessor owner: ' + owner)
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
            report['errors'].append('unit580: immutable historical predecessor checks failed')
            report['errors'].extend(prior['errors'])
            return report
        actual_selector = selector_signature((root / SELECTOR).read_text())
        if actual_selector != transition['current_selector']:
            raise ValueError('unit580: actual selector priority or fingerprint changed')
        if prior['current_selector'] != transition['previous_selector']:
            raise ValueError('unit580: predecessor selector identity differs')
        if prior['current_selector']['ordered_calls'] != ['typed_plan::lower_unit_composition_checked'] or actual_selector['ordered_calls']:
            raise ValueError('unit580: expected exactly the final old selector retirement')
        text = (root / SELECTOR).read_text()
        if 'typed_plan::lower_unit_composition_checked(' in text or 'CompositionRejection' in text:
            raise ValueError('unit580: old Unit admission or diagnostic dependency remains')
        if not text.index('"algebra.unit-actions"') < text.index('"algebra.unit-composition"') < text.index('"algebra.zswap"'):
            raise ValueError('unit580: changed lazy priority')
        for owner, markers in transition['test_only_markers'].items():
            source = (root / owner).read_text()
            for marker in markers:
                if source.count('#[cfg(test)]\n' + marker) != 1:
                    raise ValueError('unit580: oracle must remain uniquely test-only: ' + owner + ': ' + marker)
        if ROUTE in text:
            raise ValueError('unit580: legacy selector hook remains in production selector')
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
            raise ValueError('unit580: actual source anchor drift outside the reviewed retirement')
        report['retired_routes'] = copy.deepcopy(prior['retired_routes'])
        report['retired_routes'].append({'id':ROUTE,'status':'retired-from-production-test-oracle-retained','passed':True,'evidence':transition})
        ids = prior['reviewed_instrumentation']['instrumented_route_ids']
        if ids.count(ROUTE) != 1 or REPLACEMENT in ids:
            raise ValueError('unit580: unexpected prior instrumentation identity')
        report['reviewed_instrumentation'] = {'issue':580,'instrumented_route_ids':[REPLACEMENT if x == ROUTE else x for x in ids], 'claim':'reviewed source instrumentation; execution of every boundary is not claimed'}
        for owner, expected in baseline['source_fingerprints'].items():
            actual = digest((root / owner).read_bytes())
            if actual != expected:
                report['source_changes'].append({'source':owner,'baseline_sha256':expected,'current_sha256':actual})
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError) as error:
        report['errors'].append(str(error))
    return report
