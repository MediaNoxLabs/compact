# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive test-only measurement authority; no resource policy or support change."""
import copy
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_test_adaptations import restore_tests, TestCheckpoint

MANIFEST_SHA = '9c6f1535569f7dedfe82ebfd84a12e49300828886a80f4772843be9a9dcd5975'
BASELINE_COMMIT = '76d99c743902fb47dc66433ee3e59d2e44158278'
BASE = 'tools/compact-rust-backend/'
CHECKER = BASE + 'src/recorded/effect_plan/pure_certification/checker.rs'
PROBE = BASE + 'src/recorded/effect_plan/pure_certification/checker/resource_measurement.rs'
PARENT = BASE + 'recording_admission_inventory.body-checker-622d4c.json'
REGISTRATION = '\n#[cfg(test)]\nmod resource_measurement;\n'
LABEL = 'measurement622D5'


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA
            or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'D5-measurement'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != {CHECKER}
            or set(extension.get('new_sources', {})) != {PROBE}
            or extension['owners'][CHECKER]['test_adaptations'] != [
                {'current': REGISTRATION, 'previous': ''}]
            or len(extension.get('historical_manifests', {})) != 79
            or PARENT not in extension['historical_manifests']):
        raise ValueError(LABEL + ': unreviewed or incomplete test-only extension')


def predecessor_sources(root, extension):
    authenticate(extension)
    # Current probe, historical authority and current owner are authenticated
    # before creating any read-only predecessor view.
    for category in ('new_sources', 'new_and_shared_sources', 'historical_manifests'):
        for owner, expected in extension[category].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError(LABEL + ': actual ' + category + ' changed: ' + owner)
    row = extension['owners'][CHECKER]
    owners = {CHECKER: (row['previous_sha256'], row['current_sha256'])}
    return restore_tests(root, extension, MANIFEST_SHA, owners, LABEL)


def checkpoint(root, extension):
    sources = predecessor_sources(root, extension)
    row = extension['owners'][CHECKER]
    return TestCheckpoint(root, sources, {
        CHECKER: (row['previous_sha256'], row['current_sha256'])}, LABEL)


def verify_extension(root, baseline, extension, historical_verify):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        previous = checkpoint(root, extension)
        prior = historical_verify(previous)
        if not prior['passed']:
            report['errors'] = [LABEL + ': predecessor checks failed', *prior['errors']]
            return report
        stage = prior.get('reviewed_body_checker_extension', {})
        if (stage.get('issue') != 622 or stage.get('slice') != 'D4c'
                or stage.get('historical_manifest_count') != 78
                or stage.get('original_oracles') != 49
                or prior['route_count'] != 84 or len(prior['retired_routes']) != 69):
            raise ValueError(LABEL + ': mandatory actual D4c authority/accounting missing')
        report = copy.deepcopy(prior)
        report['reviewed_resource_measurement_extension'] = {
            'issue': 622, 'slice': 'D5-measurement',
            'predecessor_commit': BASELINE_COMMIT, 'new_retired_routes': [],
            'historical_manifest_count': 79, 'original_oracles': 49,
            'claim': extension['claim'], 'owners': copy.deepcopy(extension['owners']),
            'qualification': 'Ignored measurement probe only; production policy unchanged. '
                             'Measured work/memory, EA11/EA13 and goal closure remain separate.'}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, TypeError) as error:
        report['errors'].append(str(error))
    return report
