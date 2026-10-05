#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
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
            rendered = '{ path = ' + json.dumps(str(repo / 'runtime-rs')) + ', features = ["ledger-transaction"] }'
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
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    runtime_files = {str(p.relative_to(repo / 'runtime-rs')): digest(p)
                     for p in sorted((repo / 'runtime-rs/src').rglob('*.rs'))}
    runtime_files['Cargo.toml'] = digest(repo / 'runtime-rs/Cargo.toml')
    provenance = {'compilerSha256': digest(args.compiler.resolve()),
                  'sourceSha256': digest(here / 'shielded.compact'),
                  'generated': evidence,
                  'runtimeSourceTreeSha256': hashlib.sha256(json.dumps(runtime_files, sort_keys=True).encode()).hexdigest(),
                  'repositoryCargoLockSha256': digest(repo / 'Cargo.lock'),
                  'abi': 49, 'schema': 20}
    (scratch / 'src/build-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')

    env = dict(os.environ, CARGO_INCREMENTAL='0', CARGO_TARGET_DIR=str(args.cargo_target_dir.resolve()))
    subprocess.run(['cargo', '+1.99.0', 'build', '--offline', '--manifest-path', str(scratch / 'Cargo.toml')], env=env, check=True)


if __name__ == '__main__':
    main()
