# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate three test-only replacements; production reads stay on disk."""
from recording_admission_inventory_jubjub import canonical, digest

MANIFEST_SHA = '6ba72636214e65e23667cf99ec8981cb6cc4fdef505c503f32eb37f352a0ef10'
PRIOR_MANIFESTS = {'inequality571': '7ff0e59ef07532f5e58816ae3c0226a28dd454c9961ec38ef3a1bf43a0f3ce9a', 'funded572': '7f53232d199c81742c35217a2cecce5fdd379da0277d2d4287d25e2fe14428eb'}
OWNERS = {'tools/compact-rust-backend/src/recorded/profile_selection/context_migration_tests/routing.rs': ('257025efe92dc2095e239edc1b33cb7d572388de71a962d70aaf0a74bbe071a8', 'a6c8da2194200712825762d2bbcbc60666656db7fc78235bda1c3d0e968a1e30'), 'tools/compact-rust-backend/src/recorded/profile_selection/kernel_migration_tests/routing.rs': ('c74b2df1ae38dc0017b709cbc6326ba6b710e1351fe6c26103624a1448811e30', 'c3040caa58e64951200318d5006f84ac930e1df46d5166c17d2d4b1ec711f963'), 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs': ('7703427ad6af9e07132548f25ffb2f7ae0d41c4f73fdad141056280134f555b5', '60e761a9b46425efb1c0d22ca06b132e43876ce65dc60ba5c2748da0d450c87a')}


def predecessor_sources(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('Keccak test: unreviewed adaptation manifest')
    restored = {}
    for owner, (prior_hash, current_hash) in OWNERS.items():
        source = (root / owner).read_text()
        if digest(source.encode()) != current_hash:
            raise ValueError('Keccak test: current owner differs from reviewed source: ' + owner)
        for edit in extension['owners'][owner]['test_adaptations']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('Keccak test: exact adaptation is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != prior_hash:
            raise ValueError('Keccak test: complete predecessor reconstruction failed: ' + owner)
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
    """Exactly three authenticated test files; every other path reads the real root."""
    def __init__(self, root, sources):
        if set(sources) != set(OWNERS):
            raise ValueError('Keccak test: only the reviewed test owners may be overlaid')
        for owner, source in sources.items():
            if digest(source.encode()) != OWNERS[owner][0]:
                raise ValueError('Keccak test: unauthenticated predecessor source: ' + owner)
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        if owner in self._sources:
            return _TextSnapshot(self._sources[owner])
        return self._root / owner


def checkpoint(root, extension, inequality, funded):
    for name, manifest in [('inequality571', inequality), ('funded572', funded)]:
        if manifest is None or canonical(manifest) != PRIOR_MANIFESTS[name]:
            raise ValueError('Keccak test: immutable predecessor required: ' + name)
    return _TestCheckpoint(root, predecessor_sources(root, extension))
