# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Restore one exact routing-test checkpoint without changing admission authority."""
from recording_admission_inventory_jubjub import canonical, digest

OWNER = 'tools/compact-rust-backend/src/recorded/profile_selection/zswap_migration_tests.rs'
MANIFEST_SHA = '7ff0e59ef07532f5e58816ae3c0226a28dd454c9961ec38ef3a1bf43a0f3ce9a'
PRIOR_SHA = 'fd21c83fd817a62736bc9013897ddccc2533b7c49476005769d48f998ad0890c'
CURRENT_SHA = '7703427ad6af9e07132548f25ffb2f7ae0d41c4f73fdad141056280134f555b5'
PRIOR_ROUTING = '6a17304fb2cee492340a5ebc331c15b1e2465f3d54087aa3873162a0dddea4ce'
PRIOR_PAYOUT = '147de987a048c220ad54c7bb285818d250a49cfa8c35dfe36a84e3915e23c2d9'


def predecessor_source(source, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('inequality test: unreviewed adaptation manifest')
    if digest(source.encode()) != CURRENT_SHA:
        raise ValueError('inequality test: current test differs from reviewed source')
    previous = source
    for edit in extension['test_adaptations']:
        if not edit['current'] or previous.count(edit['current']) != 1:
            raise ValueError('inequality test: exact adaptation is not unique')
        previous = previous.replace(edit['current'], edit['previous'], 1)
    if digest(previous.encode()) != PRIOR_SHA:
        raise ValueError('inequality test: complete predecessor reconstruction failed')
    return previous


def validate(root, extension, routing, payout):
    if routing is None or canonical(routing) != PRIOR_ROUTING:
        raise ValueError('inequality test: immutable routing568 authority required')
    if payout is None or canonical(payout) != PRIOR_PAYOUT:
        raise ValueError('inequality test: immutable shielded payout570 authority required')
    return predecessor_source((root / OWNER).read_text(), extension)
