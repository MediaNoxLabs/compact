#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0

"""Shared local-test key preparation, never a compiler or test-result cache.

The scoped zkir launcher handles v2 compile/compile-many only. Every request
reads current JSON, asks the installed Forge to prepare it, verifies its export
and actual parameters, and runs the real CLI's fresh mock compilation. Private
copies preserve the normal output layout and compiler-generated manifest hashes.
COMPACT_ZKIR_MODE=uncached retains the original key-generation qualification path.
"""
from contextlib import contextmanager
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
FORGE = Path.home() / '.local/bin/midnight-zkir-forge'
PROFILE = ('ledger8-8.1.3/zkir-2.1.1/transient-2.1.1/base-1.0.2/'
           'serialize-1.1.1/proofs-0.7.1/circuits-6.1.0/stdlib-1.1.0/'
           'curves-0.2.0/tagged-v1/forge-recipe-v1')
BUILD = '340e0b3bd94394d987c1b2a073bb411afb755e7164934ca15e8d8cb02b77b879'
REVISION = 'b5d3e965a18e4cac17fac4f38446fc55c524439a'
SOURCE = 'git+https://github.com/midnightntwrk/midnight-ledger?rev=' + REVISION + '#' + REVISION
CONSUMER = {'midnight-ledger': '8.1.3', 'midnight-zkir': '2.1.1',
            'midnight-transient-crypto': '2.1.1', 'midnight-base-crypto': '1.0.2',
            'midnight-serialize': '1.1.1', 'midnight-proofs': '0.7.1',
            'midnight-circuits': '6.1.0', 'midnight-zk-stdlib': '1.1.0', 'midnight-curves': '0.2.0'}
GIT_PACKAGES = {'midnight-ledger', 'midnight-zkir', 'midnight-transient-crypto',
                'midnight-base-crypto', 'midnight-serialize'}
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
KINDS = {'source', 'binary_ir', 'prover', 'verifier'}


class CompatibilityError(ValueError):
    """Current input, parameters or local consumer do not match this provider."""


def require(condition, message):
    if not condition:
        raise CompatibilityError(message)


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def _fields(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, 'duplicate Forge JSON field: ' + key)
        value[key] = item
    return value


def read_json(raw):
    return json.loads(raw, object_pairs_hook=_fields)


def validate_consumer(lock=None):
    packages = tomllib.loads((lock or ROOT / 'Cargo.lock').read_text())['package']
    for name, version in CONSUMER.items():
        rows = [row for row in packages if row['name'] == name]
        if name == 'midnight-zkir' and not rows:
            continue  # Runtime consumers need not depend on the separate CLI/parser crate.
        expected_source = SOURCE if name in GIT_PACKAGES else REGISTRY
        require(len(rows) == 1 and rows[0]['version'] == version and rows[0].get('source') == expected_source,
                'Forge requires the matching official ledger8.1.3 consumer: ' + name)


def parameter_directory(env):
    if env.get('MIDNIGHT_PP'):
        return Path(env['MIDNIGHT_PP']).resolve()
    cache = Path(env['XDG_CACHE_HOME']) if env.get('XDG_CACHE_HOME') else Path(env.get('HOME', str(Path.home()))) / '.cache'
    return (cache / 'midnight/zk-params').resolve()


def zkir_executable(env):
    """Snapshot the actual tool, even when a parent harness already has a shim."""
    return env.get('COMPACT_ZKIR_REAL') or shutil.which('zkir', path=env.get('PATH'))


def validate_export(value, source, params, *, expected_source=None):
    expected_source = expected_source or digest(source)
    require(type(value.get('schema')) is int and value['schema'] == 1
            and value.get('profile') == PROFILE and value.get('build_fingerprint') == BUILD
            and value.get('ledger_revision') == REVISION, 'unqualified Forge profile/build/revision')
    require(type(value['cache']['hit']) is bool, 'missing Forge cache outcome')
    require(value['export']['lifetime'] == 'persistent_until_owner_removal', 'persistent Forge export required')
    base = Path(value['export']['base'])
    require(base.is_absolute() and not base.is_symlink(), 'Forge export base must be a regular absolute directory')
    manifest_path = base / 'manifest.json'
    require(not manifest_path.is_symlink(), 'Forge manifest symlink refused')
    manifest = read_json(manifest_path.read_bytes())
    require(type(manifest.get('schema')) is int and manifest['schema'] == 1, 'invalid Forge manifest schema')
    for key in ('profile', 'build_fingerprint', 'ledger_revision', 'bundle_id', 'derivation_id', 'artifact_set_id'):
        require(manifest[key] == value[key], 'Forge export manifest identity differs: ' + key)
    require(manifest['source']['kind'] == 'source' and manifest['source']['digest'] == expected_source,
            'Forge source differs from current input')
    k = manifest['k']
    require(type(k) is int and 0 <= k <= 255, 'invalid Forge parameter size')
    require(digest(params / f'bls_midnight_2p{k}') == manifest['parameter_digest'],
            'Forge parameters differ from actual consumer parameter bytes')
    declared = {}
    for row in [manifest['source'], *manifest['artifacts']]:
        require(row['kind'] not in declared, 'duplicate Forge manifest artifact kind')
        declared[row['kind']] = row
    paths = {}
    for row in value['export']['files']:
        kind, path = row['kind'], Path(row['path'])
        require(kind in KINDS and kind not in paths, 'unknown or duplicate Forge artifact kind')
        require(path.is_absolute() and not path.is_symlink() and path.resolve().is_relative_to(base.resolve()),
                'Forge artifact must be a regular file inside its persistent export')
        require(kind in declared, 'Forge manifest artifact missing: ' + kind)
        item = declared[kind]
        require(type(row['bytes']) is int and row['bytes'] > 0 and path.stat().st_size == row['bytes'] == item['size']
                and type(item['size']) is int and digest(path) == row['sha256'] == item['digest'],
                'corrupt Forge artifact: ' + kind)
        paths[kind] = path
    require(set(paths) == set(declared) == KINDS, 'incomplete Forge export')
    require(digest(source) == expected_source and paths['source'].read_bytes() == source.read_bytes(),
            'current circuit changed during Forge preparation')
    return paths, manifest


def prepare(source, params=None, *, forge=None, cache_dir=None, env=None):
    env = dict(os.environ if env is None else env)
    source = Path(source)
    require(not source.is_symlink() and source.suffix == '.zkir', 'prepare requires current regular JSON ZKIR')
    source = source.resolve(strict=True)
    source_sha = digest(source)  # Read current generated circuit on every request.
    validate_consumer()
    params = Path(params).resolve() if params is not None else parameter_directory(env)
    argv = [str(forge or env.get('COMPACT_ZKIR_FORGE_BIN', FORGE))]
    if cache_dir is not None:
        argv += ['--cache-dir', str(Path(cache_dir).resolve())]
    argv += ['--params-dir', str(params), 'prepare', str(source)]
    if re.fullmatch(r'[A-Za-z0-9_-]{1,128}', source.stem) is None:
        # Export names are cosmetic. Preserve arbitrary compiler circuit names
        # at the requested destinations without adding Forge's name restriction.
        argv += ['--name', 'compact-' + source_sha[:24]]
    start = time.perf_counter()
    result = subprocess.run(argv, env=env, check=True, capture_output=True, text=True)
    value = read_json(result.stdout)
    paths, manifest = validate_export(value, source, params, expected_source=source_sha)
    return paths, {'provider': 'midnight-zkir-forge', 'profile': PROFILE,
                   'prepare_wall_seconds': time.perf_counter() - start, 'cache': value['cache'],
                   'bundle_id': value['bundle_id'], 'derivation_id': value['derivation_id'],
                   'artifact_set_id': value['artifact_set_id'], 'persistent_export': value['export']['base'],
                   'source_sha256': source_sha, 'parameter_digest': manifest['parameter_digest'], 'k': manifest['k'],
                   'artifacts': value['export']['files']}


def _copy(source, destination, receipt, env):
    destination = Path(destination)
    cache = Path(env.get('MIDNIGHT_ZKIR_FORGE_DIR', str(Path.home() / '.midnight-zkir-forge'))).resolve()
    require(not destination.is_symlink() and not destination.resolve().is_relative_to(cache)
            and not destination.resolve().is_relative_to(Path(receipt['persistent_export']).resolve()),
            'refusing to overwrite Forge storage/export or an output symlink')
    destination.parent.mkdir(parents=True, exist_ok=True)
    expected = next(row for row in receipt['artifacts'] if Path(row['path']) == source)
    with tempfile.NamedTemporaryFile(dir=destination.parent, prefix='.forge-', delete=False) as stream:
        temporary = Path(stream.name)
    try:
        shutil.copyfile(source, temporary)
        require(digest(temporary) == expected['sha256'] and temporary.stat().st_size == expected['bytes'],
                'Forge export changed while copying')
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)


def compile_circuit(source, prover, verifier, real_zkir, env):
    source = Path(source)
    require(not source.is_symlink(), 'current ZKIR symlink refused')
    source = source.resolve(strict=True)
    destinations = [source.with_suffix('.bzkir'), Path(prover), Path(verifier)]
    store = Path(env.get('MIDNIGHT_ZKIR_FORGE_DIR', str(Path.home() / '.midnight-zkir-forge'))).resolve()
    require(not source.is_relative_to(store), 'current test input cannot be a Forge store object')
    require(len({path.resolve() for path in destinations}) == 3 and source not in {path.resolve() for path in destinations},
            'circuit input/output paths must be distinct')
    require(all(not path.is_symlink() and not path.resolve().is_relative_to(store) for path in destinations),
            'refusing output symlink or Forge storage target')
    # The actual CLI supplies fresh model metrics and independently serializes
    # current IR. This is NOT keygen; label it honestly even on a Forge miss.
    version = subprocess.run([str(real_zkir), '--version'], env=env, check=True, capture_output=True, text=True)
    require(re.fullmatch(r'(?:midnight-)?zkir 2\.1\.1', version.stdout.strip()),
            'Forge requires matching actual zkir2.1.1 CLI')
    paths, receipt = prepare(source, env=env)
    model = subprocess.run([str(real_zkir), 'mock-compile', str(source)], env=env,
                           check=True, capture_output=True, text=True)
    shape = re.findall(r'\(k=(\d+), rows=(\d+)\)', model.stdout + model.stderr)
    require(len(shape) == 1 and int(shape[0][0]) == receipt['k'], 'fresh CLI model differs from Forge parameter size')
    require(digest(source.with_suffix('.bzkir')) == digest(paths['binary_ir']),
            'actual CLI serialization differs from Forge binary IR')
    require(digest(source) == receipt['source_sha256'], 'source changed while measuring current model')
    _copy(paths['binary_ir'], source.with_suffix('.bzkir'), receipt, env)
    _copy(paths['prover'], Path(prover), receipt, env)
    _copy(paths['verifier'], Path(verifier), receipt, env)
    receipt.update(rows=int(shape[0][1]), model_provider='actual zkir mock-compile (fresh; not keygen)',
                   materialization='private copies into requested output layout')
    print('Forge artifacts (' + ('hit' if receipt['cache']['hit'] else 'miss') + '); fresh CLI model follows', file=sys.stderr)
    print(model.stdout, end='', file=sys.stdout)
    print(model.stderr, end='', file=sys.stderr)
    print('COMPACT_FORGE ' + json.dumps(receipt, sort_keys=True), file=sys.stderr)
    if env.get('COMPACT_ZKIR_EVENTS'):
        directory = Path(env['COMPACT_ZKIR_EVENTS'])
        directory.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(mode='w', prefix='prepare-', suffix='.json', dir=directory, delete=False) as stream:
            json.dump(receipt, stream, indent=2)
            stream.write('\n')
    return receipt


def configure_environment(environment, directory, *, real_zkir=None):
    """Return a scoped subprocess env; caller owns directory and its lifetime."""
    env = dict(environment)
    mode = env.get('COMPACT_ZKIR_MODE', 'uncached')
    require(mode in ('forge', 'uncached'), 'COMPACT_ZKIR_MODE must be forge or uncached')
    if mode == 'uncached':
        return env
    real = real_zkir or zkir_executable(env)
    real = Path(real).resolve(strict=True) if real is not None else None
    directory = Path(directory).resolve()
    binary = directory / 'bin/zkir'
    require(real != binary, 'recursive Forge shim refused')
    binary.parent.mkdir(parents=True, exist_ok=True)
    require(not binary.exists() and not binary.is_symlink(), 'fresh Forge shim directory required')
    binary.write_text(f'#!{sys.executable}\nimport runpy, sys\n'
                      f'adapter = runpy.run_path({str(Path(__file__).resolve())!r})\n'
                      'raise SystemExit(adapter["_shim"](sys.argv[1:]))\n')
    binary.chmod(0o700)
    env.update(COMPACT_ZKIR_REAL=str(real) if real is not None else '', COMPACT_ZKIR_SHIM=str(binary),
               COMPACT_ZKIR_EVENTS=str(directory / 'events'), PATH=str(binary.parent) + os.pathsep + env.get('PATH', ''))
    return env


def command(argv, env):
    """Also cover gates that invoke a frozen absolute zkir snapshot."""
    args = [str(value) for value in argv]
    if (env.get('COMPACT_ZKIR_MODE', 'uncached') == 'forge' and len(args) > 1
            and Path(args[0]).name == 'zkir' and args[1] in ('compile', 'compile-many')):
        require(bool(env.get('COMPACT_ZKIR_SHIM')), 'key preparation requires a configured test environment')
        return [env['COMPACT_ZKIR_SHIM'], *args[1:]]
    return args


def provenance(env):
    mode = env.get('COMPACT_ZKIR_MODE', 'uncached')
    return {'mode': mode, 'provider_on_generation_requests': 'Forge prepare' if mode == 'forge' else 'actual zkir compile',
            'legacy_cli_keygen_on_generation_requests': mode == 'uncached',
            'actual_zkir': env.get('COMPACT_ZKIR_REAL'), 'events_directory': env.get('COMPACT_ZKIR_EVENTS'),
            'scope': 'Artifacts only; proofs/assertions always execute. Forge hits use fresh actual mock-compile metrics, not legacy keygen.'}


@contextmanager
def test_environment(directory):
    """Process-local test harness context; production compiler remains unaware."""
    configured = configure_environment(os.environ, directory)
    with patch.dict(os.environ, configured):
        yield configured


def _shim(argv):
    env = dict(os.environ)
    real = env['COMPACT_ZKIR_REAL']
    require(bool(real), 'Forge test harness requires an actual zkir CLI for this operation')
    if env.get('COMPACT_ZKIR_MODE') == 'uncached' or not argv or argv[0] not in ('compile', 'compile-many'):
        return subprocess.call([real, *argv], env=env)
    if argv[0] == 'compile':
        require(len(argv) == 4, 'Forge shim supports compile SOURCE PROVER VERIFIER only')
        compile_circuit(argv[1], argv[2], argv[3], real, env)
    else:
        require(len(argv) == 3, 'Forge shim supports compile-many IR_DIR KEY_DIR only')
        sources, keys = Path(argv[1]), Path(argv[2])
        require(not keys.is_symlink(), 'key directory symlink refused')
        keys.mkdir(parents=True, exist_ok=True)
        stems = {path.stem for path in sources.iterdir() if path.suffix in ('.zkir', '.bzkir')}
        for name in sorted(stems):
            require((sources / (name + '.zkir')).is_file(), 'Forge refuses binary-only stale input: ' + name)
            compile_circuit(sources / (name + '.zkir'), keys / (name + '.prover'), keys / (name + '.verifier'), real, env)
    return 0


def main():
    if os.environ.get('COMPACT_ZKIR_SHIM') == str(Path(sys.argv[0]).resolve()):
        return _shim(sys.argv[1:])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--uncached', action='store_true')
    parser.add_argument('--forge', type=Path)
    parser.add_argument('--cache-dir', type=Path)
    parser.add_argument('--params-dir', type=Path)
    parser.add_argument('--receipt-dir', type=Path)
    parser.add_argument('argv', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    argv = args.argv[1:] if args.argv[:1] == ['--'] else args.argv
    require(bool(argv), 'test command required after --')
    env = dict(os.environ)
    # Invoking this local wrapper is the explicit opt-in. Imported helpers
    # keep the ordinary toolchain unless the caller requests Forge.
    env.setdefault('COMPACT_ZKIR_MODE', 'forge')
    if args.uncached:
        env['COMPACT_ZKIR_MODE'] = 'uncached'
    for key, value in (('COMPACT_ZKIR_FORGE_BIN', args.forge), ('MIDNIGHT_ZKIR_FORGE_DIR', args.cache_dir),
                       ('MIDNIGHT_PP', args.params_dir)):
        if value is not None:
            env[key] = str(value.resolve())
    with tempfile.TemporaryDirectory(prefix='compact-forge-harness-') as temporary:
        configured = configure_environment(env, Path(temporary))
        if args.receipt_dir:
            configured['COMPACT_ZKIR_EVENTS'] = str(args.receipt_dir.resolve())
        return subprocess.call(command(argv, configured), env=configured)


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (CompatibilityError, OSError, ValueError, KeyError) as error:
        print('Forge test harness refused: ' + str(error), file=sys.stderr)
        raise SystemExit(2)
