# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate five test-only replacements; production reads stay on disk."""
from recording_admission_inventory_jubjub import canonical, digest

MANIFEST_SHA = '22ee8d9c3a4b46175d66ddbfbed99a3c2c5243f3c44734942293eefadb192cdf'
PRIOR_MANIFESTS = {'keccak581': '6ba72636214e65e23667cf99ec8981cb6cc4fdef505c503f32eb37f352a0ef10'}
OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection/context_migration_tests/routing.rs': ('a6c8da2194200712825762d2bbcbc60666656db7fc78235bda1c3d0e968a1e30', '6bb36fd5a1e72a875014da0149fd6cd6785f4ef2071709402775476f0bd9e39e'), 'tools/compact-rust-backend/src/recorded/profile_selection/kernel_migration_tests/routing.rs': ('c3040caa58e64951200318d5006f84ac930e1df46d5166c17d2d4b1ec711f963', 'ee2e177221ad1e9ba8264218edda7400d7ec97c86645fae8058a81b64273bfc3'), 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs': ('60e761a9b46425efb1c0d22ca06b132e43876ce65dc60ba5c2748da0d450c87a', '45d038effc2eded6aceae796118a3b2fd8df4657b9c38a7e4b2c5ef9a74c9362'), 'tools/compact-rust-backend/src/recorded/profile_selection/receive_migration_tests/routing.rs': ('9060be949c9586777464c46643ec142f26742e4b0dcc2f5251a996ca363ee1b1', '89f2a1b2a12896d1c7dedeb5769791228b00c24bc62f24c0764d73fb77acea7d'), 'tools/compact-rust-backend/src/recorded/profile_selection/voting_migration_tests/routing.rs': ('3afc9c2b8f52cea1305f9986c579dfdb0a03ea62118b60a1e05d461a2c060bd3', 'f74d2a67c747ae71ccfa61436f352233af5a00ff83c02af6440ce93784c83c6e')}


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('Aligned hash test: unreviewed adaptation manifest')
    restored = {}
    for owner, (prior_hash, current_hash) in OWNERS.items():
        source = (root / owner).read_text()
        if digest(source.encode()) != current_hash:
            raise ValueError('Aligned hash test: current owner differs from reviewed source: ' + owner)
        for edit in extension['owners'][owner]['test_adaptations']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('Aligned hash test: exact adaptation is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != prior_hash:
            raise ValueError('Aligned hash test: complete predecessor reconstruction failed: ' + owner)
        restored[owner] = source
    return restored


class _TextSnapshot:
    def __init__(self, source):
        self._source = source

    def read_text(self):
        return self._source

    def read_bytes(self):
        return self._source.encode()


class _TestCheckpoint:
    """Exactly five authenticated test files; every other path reads the real root."""
    def __init__(self, root, sources):
        if set(sources) != set(OWNERS):
            raise ValueError('Aligned hash test: only the reviewed test owners may be overlaid')
        for owner, source in sources.items():
            if digest(source.encode()) != OWNERS[owner][0]:
                raise ValueError('Aligned hash test: unauthenticated predecessor source: ' + owner)
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        if owner in self._sources:
            return _TextSnapshot(self._sources[owner])
        return self._root / owner


def checkpoint(root, extension, keccak):
    if keccak is None or canonical(keccak) != PRIOR_MANIFESTS['keccak581']:
        raise ValueError('Aligned hash test: immutable predecessor required: keccak581')
    return _TestCheckpoint(root, predecessor_sources(root, extension))
