# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate one routing test update without changing production authority."""
from recording_admission_inventory_jubjub import canonical, digest

OWNER = 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs'
MANIFEST_SHA = '6a17304fb2cee492340a5ebc331c15b1e2465f3d54087aa3873162a0dddea4ce'
PRIOR_SHA = 'cd955224fe2028ab88489c849c7fe013123751ddcb7326ad8a048f9caf737a7c'
CURRENT_SHA = 'fd21c83fd817a62736bc9013897ddccc2533b7c49476005769d48f998ad0890c'
PRIOR_MANIFESTS = {'zswap': 'd0bd62c6624e4c21c2e0926ec010ceb6c343e56291b087605d98e95da1d5a573', 'membership': '59909b13f5a5c48f7288a38c186debdef278d032344298530a69c526f58dc3a1', 'send': '660f74f8da0b8b2a35e9feba30966d2047e1094d045c00629aa049ffb09e02a6', 'guarded': 'c3f02a85cae2dac6c680f2dc18897cc456b5712dc3b4a7335610b5fcb131a57d'}


def predecessor_source(source, extension):
    """Restore one exact test and its import only after whole-owner authentication."""
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('routing test: unreviewed adaptation manifest')
    if digest(source.encode()) != CURRENT_SHA:
        raise ValueError('routing test: current owner differs from reviewed source')
    previous = source
    for edit in extension['test_adaptations']:
        if not edit['current'] or previous.count(edit['current']) != 1:
            raise ValueError('routing test: exact adaptation is not unique')
        previous = previous.replace(edit['current'], edit['previous'], 1)
    if digest(previous.encode()) != PRIOR_SHA:
        raise ValueError('routing test: complete predecessor reconstruction failed')
    return previous


def validate(root, extension, zswap, membership, send, guarded):
    for key, predecessor in [('zswap', zswap), ('membership', membership), ('send', send), ('guarded', guarded)]:
        if predecessor is None or canonical(predecessor) != PRIOR_MANIFESTS[key]:
            raise ValueError('routing test: frozen predecessor changed or unavailable: ' + key)
    if (membership['reviewed_source_fingerprints'][OWNER] != PRIOR_SHA
            or send['reviewed_source_fingerprints'][OWNER] != PRIOR_SHA):
        raise ValueError('routing test: prior owner is not bound to historical transitions')
    return predecessor_source((root / OWNER).read_text(), extension)
