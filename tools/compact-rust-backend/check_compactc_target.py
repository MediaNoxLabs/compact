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

"""Exercise compactc targets, a separate Rust consumer, proof, and deployment."""

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
    def check_tree(path: Path, entries: dict) -> None:
        for name, entry in entries.items():
            if name == "type":
                continue
            child = path / name
            if entry["type"] == "directory":
                assert child.is_dir(), child
                check_tree(child, entry)
            else:
                assert entry["type"] == "file", child
                content = child.read_bytes()
                assert entry["size"] == len(content), child
                assert entry["hash"] == hashlib.sha256(content).hexdigest(), child
    directories = ["compiler", "contract", "zkir"]
    if (output / "keys").is_dir():
        directories.append("keys")
    for directory in directories:
        assert directory in manifest, f"missing {directory} in contract manifest"
        check_tree(output / directory, manifest[directory])


def check_consumer(contract: Path, pure_contract: Path, consumer: Path) -> None:
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    runtime = package["dependencies"]["midnight-compact-runtime"]
    assert runtime["path"] == "runtime-rs"
    assert (contract / "runtime-rs/Cargo.toml").is_file()
    assert (contract / "runtime-rs-macros/Cargo.toml").is_file()
    consumer.mkdir()
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-target-smoke\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))} }}\n'
        f'midnight-compact-runtime = {{ path = {json.dumps(str(contract / runtime["path"]))} }}\n'
    )
    (consumer / "tests/counter.rs").write_text(
        "use compact_contract_counter::ledger_contract::{initial_state, recorded, Contract};\n"
        "use midnight_compact_runtime::context::ConstructorContext;\n"
        "use midnight_compact_runtime::ledger::ContractAddress;\n\n"
        "#[test]\nfn generated_contract_runs_outside_the_compiler_workspace() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let contract = Contract::default();\n"
        "    let step = contract.increment(context).unwrap();\n"
        "    let read = contract.read_round(step.context).unwrap();\n"
        "    assert_eq!(read.result.value(), 1);\n}\n"
        "\n#[test]\nfn generated_counter_trace_replays_outside_the_compiler_workspace() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let call = recorded::increment(context).unwrap();\n"
        "    let replay = call.public.initial().query(\n"
        "        call.public.verify_ops(), None, &call.execution.context.cost_model,\n"
        "    ).unwrap();\n"
        "    assert_eq!(replay.context.effects, call.execution.context.query.effects);\n}\n"
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)

    pure_package = tomllib.loads((pure_contract / "Cargo.toml").read_text())
    pure_consumer = consumer.parent / "pure-consumer"
    pure_consumer.mkdir()
    (pure_consumer / "tests").mkdir()
    (pure_consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-pure-target-smoke\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[dependencies]\n"
        f'{pure_package["package"]["name"]} = {{ path = {json.dumps(str(pure_contract))} }}\n'
    )
    (pure_consumer / "tests/pure.rs").write_text(
        "use compact_contract_field_add::pure_circuits::field_add;\n"
        "#[test]\nfn pure_circuit_runs_outside_the_compiler_workspace() {\n"
        "    let sum = field_add(2u64.into(), 3u64.into()).unwrap();\n"
        "    assert_eq!(sum, 5u64.into());\n}\n"
    )
    subprocess.run(["cargo", "test", "--quiet"], cwd=pure_consumer, env=environment, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--consumer", action="store_true", help="build and run a separate consumer")
    parser.add_argument("--proof", action="store_true", help="generate ZKIR and proving keys")
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
        protected = base / "protected"
        protected.mkdir()
        marker = protected / "keep.txt"
        marker.write_text("compiler output cleanup must not follow symlinks\n")
        nested_link = rust / "contract/linked"
        top_level_link = rust / "keys"
        nested_link.symlink_to(protected, target_is_directory=True)
        top_level_link.symlink_to(protected, target_is_directory=True)
        run(compiler, "--target", "rust", "--skip-zk", str(SOURCE), str(rust))
        assert marker.is_file(), "compiler output cleanup followed a directory symlink"
        assert not nested_link.is_symlink()
        assert not top_level_link.is_symlink()
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
        if args.proof:
            proof = base / "proof"
            run(compiler, "--target", "rust", str(SOURCE), str(proof))
            check_manifest(proof)
            for circuit in ("increment", "read_round"):
                for extension in ("prover", "verifier"):
                    assert (proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (proof / "zkir" / f"{circuit}.{extension}").is_file()
            run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", str(proof))
    print("compactc target boundary and manifest: passed")


if __name__ == "__main__":
    main()
