# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate one observation-test update without changing instrumentation authority."""
from recording_admission_inventory_jubjub import canonical, digest

OWNER = 'tools/compact-rust-backend/src/recorded/admission_probe.rs'
MANIFEST_SHA = 'bd0286e994fcc65fd9153a4ba06ea116a4c3b0b4cc2bbf36bab3c709f8300f17'
PRIOR_SHA = '07e6e6e996fd954b49b62c8939cc13a91ef3ddb34160c3c07ca8e5b2551c304a'
CURRENT_SHA = 'be56f6b9c8a718224d8ba6e4251088d33f0341cc30e3e73b1a55f7be0acd69ce'
PRIOR_MANIFESTS = {'instrumentation': 'dc5dca0cb5499556e2b00f7b18a649fba58ff294d40f8dd593f7d6486b3ff428', 'calls': '4e814060724e33880180cb7df89401a7aa5e73c73b698debf514efde2ea54948', 'send': '660f74f8da0b8b2a35e9feba30966d2047e1094d045c00629aa049ffb09e02a6'}


def predecessor_source(source, extension):
    """Expose historical bytes only for the complete reviewed current owner."""
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('observation test: unreviewed adaptation manifest')
    if digest(source.encode()) != CURRENT_SHA:
        raise ValueError('observation test: current owner differs from reviewed source')
    edit = extension['test_adaptation']
    if not edit['current'].startswith('#[test]\nfn ') or source.count(edit['current']) != 1:
        raise ValueError('observation test: exact test adaptation is not unique')
    previous = source.replace(edit['current'], edit['previous'], 1)
    if digest(previous.encode()) != PRIOR_SHA:
        raise ValueError('observation test: complete predecessor reconstruction failed')
    return previous


def validate(root, extension, instrumentation, calls, send):
    for key, predecessor in [('instrumentation', instrumentation), ('calls', calls), ('send', send)]:
        if predecessor is None or canonical(predecessor) != PRIOR_MANIFESTS[key]:
            raise ValueError('observation test: frozen predecessor changed or unavailable: ' + key)
    if calls['reviewed_source_fingerprints'][OWNER] != PRIOR_SHA:
        raise ValueError('observation test: prior owner is not bound to call instrumentation')
    return predecessor_source((root / OWNER).read_text(), extension)
