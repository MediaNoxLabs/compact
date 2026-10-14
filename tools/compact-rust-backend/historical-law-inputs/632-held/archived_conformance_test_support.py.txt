# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Exact archived inputs for one immutable repository-baseline test case."""
from contextlib import contextmanager
from pathlib import Path
from unittest import mock


def authenticate_archive(archive, fingerprints, read_bytes=None):
    import historical_law_dispatcher as dispatcher
    read_bytes = read_bytes or Path.read_bytes
    snapshots = {}
    for relative, expected in fingerprints.items():
        path = (archive / relative).resolve()
        if not path.is_relative_to(archive.resolve()):
            raise ValueError('archived conformance input escapes root: ' + relative)
        value = read_bytes(path)
        if dispatcher.digest(value) != expected:
            raise ValueError('archived conformance input drift: ' + relative)
        snapshots[relative] = value
    return snapshots


@contextmanager
def archived_conformance_reads(root, descriptor):
    """Authenticate current and archived bytes before, during and after replay."""
    import historical_law_dispatcher as dispatcher
    dispatcher.authenticate_descriptor(descriptor)
    dispatcher.authenticate_sources(root, descriptor)
    authority = descriptor['archived_conformance']
    archive = Path(authority['archive_root']).resolve()
    old = authority['old_source_fingerprints']
    snapshots = authenticate_archive(archive, old)
    read_bytes, read_text = Path.read_bytes, Path.read_text
    rows = {(root / relative).resolve(): relative for relative in snapshots}
    current = descriptor['actual_source_fingerprints']
    if not set(old) <= current.keys():
        raise ValueError('archived conformance current prerequisites are unbound')

    def qualified_bytes(path):
        relative = rows.get(path.resolve())
        if relative is None:
            return read_bytes(path)
        if dispatcher.digest(read_bytes(root / relative)) != current[relative]:
            raise ValueError('archived conformance actual input drift: ' + relative)
        if dispatcher.digest(read_bytes(archive / relative)) != old[relative]:
            raise ValueError('archived conformance input drift: ' + relative)
        return snapshots[relative]

    def qualified_text(path, *args, **kwargs):
        if path.resolve() not in rows:
            return read_text(path, *args, **kwargs)
        encoding = kwargs.get('encoding') or (args[0] if args else None) or 'utf-8'
        errors = kwargs.get('errors') or (args[1] if len(args) > 1 else None) or 'strict'
        return qualified_bytes(path).decode(encoding, errors)

    try:
        with mock.patch.object(Path, 'read_bytes', qualified_bytes), \
                mock.patch.object(Path, 'read_text', qualified_text):
            yield
    finally:
        dispatcher.authenticate_sources(root, descriptor, read_bytes=read_bytes)
        authenticate_archive(archive, old, read_bytes=read_bytes)
