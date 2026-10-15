# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Explicit suite-only signed checker view; actual bytes remain guarded on each read."""
from contextlib import contextmanager
from pathlib import Path
from unittest import mock
import recording_admission_inventory_resource_measurement as measurement
from recording_admission_inventory_jubjub import digest


@contextmanager
def historical_suite_reads(root, extension):
    """Enter only for historical laws; new measurement laws use unwrapped actual files.

    Overlay mutations remain live snapshots and do not pass through Path reads.
    No historical verifier or assertion is patched, skipped or accepted specially.
    """
    previous = measurement.predecessor_sources(root, extension)
    target = (root / measurement.CHECKER).resolve()
    probe = (root / measurement.PROBE).resolve()
    expected = extension['owners'][measurement.CHECKER]['current_sha256']
    probe_expected = extension['new_sources'][measurement.PROBE]
    read_bytes, read_text = Path.read_bytes, Path.read_text

    def qualified_bytes(path):
        if path.resolve() != target:
            return read_bytes(path)
        if digest(read_bytes(path)) != expected:
            raise ValueError('measurement622D5 suite: actual checker drift before projection')
        if digest(read_bytes(probe)) != probe_expected:
            raise ValueError('measurement622D5 suite: actual probe drift before projection')
        return previous[measurement.CHECKER].encode()

    def qualified_text(path, *args, **kwargs):
        if path.resolve() != target:
            return read_text(path, *args, **kwargs)
        encoding = kwargs.get('encoding') or (args[0] if args else None) or 'utf-8'
        errors = kwargs.get('errors') or (args[1] if len(args) > 1 else None) or 'strict'
        return qualified_bytes(path).decode(encoding, errors)

    with mock.patch.object(Path, 'read_bytes', qualified_bytes), \
            mock.patch.object(Path, 'read_text', qualified_text):
        yield
