# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Test-only authenticated D3 source data for immutable historical law cases.

Mutations are constructed from the qualified predecessor, never repaired after
changing the current Let. Only exact approved owner reads are staged;
real paths/output protection stay intact. Production never imports this helper.
"""
import json
from pathlib import Path
from unittest import mock
import recording_admission_inventory_lexical_symbols as symbols
from recording_admission_inventory_jubjub import digest


def qualified_read_data(root, extension):
    """Authenticate all actual current sources before exposing historical data."""
    return symbols.predecessor_sources(root, extension)


def install_historical_reads(case, root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.lexical-symbols-629d4a.json')
    if not path.exists():
        return
    extension = json.loads(path.read_text())
    previous = qualified_read_data(root, extension)
    rows = {(root / owner).resolve(): (source.encode(), extension['owners'][owner]['current_sha256'])
            for owner, source in previous.items()}
    read_bytes, read_text = Path.read_bytes, Path.read_text

    def qualified_bytes(path):
        row = rows.get(path.resolve())
        if row is None:
            return read_bytes(path)
        historical, expected = row
        if digest(read_bytes(path)) != expected:
            raise ValueError('symbols629D4a test: actual current owner drift; refusing historical read view')
        return historical

    def qualified_text(path, *args, **kwargs):
        if path.resolve() not in rows:
            return read_text(path, *args, **kwargs)
        encoding = kwargs.get('encoding') or (args[0] if args else None) or 'utf-8'
        errors = kwargs.get('errors') or (args[1] if len(args) > 1 else None) or 'strict'
        return qualified_bytes(path).decode(encoding, errors)

    case.enterContext(mock.patch.object(Path, 'read_bytes', qualified_bytes))
    case.enterContext(mock.patch.object(Path, 'read_text', qualified_text))
