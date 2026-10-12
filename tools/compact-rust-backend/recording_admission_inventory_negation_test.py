# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate EcNeg test updates; all production reads remain live."""
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = 'e9e5cb3217bec452fa16072a9906f78bbc9f74f6f48eee3fde4ecaacd07c2732'
PRIOR_MANIFEST_SHA = '881dbeecd6a1aaffe9d251fe8ea9e3d82cad42e4462afc0ca86641984a52a1f8'
OWNERS = {
    'tools/compact-rust-backend/src/recorded/profile_selection/jubjub_migration_tests.rs': (
        'fc69a8161bfc9258e9bcf82bd83c07c649bb41df2a1ffd5e4d3260d09df1eb82',
        '5e59d83f2ea7a1a17b1bf166e3a6e26d03ee63a5a07acbe632da7b349f602ae1',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/context_migration_tests/routing.rs': (
        '6bb36fd5a1e72a875014da0149fd6cd6785f4ef2071709402775476f0bd9e39e',
        'daef435b975f919af2897353e1a96a082d9edf7ee6e87a08d170bb194ef6a0e7',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/kernel_migration_tests/routing.rs': (
        'ee2e177221ad1e9ba8264218edda7400d7ec97c86645fae8058a81b64273bfc3',
        '6252ae3775f2203d7731578620964ff5bcddafc98ab987c62af21f24f667b221',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs': (
        '45d038effc2eded6aceae796118a3b2fd8df4657b9c38a7e4b2c5ef9a74c9362',
        '3b49e9404eef90f6df88ca210f85b01cb6a305a2799c7c9487cd1d703bc03deb',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/receive_migration_tests/routing.rs': (
        '89f2a1b2a12896d1c7dedeb5769791228b00c24bc62f24c0764d73fb77acea7d',
        '034c8e9c1baaf96953c369789934b511cf7d9f81fc8a4dc65682d1512f139db0',
    ),
    'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': (
        'f74d2a67c747ae71ccfa61436f352233af5a00ff83c02af6440ce93784c83c6e',
        '33ea2c276a6dede61be2fad0b1c5880959c8d421e77c17e23afe1f98e1cea411',
    ),
}


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('negation585: unreviewed test manifest')
    restored = {}
    for owner, (previous, current) in OWNERS.items():
        source = (root / owner).read_text()
        if digest(source.encode()) != current:
            raise ValueError('negation585: current test owner changed: ' + owner)
        for edit in extension['owners'][owner]['test_adaptations']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('negation585: replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != previous:
            raise ValueError('negation585: complete predecessor reconstruction failed: ' + owner)
        restored[owner] = source
    return restored


class _TestCheckpoint:
    """Only six authenticated test owners; never redirect a production source."""
    def __init__(self, root, sources):
        if set(sources) != set(OWNERS) or any(
                digest(source.encode()) != OWNERS[owner][0] for owner, source in sources.items()):
            raise ValueError('negation585: unauthenticated test snapshot')
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension, encoding):
    if encoding is None or canonical(encoding) != PRIOR_MANIFEST_SHA:
        raise ValueError('negation585: immutable encoding583 predecessor required')
    return _TestCheckpoint(root, predecessor_sources(root, extension))
