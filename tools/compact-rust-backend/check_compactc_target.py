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
CELL_SOURCE = ROOT / "examples/rust_backend/cell_boolean.compact"
CELL_READ_SOURCE = ROOT / "examples/rust_backend/cell_read.compact"
WITNESS_CELL_SOURCE = ROOT / "examples/rust_backend/witness_cell_write.compact"
NESTED_COUNTER_SOURCE = ROOT / "examples/rust_backend/stateful_circuit_call.compact"
NESTED_WITNESS_SOURCE = ROOT / "examples/rust_backend/nested_witness_call_oracle.compact"


def run(*arguments: str, cwd: Path = ROOT) -> None:
    subprocess.run(arguments, cwd=cwd, check=True)


def check_manifest(output: Path, *, require_zkir: bool = True) -> None:
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
    directories = ["compiler", "contract"]
    if require_zkir:
        directories.append("zkir")
    if (output / "keys").is_dir():
        directories.append("keys")
    for directory in directories:
        assert directory in manifest, f"missing {directory} in contract manifest"
        check_tree(output / directory, manifest[directory])


def check_consumer(contract: Path, pure_contract: Path, consumer: Path) -> None:
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    runtime = package["dependencies"]["midnight-compact-runtime"]
    assert runtime["path"] == "runtime-rs"
    assert package["features"]["ledger-transaction"] == ["midnight-compact-runtime/ledger-transaction"]
    assert (contract / "runtime-rs/Cargo.toml").is_file()
    assert (contract / "runtime-rs-macros/Cargo.toml").is_file()
    consumer.mkdir()
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-target-smoke\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/counter.rs").write_text(
        "use compact_contract_counter::ledger_contract::{initial_state, recorded, Contract};\n"
        "use compact_contract_counter::runtime::context::ConstructorContext;\n"
        "use compact_contract_counter::runtime::ledger::ContractAddress;\n"
        "use compact_contract_counter::runtime::transaction::CallSpec;\n\n"
        "#[test]\nfn generated_contract_runs_outside_the_compiler_workspace() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let contract = Contract::default();\n"
        "    let _call_spec_type = core::mem::size_of::<CallSpec>();\n"
        "    let step = contract.increment(context).unwrap();\n"
        "    let read = contract.read_round(step.context).unwrap();\n"
        "    assert_eq!(read.result.value(), 1);\n}\n"
        "\n#[test]\nfn generated_counter_slot_is_typed_and_executable() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let step = compact_contract_counter::ledger_slots::round.increment(context, 2).unwrap();\n"
        "    let read = compact_contract_counter::ledger_slots::round.read(step.context).unwrap();\n"
        "    assert_eq!(read.result, 2);\n}\n"
        "\n#[test]\nfn generated_counter_trace_replays_outside_the_compiler_workspace() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let call = Contract::default().recording.increment(context).unwrap();\n"
        "    let replay = call.public.initial().query(\n"
        "        call.public.verify_ops(), None, &call.execution.context.cost_model,\n"
        "    ).unwrap();\n"
        "    assert_eq!(replay.context.effects, call.execution.context.query.effects);\n"
        "    let read = recorded::read_round(call.execution.context).unwrap();\n"
        "    assert_eq!(read.execution.result.value(), 1);\n"
        "    let replay = read.public.initial().query(\n"
        "        read.public.verify_ops(), None, &read.execution.context.cost_model,\n"
        "    ).unwrap();\n"
        "    assert_eq!(replay.context.effects, read.execution.context.query.effects);\n}\n"
        "\n#[test]\nfn generated_counter_slot_records_a_replayable_call() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let frame = compact_contract_counter::runtime::recording::RecordingFrame::new(context);\n"
        "    let frame = compact_contract_counter::ledger_slots::round.record_increment(frame, 1).unwrap();\n"
        "    let call = frame.finish(());\n"
        "    let replay = call.public.initial().query(call.public.verify_ops(), None, &call.execution.context.cost_model).unwrap();\n"
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


def check_shared_runtime_consumer(compiler: str, base: Path) -> None:
    contracts = []
    for name, source in (("counter", SOURCE), ("field_add", PURE_SOURCE), ("cell_boolean", CELL_SOURCE)):
        output = base / f"shared-{name}"
        run(
            compiler, "--target", "rust", "--rust-runtime-root", str(ROOT),
            "--skip-zk", str(source), str(output),
        )
        check_manifest(output, require_zkir=name != "field_add")
        contract = output / "contract"
        manifest = tomllib.loads((contract / "Cargo.toml").read_text())
        assert Path(manifest["dependencies"]["midnight-compact-runtime"]["path"]) == ROOT / "runtime-rs"
        assert not (contract / "runtime-rs").exists()
        contracts.append((contract, manifest["package"]["name"]))

    consumer = base / "shared-consumer"
    consumer.mkdir()
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-shared-runtime-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        + "".join(
            f"{name} = {{ path = {json.dumps(str(contract))} }}\n"
            for contract, name in contracts
        )
        + f'midnight-compact-runtime = {{ path = {json.dumps(str(ROOT / "runtime-rs"))} }}\n'
    )
    (consumer / "tests/both.rs").write_text(
        "use compact_contract_counter::ledger_contract::{Contract, initial_state};\n"
        "use compact_contract_cell_boolean::ledger_contract::{Contract as CellContract, initial_state as initial_cell_state};\n"
        "use compact_contract_field_add::pure_circuits::field_add;\n"
        "use midnight_compact_runtime::context::ConstructorContext;\n"
        "use midnight_compact_runtime::ledger::ContractAddress;\n"
        "#[test]\nfn two_generated_contracts_share_one_runtime() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let result = Contract::default().increment(context).unwrap();\n"
        "    let read = Contract::default().read_round(result.context).unwrap();\n"
        "    assert_eq!(read.result.value(), 1);\n"
        "    let cell = initial_cell_state(ConstructorContext::new(())).unwrap();\n"
        "    let cell_context = cell.into_circuit_context(ContractAddress::default());\n"
        "    let _: midnight_compact_runtime::slots::CellSlot<bool> = compact_contract_cell_boolean::ledger_slots::flag;\n"
        "    let cell_call = CellContract::default().recording.set_flag(cell_context).unwrap();\n"
        "    let cell_replay = cell_call.public.initial().query(\n"
        "        cell_call.public.verify_ops(), None, &cell_call.execution.context.cost_model,\n"
        "    ).unwrap();\n"
        "    assert_eq!(cell_replay.context.effects, cell_call.execution.context.query.effects);\n"
        "    assert_eq!(field_add(2u64.into(), 3u64.into()).unwrap(), 5u64.into());\n"
        "}\n"
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)


def check_witness_consumer(compiler: str, base: Path) -> None:
    output = base / "witness-contract"
    run(compiler, "--target", "rust", "--skip-zk", str(WITNESS_CELL_SOURCE), str(output))
    check_manifest(output)
    contract = output / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "witness-consumer"
    consumer.mkdir()
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-witness-target-smoke\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/witness.rs").write_text(
        "use compact_contract_witness_cell_write::ledger_contract::{Contract, LedgerView, Witnesses, initial_state};\n"
        "use compact_contract_witness_cell_write::runtime::Field;\n"
        "use compact_contract_witness_cell_write::runtime::context::{ConstructorContext, WitnessContext};\n"
        "use compact_contract_witness_cell_write::runtime::ledger::ContractAddress;\n"
        "use compact_contract_witness_cell_write::runtime::transaction::CallSpec;\n"
        "struct Secret;\n"
        "impl Witnesses<u64> for Secret {\n"
        "    fn secret(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, seed: Field) -> (u64, Field) {\n"
        "        let private = *context.private_state;\n"
        "        let cell = context.ledger.cell().unwrap();\n"
        "        assert_eq!(cell, Field::from(if private == 7 { 0_u64 } else { 9_u64 }));\n"
        "        (private + 1, seed + Field::from(private))\n"
        "    }\n}\n"
        "#[test]\nfn generated_witness_recording_works_with_one_crate_dependency() {\n"
        "    let _call_spec_type = core::mem::size_of::<CallSpec>();\n"
        "    let state = initial_state(ConstructorContext::new(7_u64)).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let contract = Contract::from(Secret);\n"
        "    let call = contract.recording().write_twice(context, Field::from(2_u64)).unwrap();\n"
        "    assert_eq!(call.execution.context.private_state, 9);\n"
        "    assert_eq!(call.execution.private_transcript_outputs.len(), 2);\n"
        "    let replay = call.public.initial().query(\n"
        "        call.public.verify_ops(), None, &call.execution.context.cost_model,\n"
        "    ).unwrap();\n"
        "    assert_eq!(replay.context.effects, call.execution.context.query.effects);\n"
        "    let read = contract.recording().read_cell(call.execution.context).unwrap();\n"
        "    assert_eq!(read.execution.result, Field::from(10_u64));\n"
        "}\n"
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)


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
            check_shared_runtime_consumer(compiler, base)
            check_witness_consumer(compiler, base)
        if args.proof:
            proof = base / "proof"
            run(compiler, "--target", "rust", str(SOURCE), str(proof))
            check_manifest(proof)
            for circuit in ("increment", "read_round"):
                for extension in ("prover", "verifier"):
                    assert (proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (proof / "zkir" / f"{circuit}.{extension}").is_file()
            cell_proof = base / "cell-proof"
            run(compiler, "--target", "rust", str(CELL_SOURCE), str(cell_proof))
            check_manifest(cell_proof)
            for extension in ("prover", "verifier"):
                assert (cell_proof / "keys" / f"set_flag.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (cell_proof / "zkir" / f"set_flag.{extension}").is_file()
            cell_read_proof = base / "cell-read-proof"
            run(compiler, "--target", "rust", str(CELL_READ_SOURCE), str(cell_read_proof))
            check_manifest(cell_read_proof)
            for extension in ("prover", "verifier"):
                assert (cell_read_proof / "keys" / f"read_flag.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (cell_read_proof / "zkir" / f"read_flag.{extension}").is_file()
            witness_proof = base / "witness-proof"
            run(compiler, "--target", "rust", str(WITNESS_CELL_SOURCE), str(witness_proof))
            check_manifest(witness_proof)
            for circuit in ("write_twice", "write_nested_twice"):
                for extension in ("prover", "verifier"):
                    assert (witness_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (witness_proof / "zkir" / f"{circuit}.{extension}").is_file()
            nested_proof = base / "nested-proof"
            run(compiler, "--target", "rust", str(NESTED_COUNTER_SOURCE), str(nested_proof))
            check_manifest(nested_proof)
            for circuit in ("bump_twice", "add_twice"):
                for extension in ("prover", "verifier"):
                    assert (nested_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (nested_proof / "zkir" / f"{circuit}.{extension}").is_file()
            nested_witness_proof = base / "nested-witness-proof"
            run(compiler, "--target", "rust", str(NESTED_WITNESS_SOURCE), str(nested_witness_proof))
            check_manifest(nested_witness_proof)
            for circuit in ("outer", "outerValue"):
                for extension in ("prover", "verifier"):
                    assert (nested_witness_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (nested_witness_proof / "zkir" / f"{circuit}.{extension}").is_file()
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                str(proof), str(cell_proof), str(cell_read_proof), str(witness_proof), str(nested_proof),
                str(nested_witness_proof),
            )
    print("compactc target boundary and manifest: passed")


if __name__ == "__main__":
    main()
