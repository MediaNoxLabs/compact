# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticate one generated-call encoding change before historical admission checks."""
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_keccak_test import _TextSnapshot
MANIFEST_SHA = '881dbeecd6a1aaffe9d251fe8ea9e3d82cad42e4462afc0ca86641984a52a1f8'
OWNER = 'tools/compact-rust-backend/src/recorded/typed_plan/effects.rs'
PREVIOUS_SHA = '72174159a525738409d05610e997c1b5757045638336a69e9c6e6f0e3bbd1429'
CURRENT_SHA = '9ffb60cb4d315c3d4408642eb978b88ad0c5bb1dded6776fd26a0ff78a73dd02'
PRIOR_MANIFEST_SHA = '22ee8d9c3a4b46175d66ddbfbed99a3c2c5243f3c44734942293eefadb192cdf'
SELECTOR = {'ordered_calls': ['typed_plan::lower_unit_composition_checked'], 'function_sha256': '55050283c0525fd949780b55d2e7bf4b659e05452fdbd593763c4e9e9a095351'}


def admission_prefix(source):
    start = source.index('            Expr::WitnessCall { name, arguments } => {')
    end = source.index('                self.evidence.witness_calls += 1;', start)
    return source[start:end]


def predecessor_source(root, extension):
    if canonical(extension) != MANIFEST_SHA:
        raise ValueError('encoding583: unreviewed manifest')
    current = (root / OWNER).read_text()
    if digest(current.encode()) != CURRENT_SHA:
        raise ValueError('encoding583: live complete shared source changed')
    prior = current
    for edit in extension['rendering_replacements']:
        if not edit['current'] or prior.count(edit['current']) != 1:
            raise ValueError('encoding583: encoding replacement is not unique')
        prior = prior.replace(edit['current'], edit['previous'], 1)
    if digest(prior.encode()) != PREVIOUS_SHA:
        raise ValueError('encoding583: complete predecessor reconstruction failed')
    # All declaration/signature/type/arity/domain predicates and argument lowering
    # are byte-identical; only the later quote's method name selects encoding.
    if admission_prefix(current) != admission_prefix(prior):
        raise ValueError('encoding583: actual witness admission changed')
    if digest(admission_prefix(current).encode()) != extension['witness_admission_prefix_sha256']:
        raise ValueError('encoding583: unreviewed admission prefix')
    return prior


class _EncodingCheckpoint:
    """Only authenticated effects.rs fingerprint projection; other sources stay live."""
    def __init__(self, root, source):
        if digest(source.encode()) != PREVIOUS_SHA:
            raise ValueError('encoding583: unauthenticated predecessor')
        self._root, self._source = root, source

    def __truediv__(self, owner):
        return _TextSnapshot(self._source) if owner == OWNER else self._root / owner


def checkpoint(root, extension, aligned, actual_selector):
    if aligned is None or canonical(aligned) != PRIOR_MANIFEST_SHA:
        raise ValueError('encoding583: immutable aligned582 predecessor required')
    if actual_selector != SELECTOR:
        raise ValueError('encoding583: actual selector routes/order changed')
    return _EncodingCheckpoint(root, predecessor_source(root, extension))
