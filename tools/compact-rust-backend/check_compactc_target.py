#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# 	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Exercise the public compactc target boundary and a separate Rust consumer."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "examples/rust_backend/counter.compact"
PURE_SOURCE = ROOT / "examples/rust_backend/field_add.compact"


def run(*arguments: str, cwd: Path = ROOT) -> None:
    subprocess.run(arguments, cwd=cwd, check=True)


def check_manifest(output: Path) -> None:
    manifest = json.loads((output / "compiler/contract-manifest.json").read_text())
    for directory in ("compiler", "contract", "zkir"):
        assert directory in manifest, f"missing {directory} in contract manifest"
        for name, entry in manifest[directory].items():
            if name == "type":
                continue
            path = output / directory / name
            assert entry["type"] == "file", path
            content = path.read_bytes()
            assert entry["size"] == len(content), path
            assert entry["hash"] == hashlib.sha256(content).hexdigest(), path


def check_consumer(contract: Path, pure_contract: Path, consumer: Path) -> None:
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    pure_package = tomllib.loads((pure_contract / "Cargo.toml").read_text())
    runtime = package["dependencies"]["midnight-compact-runtime"]
    consumer.mkdir()
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-target-smoke\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))} }}\n'
        f'{pure_package["package"]["name"]} = {{ path = {json.dumps(str(pure_contract))} }}\n'
        "midnight-compact-runtime = { "
        f'git = {json.dumps(runtime["git"])}, '
        f'rev = {json.dumps(runtime["rev"])} }}\n'
    )
    (consumer / "tests/counter.rs").write_text(
        "use compact_contract_counter::ledger_contract::{initial_state, increment, read_round};\n"
        "use compact_contract_field_add::pure_circuits::field_add;\n"
        "use midnight_compact_runtime::context::ConstructorContext;\n"
        "use midnight_compact_runtime::ledger::ContractAddress;\n\n"
        "#[test]\nfn pure_circuit_runs_outside_the_compiler_workspace() {\n"
        "    let sum = field_add(2u64.into(), 3u64.into()).unwrap();\n"
        "    assert_eq!(sum, 5u64.into());\n}\n\n"
        "#[test]\nfn generated_contract_runs_outside_the_compiler_workspace() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let step = increment(context).unwrap();\n"
        "    let read = read_round(step.context).unwrap();\n"
        "    assert_eq!(read.result.value(), 1);\n}\n"
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--consumer", action="store_true", help="build and run a separate consumer")
    args = parser.parse_args()
    compiler = os.environ.get("COMPACTC", "compactc")
    with tempfile.TemporaryDirectory(prefix="compactc-target-") as temporary:
        base = Path(temporary)
        ts, rust, both, pure = (base / name for name in ("ts", "rust", "both", "pure"))
        run(compiler, "--skip-zk", str(SOURCE), str(ts))
        assert (ts / "contract/index.js").is_file()
        assert not (ts / "contract/lib.rs").exists()
        run(compiler, "--target", "rust", "--skip-zk", str(SOURCE), str(rust))
        assert (rust / "contract/lib.rs").is_file()
        assert (rust / "contract/Cargo.toml").is_file()
        assert not (rust / "contract/index.js").exists()
        check_manifest(rust)
        run(
            compiler, "--target=ts", "--target=rust", "--skip-zk",
            str(SOURCE), str(both),
        )
        assert (both / "contract/index.js").is_file()
        assert (both / "contract/lib.rs").is_file()
        check_manifest(both)
        if args.consumer:
            run(compiler, "--target", "rust", "--skip-zk", str(PURE_SOURCE), str(pure))
            check_consumer(rust / "contract", pure / "contract", base / "consumer")
    print("compactc target boundary and manifest: passed")


if __name__ == "__main__":
    main()
