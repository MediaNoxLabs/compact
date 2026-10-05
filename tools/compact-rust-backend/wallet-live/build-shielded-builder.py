#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Build a standalone consumer of an unedited generated shielded acceptance crate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def runtime_inventory(runtime, expected_package="midnight-compact-runtime"):
    manifest = tomllib.loads((runtime / 'Cargo.toml').read_text())
    if manifest.get('package', {}).get('name') != expected_package:
        raise ValueError('generated runtime package identity mismatch')
    files = {str(path.relative_to(runtime)): digest(path)
             for path in sorted((runtime / 'src').rglob('*')) if path.is_file()}
    if not files:
        raise ValueError('generated runtime source is empty')
    files['Cargo.toml'] = digest(runtime / 'Cargo.toml')
    return files


def runtime_dependency_inventory(runtime):
    inventory = runtime_inventory(runtime)
    manifest = tomllib.loads((runtime / 'Cargo.toml').read_text())
    macros = manifest.get('dependencies', {}).get('midnight-compact-runtime-macros')
    if not isinstance(macros, dict) or not isinstance(macros.get('path'), str):
        raise ValueError('generated runtime must declare its local macro dependency')
    macro_root = (runtime / macros['path']).resolve()
    macro_inventory = runtime_inventory(macro_root, 'midnight-compact-runtime-macros')
    return {**{'runtime/' + key: value for key, value in inventory.items()},
            **{'macros/' + key: value for key, value in macro_inventory.items()}}


def resolve_generated_runtime(generated, reference):
    manifest = tomllib.loads((generated / 'Cargo.toml').read_text())
    dependency = manifest.get('dependencies', {}).get('midnight-compact-runtime')
    if not isinstance(dependency, dict) or not isinstance(dependency.get('path'), str):
        raise ValueError('generated contract must declare its local runtime path')
    if dependency.get('package', 'midnight-compact-runtime') != 'midnight-compact-runtime':
        raise ValueError('generated runtime dependency was renamed to another package')
    runtime = (generated / dependency['path']).resolve()
    inventory = runtime_dependency_inventory(runtime)
    if inventory != runtime_dependency_inventory(reference):
        raise ValueError('generated runtime sources differ from current reference runtime')
    return runtime, inventory


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generated-contract', type=Path, required=True)
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--scratch', type=Path, required=True)
    parser.add_argument('--cargo-target-dir', type=Path, required=True)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    repo = here.parents[2]
    generated = args.generated_contract.resolve()
    if not (generated / 'lib.rs').is_file():
        raise SystemExit('missing generated lib.rs')
    runtime, runtime_files = resolve_generated_runtime(generated, repo / 'runtime-rs')
    scratch = args.scratch.resolve()
    scratch.mkdir(mode=0o700, parents=True, exist_ok=False)
    shutil.copytree(here / 'shielded-builder', scratch / 'src')
    shutil.copy2(repo / 'Cargo.lock', scratch / 'Cargo.lock')
    source = tomllib.loads((repo / 'tools/compact-rust-proof-smoke/Cargo.toml').read_text())
    dependencies = []
    for name, spec in source['dependencies'].items():
        if name.startswith('compact-rust-'):
            continue
        if isinstance(spec, str):
            rendered = json.dumps(spec)
        elif 'path' in spec:
            if name != 'midnight-compact-runtime':
                raise ValueError('unexpected local proving dependency: ' + name)
            # Match the generated crate's actual path, including portable bundles.
            rendered = '{ path = ' + json.dumps(str(runtime)) + ', features = ["ledger-transaction"] }'
        else:
            # Preserve the pinned upstream graph; the standalone builder never
            # calls test utilities or installs a new dependency.
            rendered = '{ ' + ', '.join(f'{key} = {json.dumps(value)}' for key, value in spec.items()) + ' }'
        dependencies.append(f'{name} = {rendered}')
    dependencies.append('compact-contract-shielded = { path = ' + json.dumps(str(generated)) + ', features = ["ledger-transaction"] }')
    manifest = '[package]\nname = "compact-wallet-shielded-builder"\nversion = "0.1.0"\nedition = "2024"\npublish = false\n[dependencies]\n' + '\n'.join(dependencies) + '\n[workspace]\n'
    (scratch / 'Cargo.toml').write_text(manifest)
    evidence = {name: hashlib.sha256((generated / name).read_bytes()).hexdigest() for name in ('Cargo.toml', 'lib.rs')}
    (scratch / 'generated-inputs.json').write_text(json.dumps(evidence, indent=2) + '\n')
    provenance = {'compilerSha256': digest(args.compiler.resolve()),
                  'sourceSha256': digest(here / 'shielded.compact'),
                  'generated': evidence,
                  'runtimeDependencyPath': str(runtime),
                  'runtimeSourceTreeSha256': hashlib.sha256(json.dumps(runtime_files, sort_keys=True).encode()).hexdigest(),
                  'repositoryCargoLockSha256': digest(repo / 'Cargo.lock'),
                  'abi': 49, 'schema': 20}
    (scratch / 'src/build-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')

    env = dict(os.environ, CARGO_INCREMENTAL='0', CARGO_TARGET_DIR=str(args.cargo_target_dir.resolve()))
    subprocess.run(['cargo', '+1.99.0', 'build', '--offline', '--manifest-path', str(scratch / 'Cargo.toml')], env=env, check=True)


if __name__ == '__main__':
    main()
