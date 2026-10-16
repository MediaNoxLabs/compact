"""Byte-identical current inputs under two historical TS producer basenames.

This is fixture-producer selection, not source rewriting or a new import parser.
All nine exact reviewed source hashes bind the stdlib-only closure inspected for
this gate. A different source/include graph requires explicit re-review.
"""
from pathlib import Path
import hashlib
import local_parity_gate as common

SOURCES = {
    'point-digest': ('tools/compact-rust-backend/tests/point-composition/point_digest.compact', 'a25583542fa4663943c5bec61144911c2014df7019e44cae0d071b97198f994d'),
    'point-guard': ('tools/compact-rust-backend/tests/point-composition/point_guard.compact', 'fa46c08ef5a54c8fe987be9bc1ffa40f223f663630d5bc73f73050a7ecacaf55'),
    'alias-set': ('tools/compact-rust-backend/tests/set-composition/set_string.compact', '04694fd4192ac63d14afacb5fb478b073dbcb977eaae262d0b4d3c6740f6aa98'),
    'alias-digest': ('tools/compact-rust-backend/tests/set-composition/opaque_digest.compact', 'a37c43c298b2f72e5cc28cf94c56ab4eaa4e16757d577acdf023be5f81f7a3c7'),
    'alias-guard': ('tools/compact-rust-backend/tests/set-composition/pure_unit_guard.compact', '411bb785786295d6a6fbd83f8d97b6a791a0b1c57f78792629853dcf311d3e91'),
    'service-map': ('tools/compact-rust-backend/tests/map-composition/service_mutation.compact', '976cf5dc624bf853175b032590925e34fcd9e93c82e660f61ebe96419a3540d9'),
    'point-map': ('tools/compact-rust-backend/tests/map-point-composition/point_nested.compact', 'd270124dd6e2eb619bfe5e4a86ed1e8be8d54a13c7321178481fe83f9b7bfa56'),
    'nested-map': ('tools/compact-rust-backend/tests/map-nested-product/nested-enum.compact', '9be89300f16afe87853910611bf5e8d84f702d0756b1158b2d3749fa342550aa'),
    'enum-map': ('tools/compact-rust-backend/tests/map-nested-product/nested-enum-wide.compact', 'fa3d21da3db95d995d88dacb9f0d282bd2a0d74e8f44d201e9998368ef766a1c'),
}
ALIASES = {'point-map':'point-and-nested-member.compact', 'nested-map':'nested-enum-crud.compact'}


def require(condition, message):
    if not condition:
        raise common.GateError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(root, directory, row, initial_sha256):
    kind = row['kind']
    require(kind in SOURCES, 'unreviewed capture source kind')
    source, expected = SOURCES[kind]
    require(row['source'] == source, 'capture source identity changed')
    original = (Path(root)/source).resolve()
    require(initial_sha256 == expected == digest(original), 'capture source closure changed')
    alias = ALIASES.get(kind)
    invocation = original
    if alias is not None:
        folder = Path(directory)/'inputs'/kind
        folder.mkdir(parents=True, exist_ok=False)
        invocation = folder/alias
        # Write actual current bytes only. Never load an old source/cached output.
        invocation.write_bytes(original.read_bytes())
        require(digest(invocation) == expected == digest(original), 'capture source copy drift')
    entry = {'kind':kind, 'tracked_source':source, 'tracked_source_path':str(original),
             'tracked_source_sha256':expected, 'invocation_source_path':str(invocation),
             'invocation_source_sha256':expected, 'historical_basename_alias':alias,
             'import_closure':'Exact reviewed source bytes import only CompactStandardLibrary; normal existing frontend/runtime resolution, no added search path or import recognizer.'}
    verify(root, directory, entry)
    return entry


def verify(root, directory, entry):
    kind = entry['kind']
    require(kind in SOURCES, 'unreviewed capture source kind')
    source, expected = SOURCES[kind]
    original = (Path(root)/source).resolve()
    alias = ALIASES.get(kind)
    invocation = (Path(directory)/'inputs'/kind/alias).resolve() if alias else original
    require(entry['tracked_source'] == source and entry['historical_basename_alias'] == alias,
            'capture source alias identity changed')
    require(Path(entry['tracked_source_path']).resolve() == original
            and Path(entry['invocation_source_path']).resolve() == invocation,
            'capture source invocation path changed')
    require(entry['tracked_source_sha256'] == entry['invocation_source_sha256'] == expected,
            'capture source declared hashes changed')
    require(digest(original) == expected and digest(invocation) == expected,
            'capture source tracked/staged bytes changed')
