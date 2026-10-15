# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Test-only current-candidate projection for immutable C mutation laws.

Production checkpoints never use this projection: they require complete actual
and predecessor hashes. Here approved exact regions are restored while a law's
intentional mutation outside those regions survives for its historical guard.
"""
from recording_admission_inventory_jubjub import digest
from recording_admission_inventory_shared_coercion import function
import recording_admission_inventory_shared_leaf as leaf


def project_candidate(owner, data, extension):
    leaf.authenticate(extension)
    if owner not in extension['owners']:
        return data
    source = data.decode()
    if owner == leaf.LIB:
        if digest(function(source, 'pure_leaf_expression').encode()) != extension['current_leaf_adapter_sha256']:
            raise ValueError('leaf622D1a test: current leaf mutated; refusing candidate projection')
    return leaf.inverse_regions(source, extension['owners'][owner], owner).encode()


def historical_overlay(base, extension, overlay_type):
    leaf.authenticate(extension)
    class CandidateChanges(dict):
        def __setitem__(self, owner, data):
            super().__setitem__(owner, project_candidate(owner, data, extension))
    overlay = overlay_type(base)
    overlay.changes = CandidateChanges()
    return overlay
