# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""One accepted-name escaping correction for fixed Rust 2024; no retirement."""
import copy
import json
import tomllib

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = 'fd7770665c95d5cf680cbbce32f9ba36ec5134a6db2a18b728f3224b4d234f3d'
BASELINE_COMMIT = 'ed42c434e2fefa53ffbb054659de37c9423494f4'
BASE = 'tools/compact-rust-backend/src'
OWNER = BASE + '/pure_semantics/identifier.rs'
TESTS = BASE + '/pure_semantics/identifier/tests.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 626 or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []):
        raise ValueError('edition626: unreviewed extension')
    if set(extension['owners']) != {OWNER, TESTS}:
        raise ValueError('edition626: both complete owners required')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('edition626: current complete owner changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('edition626: inverse is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('edition626: complete signed predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('edition626: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 68:
        raise ValueError('edition626: all 68 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root = root
        self._sources = predecessor_sources(root, extension)

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.target-edition-626.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def source_closure(root, extension, previous):
    source = (root / OWNER).read_bytes().decode()
    old = previous[OWNER]
    lexer = 'fn parse_identifier('
    if source[source.index(lexer):] != old[old.index(lexer):]:
        raise ValueError('edition626: lexical acceptance scanner changed')
    checker = 'pub(crate) fn check_name('
    mapping = '.map(|(text, raw)| CheckedName {'
    if source[source.index(checker):source.index(mapping)] != old[old.index(checker):old.index(mapping)]:
        raise ValueError('edition626: acceptance before escaping changed')
    for anchor in ('raw: raw || text == "gen",', 'text: text.to_owned(),'):
        if source.count(anchor) != 1:
            raise ValueError('edition626: fixed gen escaping or semantic key changed')
    if 'syn::' in source or 'proc_macro2::' in source or 'quote::' in source:
        raise ValueError('edition626: neutral name policy constructs syntax')
    manifest = tomllib.loads((root / 'tools/compact-rust-backend/Cargo.toml').read_bytes().decode())
    if manifest['package']['edition'] != '2024':
        raise ValueError('edition626: fixed target edition changed')
    return {'owner_sha256': digest(source.encode()), 'target_edition': '2024',
            'corrected_name': 'gen', 'corrected_output_spelling': 'r#gen',
            'lexical_acceptance': 'unchanged signed ed42 scanner and acceptance prefix',
            'semantic_collision_key': 'unchanged accepted text', 'new_retired_routes': [],
            'claim': 'only fixed Rust2024 gen escaping; no all-name/all-target or full622body claim'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['edition626: predecessor checks failed', *prior['errors']]
            return report
        previous_stage = prior.get('reviewed_shared_pure_semantics_extension', {})
        if previous_stage.get('issue') != 622 or previous_stage.get('slice') != 'A':
            raise ValueError('edition626: mandatory shared622A predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('edition626: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('edition626: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('edition626: reviewed target correction changed')
        report['reviewed_target_edition_identifiers_extension'] = {
            'issue': 626, 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': extension['baseline_commit'], 'historical_manifest_count': 68,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
