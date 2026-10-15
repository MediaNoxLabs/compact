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
    'tests-rust-backend/did-point-digest-reducer': '42e3242b9b7d3bec1c42df794e5bf0bc2e0db81c6357fca93b3cd8ee4cca2cad',
    'tests-rust-backend/did-point-guard-reducer': 'd5bd4072b3173a5e6fc397d5b18b1403f746a4058287c93f21bbeaa7305f348b',
    'tests-rust-backend/did-alias-set-reducer': '9d7d6fa232d32ee23abae81ca6f1727c7c273b0b93a43d74bcaefab3c40f6b4b',
    'tests-rust-backend/did-alias-digest-reducer': 'eb22bdf44fb0a6990f04a0de711f9b9fc9a8eb613e9b3429cb167951f45d2c5c',
    'tests-rust-backend/did-alias-guard-reducer': '35b9e2125072b4925fd9182bc1ef04430243ef83e3f3c99daaa4a73f24b25829',
    'tests-rust-backend/did-service-map-reducer': '1b7d6902bbbb488ee7a16d3b0a4eb6a874b9990b48fcddae9237f03018ee8c04',
    'tests-rust-backend/did-point-map-reducer': 'da38606d1d0eb87d90e068f9824bcb5175fc126905011adeb52cd4c50269eec9',
    'tests-rust-backend/did-nested-map-reducer': 'e866df9b7a3c2a9a39b517564d901abd62461299494b778eed9dabd05410c365',
    'tests-rust-backend/did-enum-map-reducer': 'ffe1969f11db3c614197c8d8856cf60e0cc2edfee091c92a520ecf2ab6da3a1e',
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
