# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate reviewed tool evolution without rewriting an old source epoch.

Historical bytes are evidence for exact owner inverses only. This component
never installs an import hook or returns those bytes as executable current code.
"""
import hashlib
import json
from pathlib import Path


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return digest(json.dumps(value, sort_keys=True, separators=(',', ':')).encode())


def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate source transition field: ' + key)
        result[key] = value
    return result


def within(root, relative):
    if not isinstance(relative, str) or not relative:
        raise ValueError('source transition path is not a relative filename')
    path = (root / relative).resolve()
    if Path(relative).is_absolute() or not path.is_relative_to(root.resolve()):
        raise ValueError('source transition path escapes root: ' + relative)
    return path


def fingerprints(root, descriptor, read_bytes=None):
    """Return actual-current guards after authenticating the complete transition.

Unchanged old guards keep their original hashes. Changed owners require both
their reviewed current hash and an exact authenticated old-owner inverse.
"""
    original = descriptor['actual_source_fingerprints']
    authority = descriptor.get('current_source_transition')
    if authority is None:
        return original
    read_bytes = read_bytes or Path.read_bytes
    manifest = json.loads(read_bytes(within(root, authority['path'])),
                          object_pairs_hook=unique_fields)
    if (canonical(manifest) != authority['canonical_sha256']
            or set(manifest) != {'schema_version', 'issue', 'epoch', 'current_sources', 'owner_inverses'}
            or type(manifest['schema_version']) is not int or manifest['schema_version'] != 1
            or type(manifest['issue']) is not int or manifest['issue'] != 632):
        raise ValueError('unreviewed current source transition')
    epoch = manifest['epoch']
    if set(epoch) != {'path', 'sha256', 'canonical_sha256'}:
        raise ValueError('source transition epoch fields differ')
    epoch_bytes = read_bytes(within(root, epoch['path']))
    historical = json.loads(epoch_bytes, object_pairs_hook=unique_fields)
    if (digest(epoch_bytes) != epoch['sha256'] or canonical(historical) != epoch['canonical_sha256']
            or historical['actual_source_fingerprints'] != original):
        raise ValueError('source transition changed the original source epoch')
    current, inverses = manifest['current_sources'], manifest['owner_inverses']
    if not isinstance(current, dict) or not isinstance(inverses, dict):
        raise ValueError('source transition owner tables differ')
    changed = {name for name, pin in current.items() if name in original and pin != original[name]}
    if set(inverses) != changed:
        raise ValueError('source transition inverse closure differs')
    for relative, pin in current.items():
        if digest(read_bytes(within(root, relative))) != pin:
            raise ValueError('current source transition owner drift: ' + relative)
    for relative, inverse in inverses.items():
        if (set(inverse) != {'path', 'sha256'} or inverse['sha256'] != original[relative]
                or digest(read_bytes(within(root, inverse['path']))) != original[relative]):
            raise ValueError('source transition historical owner inverse differs: ' + relative)
    return {**original, **current}
