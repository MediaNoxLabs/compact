# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Authenticated signed-D4b data; missing retired file remains missing on disk."""
import json
from pathlib import Path
from unittest import mock
import recording_admission_inventory_body_checker as checker
from recording_admission_inventory_jubjub import digest


def qualified_read_data(root, extension):
    return checker.predecessor_sources(root, extension)


def install_historical_reads(case, root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.body-checker-622d4c.json')
    if not path.exists():
        return
    extension = json.loads(path.read_text())
    previous = qualified_read_data(root, extension)
    rows = {(root / owner).resolve(): (source.encode(), extension['owners'][owner]['current_sha256'] if owner in extension['owners'] else None)
            for owner, source in previous.items()}
    read_bytes, read_text = Path.read_bytes, Path.read_text

    def qualified_bytes(path):
        row = rows.get(path.resolve())
        if row is None:
            return read_bytes(path)
        historical, expected = row
        if expected is None:
            try:
                read_bytes(path)
            except FileNotFoundError:
                return historical
            raise ValueError('body622D4c test: retired actual bridge returned; refusing historical view')
        if digest(read_bytes(path)) != expected:
            raise ValueError('body622D4c test: actual current owner drift; refusing historical view')
        return historical

    def qualified_text(path, *args, **kwargs):
        if path.resolve() not in rows:
            return read_text(path, *args, **kwargs)
        encoding = kwargs.get('encoding') or (args[0] if args else None) or 'utf-8'
        errors = kwargs.get('errors') or (args[1] if len(args) > 1 else None) or 'strict'
        return qualified_bytes(path).decode(encoding, errors)

    case.enterContext(mock.patch.object(Path, 'read_bytes', qualified_bytes))
    case.enterContext(mock.patch.object(Path, 'read_text', qualified_text))
