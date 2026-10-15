# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Small shared mechanics for fixed-authority, test-only source checkpoints."""
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot


def restore_tests(root, extension, manifest_hash, owners, label):
    if canonical(extension) != manifest_hash:
        raise ValueError(label + ': unreviewed test manifest')
    restored = {}
    for owner, (previous, current) in owners.items():
        source = (root / owner).read_text()
        if digest(source.encode()) != current:
            raise ValueError(label + ': current test owner changed: ' + owner)
        for edit in extension['owners'][owner]['test_adaptations']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError(label + ': replacement is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != previous:
            raise ValueError(label + ': complete predecessor reconstruction failed: ' + owner)
        restored[owner] = source
    return restored


class TestCheckpoint:
    """Wrappers supply immutable reviewed owners; every other path stays live."""
    def __init__(self, root, sources, owners, label):
        if set(sources) != set(owners) or any(
                digest(source.encode()) != owners[owner][0] for owner, source in sources.items()):
            raise ValueError(label + ': unauthenticated test snapshot')
        self._root, self._sources = root, sources

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner
