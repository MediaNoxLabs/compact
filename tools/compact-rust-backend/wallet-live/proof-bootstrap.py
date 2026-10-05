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

"""Build an isolated proof consumer of the unedited generated acceptance crate.

Reuse the repository's tested proving utilities without adding an optional
runtime/compiler dependency to the main workspace. No services or chain coin
insertion are performed; only the existing Night/Dust fixture funds fees.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--generated-contract', required=True, type=Path)
    parser.add_argument('--proof-root', required=True, type=Path)
    parser.add_argument('--scratch', required=True, type=Path)
    parser.add_argument('--cargo-target-dir', required=True, type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    repo = here.parents[2]
    harness = repo / 'tools/compact-rust-proof-smoke'
    scratch = args.scratch.resolve()
    if scratch.exists():
        raise SystemExit('scratch must be a new isolated directory; preserve earlier evidence')
    generated = args.generated_contract.resolve()
    if not (generated / 'lib.rs').is_file():
        raise SystemExit('missing unedited generated lib.rs')
    scratch.mkdir(parents=True, mode=0o700)
    shutil.copytree(harness / 'src', scratch / 'src')
    shutil.copy2(repo / 'Cargo.lock', scratch / 'Cargo.lock')
    # Preserve compile-time support/fixture references when relocating the
    # existing harness. Generated contract source is never rewritten.
    for copied in (scratch / 'src').rglob('*.rs'):
        original = harness / 'src' / copied.relative_to(scratch / 'src')
        source = re.sub(r'"(\.\./[^"]+)"',
                        lambda m: json.dumps(str((original.parent / m[1]).resolve())),
                        copied.read_text())
        copied.write_text(source)

    manifest = (harness / 'Cargo.toml').read_text()
    manifest = re.sub(r'path\s*=\s*"([^"]+)"',
                      lambda m: 'path = ' + json.dumps(str((harness / m[1]).resolve())), manifest)
    manifest = manifest.replace('name = "compact-rust-proof-smoke"', 'name = "compact-wallet-bootstrap-proof"', 1)
    dependency = 'compact-contract-shielded = { path = ' + json.dumps(str(generated)) + ', features = ["ledger-transaction"] }\n'
    manifest = manifest.replace('[dependencies]\n', '[dependencies]\n' + dependency, 1)
    (scratch / 'Cargo.toml').write_text(manifest + '\n[workspace]\n')
    shutil.copy2(here / 'shielded-bootstrap-proof.rs', scratch / 'src/wallet_bootstrap.rs')
    source = (scratch / 'src/main.rs').read_text()
    hook = '    let first = arguments.next();'
    if source.count(hook) != 1:
        raise SystemExit('proof harness argument hook changed; review the adapter')
    source = source.replace(hook, hook + '''
    if first.as_deref() == Some(OsStr::new("--wallet-bootstrap")) {
        let root=arguments.next().ok_or("expected bootstrap proof root")?;
        if arguments.next().is_some() { return Err("unexpected bootstrap argument".into()); }
        return wallet_bootstrap::run(Path::new(&root));
    }
''')
    module_hook = 'mod adt_list_bytes;'
    if source.count(module_hook) != 1:
        raise SystemExit('proof harness module hook changed; review the adapter')
    source = source.replace(module_hook, 'mod wallet_bootstrap;\n' + module_hook, 1)
    (scratch / 'src/main.rs').write_text(source)
    evidence = {name: hashlib.sha256((generated / name).read_bytes()).hexdigest()
                for name in ('Cargo.toml', 'lib.rs')}
    (scratch / 'generated-inputs.json').write_text(json.dumps(evidence, indent=2) + '\n')
    env = dict(os.environ, CARGO_INCREMENTAL='0', CARGO_TARGET_DIR=str(args.cargo_target_dir.resolve()))
    subprocess.run(['cargo', '+1.99.0', 'run', '--offline', '--manifest-path', str(scratch / 'Cargo.toml'),
                    '--', '--wallet-bootstrap', str(args.proof_root.resolve())], env=env, check=True)


if __name__ == '__main__':
    main()
