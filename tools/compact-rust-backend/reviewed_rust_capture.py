# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Consume existing reviewed Rust replacements without changing TS evidence.

Only two Rust provenance digest slots may differ. The complete fresh capture
otherwise equals the immutable retained capture, with JSON scalar types exact.
"""
import copy
import hashlib
import json
from pathlib import Path, PurePosixPath
import re

import local_parity_gate as common

SUPPORT = 'tests-rust-backend/support/reviewed_capture.rs'
SUPPORT_SHA256 = '482cea8e2806f220a939e02f459ffad9c7ff25c685e7bf6aab051f2bc528217b'
# Pins to existing reviewed contracts, not new behavior baselines.
REVIEWED_BINDINGS = {
    'tests-rust-backend/did-point-digest-reducer': '571e6bdf76db41aa49386f383263f064601d5f67189f2e6572e28d1c34ac538c',
    'tests-rust-backend/did-point-guard-reducer': '4c6d7c70549f88f62428e6b738abc6a756de602c67d7e83a99a1ad285df9099c',
    'tests-rust-backend/did-alias-set-reducer': '48cb8f3fa0348b92a65460d36ec7aba293f40dc48832055a08697cf05a44eae8',
    'tests-rust-backend/did-alias-digest-reducer': '79790bee8244b43b57134d5a03c56b5d7da4ad66f4975e053f7af534b55d0182',
    'tests-rust-backend/did-alias-guard-reducer': 'd230b3248bf488bf7e08a1e41d7d273be45a7163cc3f750758f98c17d8dbe1d8',
    'tests-rust-backend/did-service-map-reducer': '16523e2e406ccc9588cc9f7a287b159f709ad123b2a2c7e7c1f2696cd088bf78',
    'tests-rust-backend/did-point-map-reducer': '902c8f22068493de04c690f8c9bb68ee3562f7d8e2082c992f659e4541b35b86',
    'tests-rust-backend/did-nested-map-reducer': '042034360b36021b9817a70216cef6583bb3a8f080066dea4f7b0ff2e2f7baa5',
    'tests-rust-backend/did-enum-map-reducer': 'd2be9eb1206a6b57503409361490d04c0fd18310c24c398db9f2f6850dc91277',
}


def require(condition, message):
    if not condition:
        raise common.GateError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relative(value):
    require(isinstance(value, str) and bool(value) and '\\' not in value,
            'invalid reviewed relative path')
    require(not value.startswith('/') and all(part not in ('', '.', '..') for part in value.split('/')),
            'invalid reviewed relative path')
    return Path(PurePosixPath(value))


def pairs(entries):
    result = {}
    for key, value in entries:
        require(key not in result, f'duplicate JSON key: {key}')
        result[key] = value
    return result


def read_json(path):
    return json.loads(path.read_text(), object_pairs_hook=pairs)


def sha(value):
    require(isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value) is not None,
            'invalid reviewed SHA256')
    return value


def table(value, fields, key, label):
    require(isinstance(value, list) and bool(value), f'empty or missing {label}')
    result = {}
    for row in value:
        require(isinstance(row, dict) and set(row) == set(fields), f'invalid {label} row')
        name = row[key]
        relative(name)
        require(name not in result, f'duplicate {label}: {name}')
        for field in fields:
            if field.endswith('sha256'):
                sha(row[field])
        result[name] = row
    return result


def strict_equal(left, right):
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(strict_equal(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(strict_equal(a, b) for a, b in zip(left, right))
    return left == right


def compare_reviewed_capture(fresh, root, fixture, capture_name='cases.json'):
    """Authorize the existing two-epoch identity contract, then compare all data."""
    fixture = str(fixture)
    require(fixture in REVIEWED_BINDINGS, 'fixture has no pinned reviewed binding')
    root = Path(root)
    package = root / relative(fixture)
    binding_path = package / 'oracle/reviewed-rust-binding.json'
    require(digest(binding_path) == REVIEWED_BINDINGS[fixture], 'reviewed binding manifest changed')
    require(digest(root / SUPPORT) == SUPPORT_SHA256, 'reviewed Rust support authority changed')
    binding = read_json(binding_path)
    require(isinstance(binding, dict) and set(binding) == {
        'format', 'scope', 'captures', 'replacements', 'reviewed_artifacts'}, 'invalid reviewed binding fields')
    require(binding['format'] == 'compact-reviewed-rust-binding/v1'
            and isinstance(binding['scope'], str) and binding['scope'], 'invalid reviewed binding identity')
    captures = table(binding['captures'], ('file', 'sha256'), 'file', 'capture')
    require(capture_name in captures, 'capture lacks exactly one reviewed identity')
    capture_path = package / 'oracle' / relative(capture_name)
    require(digest(capture_path) == captures[capture_name]['sha256'], 'immutable TS capture changed')
    retained = read_json(capture_path)
    require(isinstance(retained, dict) and isinstance(fresh, dict), 'capture object required')
    artifacts = table(binding['reviewed_artifacts'], ('path', 'sha256'), 'path', 'reviewed artifact')
    require(SUPPORT in artifacts and artifacts[SUPPORT]['sha256'] == SUPPORT_SHA256,
            'reviewed support artifact missing')
    for path, row in artifacts.items():
        require(digest(root / relative(path)) == row['sha256'], f'stale reviewed artifact: {path}')
    replacements = table(binding['replacements'], ('path', 'captured_sha256', 'current_sha256'),
                         'path', 'replacement')
    allowed = {f'{fixture}/lib.rs', f'{fixture}/tests/provenance.rs'}
    require(set(replacements) == allowed, 'reviewed replacement range differs from exact two Rust paths')
    provenance_path = f'{fixture}/tests/provenance.rs'
    # Existing alias-guard binds this file through replacements only; every
    # current identity must still be authenticated, without inventing a new
    # artifact-list membership rule absent from reviewed_capture.rs.
    provenance_identity = (artifacts.get(provenance_path, {}).get('sha256')
                           or replacements.get(provenance_path, {}).get('current_sha256'))
    require(provenance_identity is not None, 'reviewed provenance test identity missing')
    require(digest(root / relative(provenance_path)) == provenance_identity,
            'reviewed provenance test identity changed')
    historical = table(retained.get('provenance'), ('path', 'sha256'), 'path', 'historical provenance')
    current = table(fresh.get('provenance'), ('path', 'sha256'), 'path', 'fresh provenance')
    require(set(current) == set(historical) and allowed <= set(historical), 'provenance inventory changed')
    expected = copy.deepcopy(retained)
    for row in expected['provenance']:
        path = row['path']
        actual = digest(root / relative(path))
        if path in replacements:
            replacement = replacements[path]
            require(row['sha256'] == replacement['captured_sha256'], f'historical binding changed: {path}')
            require(actual == replacement['current_sha256'], f'stale reviewed replacement: {path}')
            require(current[path]['sha256'] == replacement['current_sha256'], f'fresh Rust provenance differs: {path}')
            row['sha256'] = replacement['current_sha256']
        else:
            require(actual == row['sha256'] == current[path]['sha256'], f'unchanged producer provenance changed: {path}')
    require(strict_equal(fresh, expected), 'fresh oracle differs outside exact reviewed Rust provenance replacements')
    return {'binding': str(binding_path), 'binding_sha256': REVIEWED_BINDINGS[fixture],
            'capture_sha256': captures[capture_name]['sha256'], 'replaced_paths': sorted(allowed)}
