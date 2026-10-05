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
PM_19252_SOURCE = ROOT / "examples/bugs/pm-19252/example_ten.compact"
NATIVE_KEY_VALUE_SOURCE = ROOT / "examples/rust_backend/native_own_public_key_value.compact"
COUNTER_PARAMETER_SOURCE = ROOT / "examples/rust_backend/counter_parameter.compact"
BOUNDED_UINT_SOURCE = ROOT / "examples/rust_backend/bounded_uint_oracle.compact"
UINTS_SOURCE = ROOT / "examples/rust_backend/uints_oracle.compact"
CROSS_CIRCUIT_SOURCE = ROOT / "examples/rust_backend/cross_circuit_oracle.compact"
WIDE_UINT_SOURCE = ROOT / "examples/rust_backend/wide_uint_oracle.compact"
BUG11_SOURCE = ROOT / "examples/rust_backend/bug11_oracle.compact"
MULTI_PL_CALL_SOURCE = ROOT / "examples/rust_backend/multi_pl_call_oracle.compact"
PURE_SOURCE = ROOT / "examples/rust_backend/field_add.compact"
CELL_SOURCE = ROOT / "examples/rust_backend/cell_boolean.compact"
CELL_STRUCT_SOURCE = ROOT / "examples/rust_backend/cell_struct.compact"
SET_SOURCE = ROOT / "examples/rust_backend/set_oracle.compact"
SET_BOOLEAN_SOURCE = ROOT / "examples/rust_backend/set_boolean.compact"
MAP_BOOLEAN_SOURCE = ROOT / "examples/rust_backend/map_boolean_field.compact"
NESTED_MAP_SOURCE = ROOT / "examples/rust_backend/nested_map_oracle.compact"
NESTED_MAP_SHAPE_SOURCE = ROOT / "examples/rust_backend/nested_map_shape.compact"
CONSTRUCTOR_MAP_SOURCE = ROOT / "examples/rust_backend/constructor_map_actions.compact"
LIST_SOURCE = ROOT / "examples/rust_backend/list_field.compact"
MERKLE_SOURCE = ROOT / "examples/rust_backend/merkle_tree_oracle.compact"
MERKLE_VERIFY_SOURCE = ROOT / "examples/rust_backend/merkle_path_verify.compact"
PERSISTENT_COMMIT_SOURCE = ROOT / "examples/rust_backend/call_arg_declared_type.compact"
INTERNAL_PURE_CALL_SOURCE = ROOT / "examples/rust_backend/internal_pure_call.compact"
STATEFUL_PURE_CALL_SOURCE = ROOT / "examples/rust_backend/stateful_pure_call.compact"
TERNARY_COND_SOURCE = ROOT / "examples/rust_backend/ternary_cond_oracle.compact"
ASSET_REGISTRY_SOURCE = ROOT / "examples/rust_backend/asset_registry_oracle.compact"
HISTORIC_MERKLE_SOURCE = ROOT / "examples/rust_backend/hmt_insert_oracle.compact"
VECTOR_KEY_SOURCE = ROOT / "examples/rust_backend/vector_key_adt.compact"
COMPOSITE_KEY_SOURCE = ROOT / "examples/rust_backend/observed_composite_keys.compact"
CHUNKED_SET_SOURCE = ROOT / "examples/rust_backend/chunked_set_observed.compact"
CHUNKED_LIST_SOURCE = ROOT / "examples/rust_backend/chunked_list.compact"
CHUNKED_MAP_SOURCE = ROOT / "examples/rust_backend/chunked_map.compact"
CHUNKED_CELL_SOURCE = ROOT / "examples/rust_backend/chunked_cell.compact"
CONSTRUCTOR_LIST_SOURCE = ROOT / "examples/rust_backend/constructor_list_actions.compact"
RECORDED_ENUM_SOURCE = ROOT / "examples/rust_backend/recorded_enum_cell.compact"
TINY_SOURCE = ROOT / "examples/rust_backend/tiny_oracle.compact"
CELL_READ_SOURCE = ROOT / "examples/rust_backend/cell_read.compact"
WITNESS_CELL_SOURCE = ROOT / "examples/rust_backend/witness_cell_write.compact"
ASSERT_WITNESS_SOURCE = ROOT / "examples/rust_backend/assert_witness.compact"
MERKLE_WITNESS_SOURCE = ROOT / "examples/rust_backend/merkle_path_witness.compact"
LIST_SHAPES_SOURCE = ROOT / "examples/rust_backend/witness_list_shapes.compact"
NESTED_COUNTER_SOURCE = ROOT / "examples/rust_backend/stateful_circuit_call.compact"
NESTED_WITNESS_SOURCE = ROOT / "examples/rust_backend/nested_witness_call_oracle.compact"
ALIAS_SOURCE = ROOT / "examples/rust_backend/aliases_oracle.compact"


def run(*arguments: str, cwd: Path = ROOT) -> None:
    subprocess.run(arguments, cwd=cwd, check=True)


def check_manifest(output: Path, *, require_zkir: bool = True) -> None:
    manifest = json.loads((output / "compiler/contract-manifest.json").read_text())
    if (output / "contract/lib.rs").is_file():
        assert (output / "contract/rust-capabilities.json").is_file()
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


def check_observed_call_consumer(proof: Path, base: Path) -> None:
    """Compile and prepare a call with only the generated crate as a dependency."""
    contract = proof / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "observed-call-consumer"
    (consumer / "tests").mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-observed-call-smoke\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/observed.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/observed_call.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    environment["COMPACT_RUST_OBSERVED_STATE"] = str(
        ROOT / "tools/compact-rust-proof-smoke/tests/fixtures/confirmed-counter-state-abi18.bin"
    )
    environment["COMPACT_RUST_OBSERVED_VERIFIER"] = str(proof / "keys/increment.verifier")
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)


def check_observed_map_call_consumer(proof: Path, base: Path) -> None:
    """Invoke the two-argument generated method with no direct runtime dependency."""
    contract = proof / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "observed-map-call-consumer"
    (consumer / "tests").mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-observed-map-call-smoke\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/observed.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/observed_map_call.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    environment["COMPACT_RUST_OBSERVED_MAP_VERIFIER"] = str(proof / "keys/put.verifier")
    environment["COMPACT_RUST_OBSERVED_MAP_PAIR_VERIFIER"] = str(proof / "keys/put_pair.verifier")
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_put_arguments.rs").write_text(
        "use compact_contract_map_boolean_field::ledger_contract::Contract;\n"
        "use compact_contract_map_boolean_field::runtime::{Field, transaction::ObservedContractState};\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.put_call(observed, (), Field::from(1_u64), true);\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_put_arguments"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "swapped put arguments unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr
    assert "expected `bool`" in rejected.stderr, rejected.stderr
    (consumer / "examples/wrong_put_pair_arguments.rs").write_text(
        "use compact_contract_map_boolean_field::ledger_contract::Contract;\n"
        "use compact_contract_map_boolean_field::runtime::{Field, transaction::ObservedContractState};\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.put_pair_call(observed, (), true, Field::from(1_u64), false);\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_put_pair_arguments"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed put_pair arguments unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr
    assert "expected `bool`" in rejected.stderr, rejected.stderr


def check_observed_witness_call_consumer(proof: Path, base: Path) -> None:
    """Compile and run a borrowed two-argument witness call from one dependency."""
    contract = proof / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "observed-witness-call-consumer"
    (consumer / "tests").mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-observed-witness-call-smoke\"\nversion = \"0.1.0\"\n"
        "edition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/observed.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/witnessed_two_argument_call.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    environment["COMPACT_RUST_WITNESS_OFFSET_VERIFIER"] = str(proof / "keys/write_offset.verifier")
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)


def check_observed_composite_key_consumer(proof: Path, base: Path) -> None:
    """Type-check nested input methods through only the generated crate."""
    contract = proof / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "observed-composite-key-consumer"
    (consumer / "tests").mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-observed-composite-key-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/observed.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/observed_composite_keys.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    environment["COMPACT_RUST_COMPOSITE_KEY_PROOF"] = str(proof)
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_tuple_key.rs").write_text(
        "use compact_contract_observed_composite_keys::ledger_contract::Contract;\n"
        "use compact_contract_observed_composite_keys::runtime::{Field, transaction::ObservedContractState};\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.insert_tuple_call(observed, (), (true, Field::from(42_u64)));\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_tuple_key"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed tuple key unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr


def check_chunked_set_consumer(proof: Path, base: Path) -> None:
    """Build the physical-path Set API through the generated crate alone."""
    contract = proof / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "chunked-set-consumer"
    (consumer / "tests").mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-chunked-set-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/observed.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/chunked_set_observed.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    environment["COMPACT_RUST_CHUNKED_SET_PROOF"] = str(proof)
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_key.rs").write_text(
        "use compact_contract_chunked_set_observed::ledger_contract::Contract;\n"
        "use compact_contract_chunked_set_observed::runtime::{Field, transaction::ObservedContractState};\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.insert_key_call(observed, (), (true, Field::from(42_u64)));\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_key"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed chunked Set key unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr


def check_chunked_list_consumer(proof: Path, base: Path) -> None:
    """Build all List operations through the pathful generated API alone."""
    contract = proof / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "chunked-list-consumer"
    (consumer / "tests").mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-chunked-list-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/observed.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/chunked_list.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    environment["COMPACT_RUST_CHUNKED_LIST_PROOF"] = str(proof)
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_value.rs").write_text(
        "use compact_contract_chunked_list::ledger_contract::Contract;\n"
        "use compact_contract_chunked_list::runtime::transaction::ObservedContractState;\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.prepend_call(observed, (), true);\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_value"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed chunked List value unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr


def check_chunked_map_consumer(proof: Path, base: Path) -> None:
    """Build pathful typed Map calls through the generated crate alone."""
    contract = proof / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "chunked-map-consumer"
    (consumer / "tests").mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-chunked-map-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/observed.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/chunked_map.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    environment["COMPACT_RUST_CHUNKED_MAP_PROOF"] = str(proof)
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_key.rs").write_text(
        "use compact_contract_chunked_map::ledger_contract::Contract;\n"
        "use compact_contract_chunked_map::runtime::{Field, transaction::ObservedContractState};\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.put_call(observed, (), Field::from(1_u64), Field::from(7_u64));\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_key"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed chunked Map key unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr
    (consumer / "examples/wrong_value.rs").write_text(
        "use compact_contract_chunked_map::ledger_contract::Contract;\n"
        "use compact_contract_chunked_map::runtime::transaction::ObservedContractState;\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.put_call(observed, (), true, false);\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_value"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed chunked Map value unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr


def check_chunked_cell_consumer(proof: Path, base: Path) -> None:
    """Build pathful typed Cell calls through the generated crate alone."""
    contract = proof / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "chunked-cell-consumer"
    (consumer / "tests").mkdir(parents=True)
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-chunked-cell-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))}, features = ["ledger-transaction"] }}\n'
    )
    (consumer / "tests/observed.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/chunked_cell.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    environment["COMPACT_RUST_CHUNKED_CELL_PROOF"] = str(proof)
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_bool.rs").write_text(
        "use compact_contract_chunked_cell::ledger_contract::Contract;\n"
        "use compact_contract_chunked_cell::runtime::{Field, transaction::ObservedContractState};\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.set_active_call(observed, (), Field::from(1_u64));\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_bool"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed chunked Cell Boolean unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr
    (consumer / "examples/wrong_field.rs").write_text(
        "use compact_contract_chunked_cell::ledger_contract::Contract;\n"
        "use compact_contract_chunked_cell::runtime::transaction::ObservedContractState;\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.set_amount_call(observed, (), true);\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_field"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed chunked Cell Field unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr
    (consumer / "examples/wrong_equals.rs").write_text(
        "use compact_contract_chunked_cell::ledger_contract::Contract;\n"
        "use compact_contract_chunked_cell::runtime::{Field, transaction::ObservedContractState};\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.active_equals_call(observed, (), Field::from(1_u64));\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_equals"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed Cell equality unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr
    (consumer / "examples/wrong_sum.rs").write_text(
        "use compact_contract_chunked_cell::ledger_contract::Contract;\n"
        "use compact_contract_chunked_cell::runtime::transaction::ObservedContractState;\n"
        "fn wrong(observed: &ObservedContractState) {\n"
        "    let _ = Contract::default().recording.plus_amount_call(observed, (), true);\n"
        "}\nfn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_sum"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong-typed Cell sum unexpectedly compiled"
    assert "error[E0308]" in rejected.stderr, rejected.stderr
    for name in ("subtract_amount", "multiply_amount"):
        (consumer / "examples" / f"wrong_{name}.rs").write_text(
            "use compact_contract_chunked_cell::ledger_contract::Contract;\n"
            "use compact_contract_chunked_cell::runtime::transaction::ObservedContractState;\n"
            "fn wrong(observed: &ObservedContractState) {\n"
            f"    let _ = Contract::default().recording.{name}_call(observed, (), true);\n"
            "}\nfn main() {}\n"
        )
        rejected = subprocess.run(
            ["cargo", "check", "--quiet", "--example", f"wrong_{name}"],
            cwd=consumer, env=environment, capture_output=True, text=True,
        )
        assert rejected.returncode != 0, f"wrong-typed {name} unexpectedly compiled"
        assert "error[E0308]" in rejected.stderr, rejected.stderr


def check_shared_runtime_consumer(compiler: str, base: Path) -> None:
    contracts = []
    for name, source in (
        ("counter", SOURCE), ("field_add", PURE_SOURCE),
        ("cell_boolean", CELL_SOURCE), ("set_oracle", SET_SOURCE),
        ("map_boolean_field", MAP_BOOLEAN_SOURCE),
        ("nested_map_oracle", NESTED_MAP_SOURCE),
        ("list_field", LIST_SOURCE),
        ("merkle_tree_oracle", MERKLE_SOURCE),
        ("hmt_insert_oracle", HISTORIC_MERKLE_SOURCE),
        ("vector_key_adt", VECTOR_KEY_SOURCE),
    ):
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
        "use compact_contract_set_oracle::ledger_contract::{Contract as SetContract, initial_state as initial_set_state};\n"
        "use compact_contract_map_boolean_field::ledger_contract::{Contract as MapContract, initial_state as initial_map_state};\n"
        "use compact_contract_nested_map_oracle::ledger_contract::initial_state as initial_nested_map_state;\n"
        "use compact_contract_list_field::ledger_contract::{Contract as ListContract, initial_state as initial_list_state};\n"
        "use compact_contract_merkle_tree_oracle::ledger_contract::initial_state as initial_merkle_state;\n"
        "use compact_contract_hmt_insert_oracle::ledger_contract::initial_state as initial_historic_merkle_state;\n"
        "use compact_contract_vector_key_adt::ledger_contract::{Contract as VectorContract, initial_state as initial_vector_state};\n"
        "use midnight_compact_runtime::context::ConstructorContext;\n"
        "use midnight_compact_runtime::ledger::ContractAddress;\n"
        "use midnight_compact_runtime::Field;\n"
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
        "    let set = initial_set_state(ConstructorContext::new(())).unwrap();\n"
        "    let set_context = set.into_circuit_context(ContractAddress::default());\n"
        "    let set_step = compact_contract_set_oracle::ledger_slots::s.insert(set_context, Field::from(7_u64)).unwrap();\n"
        "    let member = compact_contract_set_oracle::ledger_slots::s.member(set_step.context, Field::from(7_u64)).unwrap();\n"
        "    assert!(member.result);\n"
        "    let checked = SetContract::default().check(member.context, Field::from(7_u64)).unwrap();\n"
        "    assert!(compact_contract_set_oracle::runtime::ledger::read_root_cell::<bool, _>(checked.context.query.state.get_ref(), 0).unwrap());\n"
        "    let map = initial_map_state(ConstructorContext::new(())).unwrap();\n"
        "    let map_context = map.into_circuit_context(ContractAddress::default());\n"
        "    let _: midnight_compact_runtime::slots::MapSlot<bool, Field> = compact_contract_map_boolean_field::ledger_slots::table;\n"
        "    let map_call = MapContract::default().recording.put(map_context, true, Field::from(9_u64)).unwrap();\n"
        "    let map_replay = map_call.public.initial().query(\n"
        "        map_call.public.verify_ops(), None, &map_call.execution.context.cost_model,\n"
        "    ).unwrap();\n"
        "    assert_eq!(map_replay.context.effects, map_call.execution.context.query.effects);\n"
        "    let lookup = MapContract::default().recording.get(map_call.execution.context, true).unwrap();\n"
        "    assert_eq!(lookup.execution.result, Field::from(9_u64));\n"
        "    let nested = initial_nested_map_state(ConstructorContext::new(())).unwrap();\n"
        "    let nested_context = nested.into_circuit_context(ContractAddress::default());\n"
        "    let _: midnight_compact_runtime::slots::MapSlot<Field, midnight_compact_runtime::slots::MapNode<Field, midnight_compact_runtime::BoundedUint<18446744073709551615>>> = compact_contract_nested_map_oracle::ledger_slots::users_by_org;\n"
        "    let nested_empty = compact_contract_nested_map_oracle::ledger_slots::users_by_org.is_empty(nested_context).unwrap();\n"
        "    assert!(nested_empty.result);\n"
        "    let list = initial_list_state(ConstructorContext::new(())).unwrap();\n"
        "    let list_context = list.into_circuit_context(ContractAddress::default());\n"
        "    let _: midnight_compact_runtime::slots::ListSlot<Field> = compact_contract_list_field::ledger_slots::items;\n"
        "    let list_call = ListContract::default().recording.prepend(list_context, Field::from(5_u64)).unwrap();\n"
        "    let list_replay = list_call.public.initial().query(\n"
        "        list_call.public.verify_ops(), None, &list_call.execution.context.cost_model,\n"
        "    ).unwrap();\n"
        "    assert_eq!(list_replay.context.effects, list_call.execution.context.query.effects);\n"
        "    let first = ListContract::default().recording.first_item(list_call.execution.context).unwrap();\n"
        "    assert!(first.execution.result.is_some);\n"
        "    assert_eq!(first.execution.result.value, Field::from(5_u64));\n"
        "    let merkle = initial_merkle_state(ConstructorContext::new(())).unwrap();\n"
        "    let merkle_context = merkle.into_circuit_context(ContractAddress::default());\n"
        "    let _: midnight_compact_runtime::slots::MerkleSlot<midnight_compact_runtime::BoundedUint<255>, 3, false> = compact_contract_merkle_tree_oracle::ledger_slots::t;\n"
        "    let merkle_insert = compact_contract_merkle_tree_oracle::ledger_slots::t.insert(merkle_context, midnight_compact_runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();\n"
        "    let merkle_full = compact_contract_merkle_tree_oracle::ledger_slots::t.is_full(merkle_insert.context).unwrap();\n"
        "    assert!(!merkle_full.result);\n"
        "    let merkle = initial_merkle_state(ConstructorContext::new(())).unwrap();\n"
        "    let merkle_context = merkle.into_circuit_context(ContractAddress::default());\n"
        "    let recorded_merkle = compact_contract_merkle_tree_oracle::ledger_contract::recorded::append(merkle_context, midnight_compact_runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();\n"
        "    let merkle_replay = recorded_merkle.public.initial().query(recorded_merkle.public.verify_ops(), None, &recorded_merkle.execution.context.cost_model).unwrap();\n"
        "    assert_eq!(merkle_replay.context.effects, recorded_merkle.execution.context.query.effects);\n"
        "    let recorded_merkle_full = compact_contract_merkle_tree_oracle::ledger_contract::recorded::full(recorded_merkle.execution.context).unwrap();\n"
        "    assert!(!recorded_merkle_full.execution.result);\n"
        "    let merkle_full_replay = recorded_merkle_full.public.initial().query(recorded_merkle_full.public.verify_ops(), None, &recorded_merkle_full.execution.context.cost_model).unwrap();\n"
        "    assert_eq!(merkle_full_replay.context.effects, recorded_merkle_full.execution.context.query.effects);\n"
        "    let historic = initial_historic_merkle_state(ConstructorContext::new(())).unwrap();\n"
        "    let historic_context = historic.into_circuit_context(ContractAddress::default());\n"
        "    let _: midnight_compact_runtime::slots::MerkleSlot<midnight_compact_runtime::BoundedUint<255>, 3, true> = compact_contract_hmt_insert_oracle::ledger_slots::t;\n"
        "    let historic_insert = compact_contract_hmt_insert_oracle::ledger_slots::t.insert(historic_context, midnight_compact_runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();\n"
        "    let _historic_reset = compact_contract_hmt_insert_oracle::ledger_slots::t.reset_history(historic_insert.context).unwrap();\n"
        "    let historic = initial_historic_merkle_state(ConstructorContext::new(())).unwrap();\n"
        "    let historic_context = historic.into_circuit_context(ContractAddress::default());\n"
        "    let recorded_historic = compact_contract_hmt_insert_oracle::ledger_contract::recorded::append(historic_context, midnight_compact_runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();\n"
        "    let historic_replay = recorded_historic.public.initial().query(recorded_historic.public.verify_ops(), None, &recorded_historic.execution.context.cost_model).unwrap();\n"
        "    assert_eq!(historic_replay.context.effects, recorded_historic.execution.context.query.effects);\n"
        "    let recorded_historic_full = compact_contract_hmt_insert_oracle::ledger_contract::recorded::full(recorded_historic.execution.context).unwrap();\n"
        "    assert!(!recorded_historic_full.execution.result);\n"
        "    let historic_full_replay = recorded_historic_full.public.initial().query(recorded_historic_full.public.verify_ops(), None, &recorded_historic_full.execution.context.cost_model).unwrap();\n"
        "    assert_eq!(historic_full_replay.context.effects, recorded_historic_full.execution.context.query.effects);\n"
        "    let vector = initial_vector_state(ConstructorContext::new(())).unwrap();\n"
        "    let vector_context = vector.into_circuit_context(ContractAddress::default());\n"
        "    let vector_call = VectorContract::default().recording.setInsert(vector_context).unwrap();\n"
        "    let vector_replay = vector_call.public.initial().query(vector_call.public.verify_ops(), None, &vector_call.execution.context.cost_model).unwrap();\n"
        "    assert_eq!(vector_replay.context.effects, vector_call.execution.context.query.effects);\n"
        "    let vector_member = VectorContract::default().recording.setMember(vector_call.execution.context).unwrap();\n"
        "    assert!(midnight_compact_runtime::ledger::read_root_cell::<bool, _>(vector_member.execution.context.query.state.get_ref(), 2).unwrap());\n"
        "    assert_eq!(field_add(2u64.into(), 3u64.into()).unwrap(), 5u64.into());\n"
        "}\n"
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)

    # Check the generated slot type from a separate consumer crate.
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_cell_value.rs").write_text(
        "use compact_contract_cell_boolean::ledger_contract::initial_state;\n"
        "use compact_contract_cell_boolean::runtime::context::ConstructorContext;\n"
        "use compact_contract_cell_boolean::runtime::ledger::ContractAddress;\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = compact_contract_cell_boolean::ledger_slots::flag.write(context, 7_u64);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_cell_value"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Cell value type unexpectedly compiled"
    assert "expected `bool`, found `u64`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_set_element.rs").write_text(
        "use compact_contract_set_oracle::ledger_contract::initial_state;\n"
        "use compact_contract_set_oracle::runtime::context::ConstructorContext;\n"
        "use compact_contract_set_oracle::runtime::ledger::ContractAddress;\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = compact_contract_set_oracle::ledger_slots::s.insert(context, true);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_set_element"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Set element type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "found `bool`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_vector_key.rs").write_text(
        "use compact_contract_vector_key_adt::ledger_contract::initial_state;\n"
        "use compact_contract_vector_key_adt::runtime::{context::ConstructorContext, ledger::ContractAddress, recording::RecordingFrame};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let frame = RecordingFrame::new(state.into_circuit_context(ContractAddress::default()));\n"
        "    let _ = compact_contract_vector_key_adt::ledger_slots::keys.record_insert(frame, [0u8, 1u8]);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_vector_key"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "raw-byte vector key unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "FixedVector" in rejected.stderr, rejected.stderr

    (consumer / "examples/nested_map_scalar_lookup.rs").write_text(
        "use compact_contract_nested_map_oracle::ledger_contract::initial_state;\n"
        "use compact_contract_nested_map_oracle::runtime::{Field, context::ConstructorContext, ledger::ContractAddress};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = compact_contract_nested_map_oracle::ledger_slots::users_by_org.lookup(context, Field::from(1_u64));\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "nested_map_scalar_lookup"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "nested Map scalar lookup unexpectedly compiled"
    assert "lookup" in rejected.stderr and "MapNode" in rejected.stderr, rejected.stderr


    (consumer / "examples/wrong_list_element.rs").write_text(
        "use compact_contract_list_field::ledger_contract::initial_state;\n"
        "use compact_contract_list_field::runtime::{context::ConstructorContext, ledger::ContractAddress};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = compact_contract_list_field::ledger_slots::items.push_front(context, true);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_list_element"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong List element type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "found `bool`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_map_key.rs").write_text(
        "use compact_contract_map_boolean_field::ledger_contract::initial_state;\n"
        "use compact_contract_map_boolean_field::runtime::{Field, context::ConstructorContext, ledger::ContractAddress};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = compact_contract_map_boolean_field::ledger_slots::table.insert(context, Field::from(1_u64), Field::from(2_u64));\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_map_key"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Map key type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "expected `bool`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_map_value.rs").write_text(
        "use compact_contract_map_boolean_field::ledger_contract::initial_state;\n"
        "use compact_contract_map_boolean_field::runtime::{context::ConstructorContext, ledger::ContractAddress};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = compact_contract_map_boolean_field::ledger_slots::table.insert(context, true, false);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_map_value"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Map value type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "found `bool`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_merkle_leaf.rs").write_text(
        "use compact_contract_merkle_tree_oracle::ledger_contract::initial_state;\n"
        "use compact_contract_merkle_tree_oracle::runtime::{context::ConstructorContext, ledger::ContractAddress};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = compact_contract_merkle_tree_oracle::ledger_slots::t.insert(context, true);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_merkle_leaf"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong Merkle leaf type unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "expected `BoundedUint<255>`" in rejected.stderr, rejected.stderr

    (consumer / "examples/unsupported_recorded_merkle_index.rs").write_text(
        "use compact_contract_merkle_tree_oracle::ledger_contract::recorded;\n"
        "fn main() { let _ = recorded::place::<()>; }\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "unsupported_recorded_merkle_index"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "unsupported indexed Merkle trace unexpectedly compiled"
    assert "cannot find value `place` in module `recorded`" in rejected.stderr, rejected.stderr

    (consumer / "examples/unsupported_recorded_historic_index.rs").write_text(
        "use compact_contract_hmt_insert_oracle::ledger_contract::recorded;\n"
        "fn main() { let _ = recorded::place::<()>; }\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "unsupported_recorded_historic_index"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "unsupported historic indexed trace unexpectedly compiled"
    assert "cannot find value `place` in module `recorded`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_recorded_historic_leaf.rs").write_text(
        "use compact_contract_hmt_insert_oracle::ledger_contract::{initial_state, recorded};\n"
        "use compact_contract_hmt_insert_oracle::runtime::{context::ConstructorContext, ledger::ContractAddress};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = recorded::append(context, true);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_recorded_historic_leaf"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong recorded historic leaf unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "expected `BoundedUint<255>`" in rejected.stderr, rejected.stderr

    (consumer / "examples/unsupported_recorded_history_reset.rs").write_text(
        "use compact_contract_hmt_insert_oracle::ledger_contract::recorded;\n"
        "fn main() { let _ = recorded::forget_history::<()>; }\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "unsupported_recorded_history_reset"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "unsupported history reset trace unexpectedly compiled"
    assert "cannot find value `forget_history` in module `recorded`" in rejected.stderr, rejected.stderr

    (consumer / "examples/wrong_recorded_merkle_leaf.rs").write_text(
        "use compact_contract_merkle_tree_oracle::ledger_contract::{initial_state, recorded};\n"
        "use compact_contract_merkle_tree_oracle::runtime::{context::ConstructorContext, ledger::ContractAddress};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = recorded::append(context, true);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_recorded_merkle_leaf"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "wrong recorded Merkle leaf unexpectedly compiled"
    assert "error[E0308]: mismatched types" in rejected.stderr, rejected.stderr
    assert "expected `BoundedUint<255>`" in rejected.stderr, rejected.stderr

    (consumer / "examples/plain_merkle_reset_history.rs").write_text(
        "use compact_contract_merkle_tree_oracle::ledger_contract::initial_state;\n"
        "use compact_contract_merkle_tree_oracle::runtime::{context::ConstructorContext, ledger::ContractAddress};\n"
        "fn main() {\n"
        "    let state = initial_state(ConstructorContext::new(())).unwrap();\n"
        "    let context = state.into_circuit_context(ContractAddress::default());\n"
        "    let _ = compact_contract_merkle_tree_oracle::ledger_slots::t.reset_history(context);\n"
        "}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "plain_merkle_reset_history"],
        cwd=consumer, env=environment, capture_output=True, text=True,
    )
    assert rejected.returncode != 0, "plain Merkle history reset unexpectedly compiled"
    assert "error[E0599]" in rejected.stderr, rejected.stderr
    assert "MerkleSlot<T, DEPTH, true>" in rejected.stderr, rejected.stderr


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
        "use compact_contract_witness_cell_write::ledger_contract::{Contract, LedgerView, TryWitnesses, Witnesses, initial_state};\n"
        "use compact_contract_witness_cell_write::runtime::{CompactError, Field};\n"
        "use compact_contract_witness_cell_write::runtime::context::{ConstructorContext, RunningCost, WitnessContext};\n"
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
        "struct FallibleSecret;\n"
        "impl TryWitnesses<u64> for FallibleSecret {\n"
        "    fn secret(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, seed: Field) -> Result<(u64, Field), CompactError> {\n"
        "        let private = *context.private_state;\n"
        "        let cell = context.ledger.cell()?;\n"
        "        assert_eq!(cell, Field::from(if private == 7 { 0_u64 } else { 9_u64 }));\n"
        "        Ok((private + 1, seed + Field::from(private)))\n"
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
        "#[test]\nfn generated_fallible_witness_propagates_rejection() {\n"
        "    let contract = Contract::from(FallibleSecret);\n"
        "    let context = initial_state(ConstructorContext::new(7_u64)).unwrap().into_circuit_context(ContractAddress::default());\n"
        "    let call = contract.recording().write_twice(context, Field::from(2_u64)).unwrap();\n"
        "    assert_eq!(call.execution.context.private_state, 9);\n"
        "    let mut context = initial_state(ConstructorContext::new(7_u64)).unwrap().into_circuit_context(ContractAddress::default());\n"
        "    context.gas_limit = Some(RunningCost::ZERO);\n"
        "    assert!(contract.recording().write_twice(context, Field::from(2_u64)).is_err());\n"
        "}\n"
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)


def check_merkle_witness_consumer(compiler: str, base: Path) -> None:
    output = base / "merkle-witness-contract"
    run(compiler, "--target", "rust", "--skip-zk", str(MERKLE_WITNESS_SOURCE), str(output))
    check_manifest(output)
    contract = output / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "merkle-witness-consumer"
    consumer.mkdir()
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-merkle-witness-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))} }}\n'
    )
    (consumer / "tests/merkle.rs").write_text(
        "use compact_contract_merkle_path_witness::ledger_contract::{Contract, LedgerView, TryWitnesses, initial_state};\n"
        "use compact_contract_merkle_path_witness::types::{MerkleTreeDigest, MerkleTreePath};\n"
        "use compact_contract_merkle_path_witness::runtime::{BoundedUint, CompactError};\n"
        "use compact_contract_merkle_path_witness::runtime::context::{ConstructorContext, RunningCost, WitnessContext};\n"
        "use compact_contract_merkle_path_witness::runtime::ledger::ContractAddress;\n"
        "struct MerkleWitness;\n"
        "impl TryWitnesses<()> for MerkleWitness {\n"
        "    fn leaf_path(&self, context: WitnessContext<'_, (), LedgerView<'_>>) -> Result<((), MerkleTreePath), CompactError> {\n"
        "        let path = context.ledger.t()?.path_for_leaf(0, BoundedUint::<255>::new(7)?)?;\n"
        "        Ok(((), MerkleTreePath::from_ledger_path(path)?))\n"
        "    }\n"
        "    fn historic_path(&self, context: WitnessContext<'_, (), LedgerView<'_>>) -> Result<((), MerkleTreePath), CompactError> {\n"
        "        let path = context.ledger.h()?.path_for_leaf(0, BoundedUint::<255>::new(7)?)?;\n"
        "        Ok(((), MerkleTreePath::from_ledger_path(path)?))\n"
        "    }\n"
        "    fn merkle_checks(&self, context: WitnessContext<'_, (), LedgerView<'_>>) -> Result<((), bool), CompactError> {\n"
        "        let tree = context.ledger.t()?;\n"
        "        let root = MerkleTreeDigest { field: tree.root().unwrap().0 };\n"
        "        Ok(((), !tree.is_full()? && tree.check_root(root)?))\n"
        "    }\n"
        "    fn historic_checks(&self, context: WitnessContext<'_, (), LedgerView<'_>>) -> Result<((), bool), CompactError> {\n"
        "        let tree = context.ledger.h()?;\n"
        "        let root = MerkleTreeDigest { field: tree.root().unwrap().0 };\n"
        "        Ok(((), !tree.is_full()? && tree.check_root(root)?))\n"
        "    }\n"
        "}\n"
        "#[test]\nfn merkle_witness_views_work_from_a_one_dependency_crate() {\n"
        "    let contract = Contract::from(MerkleWitness);\n"
        "    let context = initial_state(ConstructorContext::new(())).unwrap().into_circuit_context(ContractAddress::default());\n"
        "    let context = contract.append(context, BoundedUint::<255>::new(7).unwrap()).unwrap().context;\n"
        "    let context = contract.append_h(context, BoundedUint::<255>::new(7).unwrap()).unwrap().context;\n"
        "    let local = contract.get_path(context).unwrap();\n"
        "    assert_eq!(local.gas_cost, RunningCost::ZERO);\n"
        "    let local_h = contract.get_historic_path(local.context).unwrap();\n"
        "    assert_eq!(local_h.gas_cost, RunningCost::ZERO);\n"
        "    let plain = contract.check_witness_merkle(local_h.context).unwrap();\n"
        "    assert!(plain.result && plain.gas_cost.read_time > RunningCost::ZERO.read_time);\n"
        "    let historic = contract.check_witness_history(plain.context).unwrap();\n"
        "    assert!(historic.result && historic.gas_cost.read_time > RunningCost::ZERO.read_time);\n"
        "    let mut rejected = initial_state(ConstructorContext::new(())).unwrap().into_circuit_context(ContractAddress::default());\n"
        "    rejected.gas_limit = Some(RunningCost::ZERO);\n"
        "    assert!(contract.check_witness_merkle(rejected).is_err());\n"
        "}\n"
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)
    (consumer / "examples").mkdir()
    (consumer / "examples/wrong_merkle_root.rs").write_text(
        "use compact_contract_merkle_path_witness::ledger_contract::LedgerView;\n"
        "use compact_contract_merkle_path_witness::runtime::context::WitnessContext;\n"
        "use compact_contract_merkle_path_witness::runtime::CompactError;\n"
        "fn invalid(context: WitnessContext<'_, (), LedgerView<'_>>) -> Result<(), CompactError> {\n"
        "    let _ = context.ledger.t()?.check_root(true)?;\n"
        "    Ok(())\n"
        "}\n"
        "fn main() {}\n"
    )
    rejected = subprocess.run(
        ["cargo", "check", "--quiet", "--example", "wrong_merkle_root"],
        cwd=consumer, env=environment, text=True, capture_output=True,
    )
    assert rejected.returncode != 0 and "expected `MerkleTreeDigest`" in rejected.stderr, rejected.stderr


def check_list_shapes_consumer(compiler: str, base: Path) -> None:
    output = base / "list-shapes-contract"
    run(compiler, "--target", "rust", "--skip-zk", str(LIST_SHAPES_SOURCE), str(output))
    check_manifest(output)
    contract = output / "contract"
    package = tomllib.loads((contract / "Cargo.toml").read_text())
    consumer = base / "list-shapes-consumer"
    consumer.mkdir()
    (consumer / "tests").mkdir()
    (consumer / "Cargo.toml").write_text(
        "[package]\nname = \"compactc-list-shapes-smoke\"\n"
        "version = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
        f'{package["package"]["name"]} = {{ path = {json.dumps(str(contract))} }}\n'
    )
    (consumer / "tests/list_shapes.rs").write_bytes(
        (ROOT / "tools/compact-rust-backend/consumers/list_shapes.rs").read_bytes()
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    subprocess.run(["cargo", "test", "--quiet"], cwd=consumer, env=environment, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--consumer", action="store_true", help="build and run a separate consumer")
    parser.add_argument("--proof", action="store_true", help="generate ZKIR and proving keys")
    args = parser.parse_args()
    # Captured Rust errors are asserted below; runner color settings must not split their text.
    os.environ["CARGO_TERM_COLOR"] = "never"
    # Proof-smoke cargo invocations also need the isolated consumer target when
    # this check runs directly rather than through local_parity_gate.py.
    os.environ.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
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
        capabilities = json.loads((rust / "contract/rust-capabilities.json").read_text())
        assert capabilities["schema_version"] == 3
        assert [(c["name"], c["recorded"], c["observed_call"]) for c in capabilities["circuits"]] == [
            ("increment", True, True), ("read_round", True, True)
        ]
        assert all(c["proof_required"] is True and c["recording_status"] == "available"
                   for c in capabilities["circuits"])
        rust_ir = json.loads((rust / "contract/compact-rust-ir.json").read_text())
        assert rust_ir["schema_version"] == 11
        native_output = base / "native-own-public-key"
        run(compiler, "--target", "rust", "--skip-zk", str(PM_19252_SOURCE), str(native_output))
        native_contract = native_output / "contract"
        native_ir = json.loads((native_contract / "compact-rust-ir.json").read_text())
        native_circuit = next(c for c in native_ir["stateful_circuits"] if c["name"] == "test1")
        assert native_ir["witnesses"] == []
        assert native_circuit["actions"] == [
            {"kind": "native_witness_call", "builtin": "own_public_key"}
        ]
        native_capability = json.loads((native_contract / "rust-capabilities.json").read_text())
        assert len(native_capability["circuits"]) == 1
        native_export = native_capability["circuits"][0]
        assert native_export["name"] == "test1"
        assert native_export["proof_required"] is False
        assert native_export["recorded"] is False
        assert native_export["observed_call"] is False
        assert native_export["recording_status"] == "not_applicable"
        assert native_export["recording_unavailable"]["ir_node"] == "StateAction::NativeWitnessCall"
        native_source = (native_contract / "lib.rs").read_text()
        assert "context.own_coin_public_key()?" in native_source
        assert "pub fn test1<Private>(" in native_source
        assert "pub fn test1<Private, W:" not in native_source
        native_value_output = base / "native-own-public-key-value"
        run(compiler, "--target", "rust", "--skip-zk", str(NATIVE_KEY_VALUE_SOURCE),
            str(native_value_output))
        native_value_contract = native_value_output / "contract"
        native_value_ir = json.loads((native_value_contract / "compact-rust-ir.json").read_text())
        assert native_value_ir["schema_version"] == 11
        assert native_value_ir["witnesses"] == []
        assert [(c["name"], c["return_value"]["value"]["kind"])
                for c in native_value_ir["stateful_circuits"]] == [
            ("key", "native_witness_call"), ("key_bytes", "struct_field")
        ]
        assert native_value_ir["stateful_circuits"][1]["return_value"]["value"]["value"] == {
            "kind": "native_witness_call", "builtin": "own_public_key"
        }
        native_value_capability = json.loads((native_value_contract / "rust-capabilities.json").read_text())
        assert [(c["name"], c["proof_required"], c["recording_status"])
                for c in native_value_capability["circuits"]] == [
            ("key", False, "not_applicable"), ("key_bytes", False, "not_applicable")
        ]
        native_value_source = (native_value_contract / "lib.rs").read_text()
        assert native_value_source.count("context.own_coin_public_key()?") == 2
        assert "pub fn key<Private, W:" not in native_value_source
        assert "pub fn key_bytes<Private, W:" not in native_value_source
        round_field = next(field for field in rust_ir["ledger_fields"] if field["id"] == "round")
        assert round_field["source"]["file"] == SOURCE.name
        assert (round_field["source"]["line"], round_field["source"]["column"]) == (18, 1)
        assert rust_ir["stateful_circuits"]
        assert (rust_ir["stateful_circuits"][0]["source"]["line"], rust_ir["stateful_circuits"][0]["source"]["column"]) == (20, 1)
        for circuit in rust_ir["stateful_circuits"]:
            assert circuit["source"]["file"] == SOURCE.name
            assert circuit["source"]["line"] > 0
        for name, source, key, first_line in (
            ("pure", PURE_SOURCE, "circuits", 18),
            ("witness", WITNESS_CELL_SOURCE, "witnesses", 20),
            ("constructor", CONSTRUCTOR_MAP_SOURCE, "constructor", 21),
            ("alias", ALIAS_SOURCE, "type_aliases", 27),
        ):
            output = base / f"source-{name}"
            run(compiler, "--target", "rust", "--skip-zk", str(source), str(output))
            emitted = json.loads((output / "contract/compact-rust-ir.json").read_text())
            assert emitted["schema_version"] == 11
            owners = emitted[key]
            if isinstance(owners, dict):
                owners = [owners]
            assert owners, f"{source.name}: missing {key}"
            assert (owners[0]["source"]["line"], owners[0]["source"]["column"]) == (first_line, 1)
            for owner in owners:
                assert owner["source"]["file"] == source.name
                assert owner["source"]["line"] > 0
                assert owner["source"]["column"] > 0
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
        legacy_rust = base / "legacy-rust"
        run(compiler, "--rust", "--skip-ts", "--skip-zk", str(SOURCE), str(legacy_rust))
        assert (legacy_rust / "contract/lib.rs").read_bytes() == (rust / "contract/lib.rs").read_bytes()
        assert (legacy_rust / "contract/Cargo.toml").read_bytes() == (rust / "contract/Cargo.toml").read_bytes()
        assert (legacy_rust / "contract/rust-capabilities.json").read_bytes() == (
            rust / "contract/rust-capabilities.json"
        ).read_bytes()
        assert not (legacy_rust / "contract/index.js").exists()
        check_manifest(legacy_rust)
        legacy_both = base / "legacy-both"
        run(compiler, "--rust", "--skip-zk", str(SOURCE), str(legacy_both))
        assert (legacy_both / "contract/index.js").read_bytes() == (both / "contract/index.js").read_bytes()
        assert (legacy_both / "contract/lib.rs").read_bytes() == (both / "contract/lib.rs").read_bytes()
        assert (legacy_both / "contract/Cargo.toml").read_bytes() == (both / "contract/Cargo.toml").read_bytes()
        check_manifest(legacy_both)
        for label, flags, message in (
            ("mixed-rust", ("--target", "rust", "--rust"), "cannot be combined"),
            ("mixed-skip-ts", ("--skip-ts", "--target=ts"), "cannot be combined"),
            ("skip-ts-alone", ("--skip-ts",), "requires --rust"),
        ):
            rejected_output = base / label
            rejected = subprocess.run(
                [compiler, *flags, "--skip-zk", str(SOURCE), str(rejected_output)],
                cwd=ROOT, capture_output=True, text=True,
            )
            assert rejected.returncode != 0, label
            assert message in rejected.stderr, (label, rejected.stderr)
            assert not rejected_output.exists(), f"{label}: invalid target selection created output"
        if args.consumer:
            run(compiler, "--target", "rust", "--skip-zk", str(PURE_SOURCE), str(pure))
            check_consumer(legacy_rust / "contract", pure / "contract", base / "consumer")
            check_shared_runtime_consumer(compiler, base)
            check_witness_consumer(compiler, base)
            check_merkle_witness_consumer(compiler, base)
            check_list_shapes_consumer(compiler, base)
        if args.proof:
            proof = base / "proof"
            run(compiler, "--target", "rust", "--rust-require-recording", str(SOURCE), str(proof))
            check_manifest(proof)
            for circuit in ("increment", "read_round"):
                for extension in ("prover", "verifier"):
                    assert (proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (proof / "zkir" / f"{circuit}.{extension}").is_file()
            check_observed_call_consumer(proof, base)
            counter_parameter_proof = base / "counter-parameter-proof"
            run(compiler, "--target", "rust", "--rust-require-recording", str(COUNTER_PARAMETER_SOURCE), str(counter_parameter_proof))
            check_manifest(counter_parameter_proof)
            for circuit in ("increment_by", "reset_round"):
                for extension in ("prover", "verifier"):
                    assert (counter_parameter_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (counter_parameter_proof / "zkir" / f"{circuit}.{extension}").is_file()
            unsigned_proofs = {}
            for label, source, circuits in (
                ("bounded-uint", BOUNDED_UINT_SOURCE, ("set_small",)),
                ("uints", UINTS_SOURCE, ("set_byte",)),
                ("cross-circuit", CROSS_CIRCUIT_SOURCE, ("reset", "reset_and_set")),
                ("wide-uint", WIDE_UINT_SOURCE, ("writeWide", "readWide")),
                ("bug11", BUG11_SOURCE, ("set_tiny", "set_medium", "set_wide")),
                ("multi-pl-call", MULTI_PL_CALL_SOURCE, ("record_update",)),
            ):
                unsigned_proof = base / f"{label}-proof"
                unsigned_proofs[label] = unsigned_proof
                run(compiler, "--target", "rust", "--rust-require-recording", str(source), str(unsigned_proof))
                check_manifest(unsigned_proof)
                capabilities = json.loads((unsigned_proof / "contract/rust-capabilities.json").read_text())
                assert {row["name"] for row in capabilities["circuits"]} == set(circuits)
                assert all(row["recorded"] and row["observed_call"] for row in capabilities["circuits"])
                for circuit in circuits:
                    for extension in ("prover", "verifier"):
                        assert (unsigned_proof / "keys" / f"{circuit}.{extension}").is_file()
                    for extension in ("zkir", "bzkir"):
                        assert (unsigned_proof / "zkir" / f"{circuit}.{extension}").is_file()
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
            composite_cell_proof = base / "composite-cell-proof"
            run(compiler, "--target", "rust", "--rust-require-recording", str(CELL_STRUCT_SOURCE), str(composite_cell_proof))
            check_manifest(composite_cell_proof)
            for circuit in ("set_record", "read_record"):
                for extension in ("prover", "verifier"):
                    assert (composite_cell_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (composite_cell_proof / "zkir" / f"{circuit}.{extension}").is_file()
            witness_proof = base / "witness-proof"
            run(compiler, "--target", "rust", str(WITNESS_CELL_SOURCE), str(witness_proof))
            check_manifest(witness_proof)
            check_observed_witness_call_consumer(witness_proof, base)
            for circuit in ("write_twice", "write_offset", "write_nested_twice"):
                for extension in ("prover", "verifier"):
                    assert (witness_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (witness_proof / "zkir" / f"{circuit}.{extension}").is_file()
            assert_witness_proof = base / "assert-witness-proof"
            run(compiler, "--target", "rust", str(ASSERT_WITNESS_SOURCE),
                str(assert_witness_proof))
            check_manifest(assert_witness_proof)
            capabilities = json.loads(
                (assert_witness_proof / "contract/rust-capabilities.json").read_text()
            )
            checked_write = next(circuit for circuit in capabilities["circuits"]
                                 if circuit["name"] == "checked_write")
            assert checked_write["proof_required"] and checked_write["recorded"] \
                and checked_write["observed_call"]
            for extension in ("prover", "verifier"):
                assert (assert_witness_proof / "keys" / f"checked_write.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (assert_witness_proof / "zkir" / f"checked_write.{extension}").is_file()
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
            for circuit in ("outer", "outerValue", "outerValue2", "outerValueExpr"):
                for extension in ("prover", "verifier"):
                    assert (nested_witness_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (nested_witness_proof / "zkir" / f"{circuit}.{extension}").is_file()
            set_proof = base / "set-proof"
            run(compiler, "--target", "rust", str(SET_BOOLEAN_SOURCE), str(set_proof))
            check_manifest(set_proof)
            for circuit in (
                "add", "contains", "remove", "seen_size", "seen_is_empty",
                "add_field", "contains_field", "reset_fields", "choose",
            ):
                for extension in ("prover", "verifier"):
                    assert (set_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (set_proof / "zkir" / f"{circuit}.{extension}").is_file()
            set_oracle_proof = base / "set-oracle-proof"
            run(compiler, "--target", "rust", str(SET_SOURCE), str(set_oracle_proof))
            check_manifest(set_oracle_proof)
            for extension in ("prover", "verifier"):
                assert (set_oracle_proof / "keys" / f"check.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (set_oracle_proof / "zkir" / f"check.{extension}").is_file()
            map_proof = base / "map-proof"
            run(compiler, "--target", "rust", str(MAP_BOOLEAN_SOURCE), str(map_proof))
            check_manifest(map_proof)
            check_observed_map_call_consumer(map_proof, base)
            for circuit in (
                "put", "put_pair", "put_default", "has", "get", "remove_key",
                "table_size", "table_is_empty", "reset_table",
            ):
                for extension in ("prover", "verifier"):
                    assert (map_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (map_proof / "zkir" / f"{circuit}.{extension}").is_file()
            constructor_map_proof = base / "constructor-map-proof"
            run(compiler, "--target", "rust", str(CONSTRUCTOR_MAP_SOURCE), str(constructor_map_proof))
            check_manifest(constructor_map_proof)
            for circuit in ("table_size", "history_size", "get_true", "get_false_history"):
                for extension in ("prover", "verifier"):
                    assert (constructor_map_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (constructor_map_proof / "zkir" / f"{circuit}.{extension}").is_file()
            list_proof = base / "list-proof"
            run(compiler, "--target", "rust", str(LIST_SOURCE), str(list_proof))
            check_manifest(list_proof)
            for circuit in ("item_count", "items_empty", "first_item", "prepend", "drop_first", "clear_items"):
                for extension in ("prover", "verifier"):
                    assert (list_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (list_proof / "zkir" / f"{circuit}.{extension}").is_file()
            list_shapes_proof = base / "list-shapes-proof"
            run(compiler, "--target", "rust", str(LIST_SHAPES_SOURCE), str(list_shapes_proof))
            check_manifest(list_shapes_proof)
            for circuit in ("push_flag", "push_count", "push_tag", "push_choice", "push_packet", "first_packet", "first_choice"):
                for extension in ("prover", "verifier"):
                    assert (list_shapes_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (list_shapes_proof / "zkir" / f"{circuit}.{extension}").is_file()
            merkle_proof = base / "merkle-proof"
            run(compiler, "--target", "rust", str(MERKLE_SOURCE), str(merkle_proof))
            check_manifest(merkle_proof)
            for circuit in ("append", "full"):
                for extension in ("prover", "verifier"):
                    assert (merkle_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (merkle_proof / "zkir" / f"{circuit}.{extension}").is_file()
            merkle_verify_proof = base / "merkle-verify-proof"
            run(compiler, "--target", "rust", str(MERKLE_VERIFY_SOURCE), str(merkle_verify_proof))
            check_manifest(merkle_verify_proof)
            capabilities = json.loads((merkle_verify_proof / "contract/rust-capabilities.json").read_text())
            verify_capability = next(circuit for circuit in capabilities["circuits"] if circuit["name"] == "verify")
            assert verify_capability["recorded"] and verify_capability["observed_call"]
            for extension in ("prover", "verifier"):
                assert (merkle_verify_proof / "keys" / f"verify.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (merkle_verify_proof / "zkir" / f"verify.{extension}").is_file()
            persistent_commit_proof = base / "persistent-commit-proof"
            run(compiler, "--target", "rust", str(PERSISTENT_COMMIT_SOURCE),
                str(persistent_commit_proof))
            check_manifest(persistent_commit_proof)
            capabilities = json.loads(
                (persistent_commit_proof / "contract/rust-capabilities.json").read_text()
            )
            for name in ("commitSmall", "commitU128", "commitFieldOnly",
                         "pureBodyFieldOnly"):
                capability = next(circuit for circuit in capabilities["circuits"]
                                  if circuit["name"] == name)
                assert capability["proof_required"] and capability["recorded"] \
                    and capability["observed_call"]
            for extension in ("prover", "verifier"):
                assert (persistent_commit_proof / "keys" / f"commitSmall.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (persistent_commit_proof / "zkir" / f"commitSmall.{extension}").is_file()
            internal_pure_call_proof = base / "internal-pure-call-proof"
            run(compiler, "--target", "rust", str(INTERNAL_PURE_CALL_SOURCE),
                str(internal_pure_call_proof))
            check_manifest(internal_pure_call_proof)
            capabilities = json.loads(
                (internal_pure_call_proof / "contract/rust-capabilities.json").read_text()
            )
            save = next(circuit for circuit in capabilities["circuits"]
                        if circuit["name"] == "save")
            assert save["proof_required"] and save["recorded"] and save["observed_call"]
            for extension in ("prover", "verifier"):
                assert (internal_pure_call_proof / "keys" / f"save.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (internal_pure_call_proof / "zkir" / f"save.{extension}").is_file()
            stateful_pure_call_proof = base / "stateful-pure-call-proof"
            run(compiler, "--target", "rust", str(STATEFUL_PURE_CALL_SOURCE),
                str(stateful_pure_call_proof))
            check_manifest(stateful_pure_call_proof)
            capabilities = json.loads(
                (stateful_pure_call_proof / "contract/rust-capabilities.json").read_text()
            )
            save = next(circuit for circuit in capabilities["circuits"]
                        if circuit["name"] == "save")
            assert save["proof_required"] and save["recorded"] and save["observed_call"]
            for extension in ("prover", "verifier"):
                assert (stateful_pure_call_proof / "keys" / f"save.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (stateful_pure_call_proof / "zkir" / f"save.{extension}").is_file()
            ternary_cond_proof = base / "ternary-cond-proof"
            run(compiler, "--target", "rust", str(TERNARY_COND_SOURCE), str(ternary_cond_proof))
            check_manifest(ternary_cond_proof)
            capabilities = json.loads(
                (ternary_cond_proof / "contract/rust-capabilities.json").read_text()
            )
            for name in ("walkerWrite", "walkerCallPure", "streamCallPure",
                         "witnessArg", "streamCallWitness", "streamAssertEq"):
                circuit = next(circuit for circuit in capabilities["circuits"]
                               if circuit["name"] == name)
                assert circuit["proof_required"] and circuit["recorded"] \
                    and circuit["observed_call"]
            for name in ("walkerWrite", "streamCallPure", "streamCallWitness", "streamAssertEq"):
                for extension in ("prover", "verifier"):
                    assert (ternary_cond_proof / "keys" / f"{name}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (ternary_cond_proof / "zkir" / f"{name}.{extension}").is_file()
            asset_writable_proof = base / "asset-writable-proof"
            run(compiler, "--target", "rust", str(ASSET_REGISTRY_SOURCE),
                str(asset_writable_proof))
            check_manifest(asset_writable_proof)
            capabilities = json.loads(
                (asset_writable_proof / "contract/rust-capabilities.json").read_text()
            )
            for circuit in ("setCustodian", "tag"):
                capability = next(item for item in capabilities["circuits"]
                                  if item["name"] == circuit)
                assert capability["proof_required"] and capability["recorded"] \
                    and capability["observed_call"]
                for extension in ("prover", "verifier"):
                    assert (asset_writable_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (asset_writable_proof / "zkir" / f"{circuit}.{extension}").is_file()
            historic_merkle_proof = base / "historic-merkle-proof"
            run(compiler, "--target", "rust", str(HISTORIC_MERKLE_SOURCE), str(historic_merkle_proof))
            check_manifest(historic_merkle_proof)
            for circuit in ("append", "full"):
                for extension in ("prover", "verifier"):
                    assert (historic_merkle_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (historic_merkle_proof / "zkir" / f"{circuit}.{extension}").is_file()
            vector_key_proof = base / "vector-key-proof"
            run(compiler, "--target", "rust", str(VECTOR_KEY_SOURCE), str(vector_key_proof))
            check_manifest(vector_key_proof)
            for extension in ("prover", "verifier"):
                assert (vector_key_proof / "keys" / f"setInsert.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (vector_key_proof / "zkir" / f"setInsert.{extension}").is_file()
            composite_key_proof = base / "composite-key-proof"
            run(compiler, "--target", "rust", str(COMPOSITE_KEY_SOURCE), str(composite_key_proof))
            check_manifest(composite_key_proof)
            for circuit in (
                "insert_vector", "insert_tuple", "insert_struct",
                "roundtrip_tuple", "roundtrip_struct", "record_twelve",
            ):
                for extension in ("prover", "verifier"):
                    assert (composite_key_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (composite_key_proof / "zkir" / f"{circuit}.{extension}").is_file()
            check_observed_composite_key_consumer(composite_key_proof, base)
            chunked_set_proof = base / "chunked-set-proof"
            run(compiler, "--target", "rust", str(CHUNKED_SET_SOURCE), str(chunked_set_proof))
            check_manifest(chunked_set_proof)
            for circuit in ("insert_key", "roundtrip_key", "key_count", "empty"):
                for extension in ("prover", "verifier"):
                    assert (chunked_set_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (chunked_set_proof / "zkir" / f"{circuit}.{extension}").is_file()
            check_chunked_set_consumer(chunked_set_proof, base)
            chunked_list_proof = base / "chunked-list-proof"
            run(compiler, "--target", "rust", str(CHUNKED_LIST_SOURCE), str(chunked_list_proof))
            check_manifest(chunked_list_proof)
            for circuit in (
                "item_count", "items_empty", "first_item", "prepend", "drop_first", "clear_items",
            ):
                for extension in ("prover", "verifier"):
                    assert (chunked_list_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (chunked_list_proof / "zkir" / f"{circuit}.{extension}").is_file()
            check_chunked_list_consumer(chunked_list_proof, base)
            chunked_map_proof = base / "chunked-map-proof"
            run(compiler, "--target", "rust", str(CHUNKED_MAP_SOURCE), str(chunked_map_proof))
            check_manifest(chunked_map_proof)
            for circuit in (
                "put", "put_pair", "put_default", "has", "get", "remove_key",
                "table_size", "table_is_empty", "reset_table",
            ):
                for extension in ("prover", "verifier"):
                    assert (chunked_map_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (chunked_map_proof / "zkir" / f"{circuit}.{extension}").is_file()
            check_chunked_map_consumer(chunked_map_proof, base)
            chunked_cell_proof = base / "chunked-cell-proof"
            run(compiler, "--target", "rust", "--rust-require-recording", str(CHUNKED_CELL_SOURCE), str(chunked_cell_proof))
            check_manifest(chunked_cell_proof)
            capabilities = json.loads((chunked_cell_proof / "contract/rust-capabilities.json").read_text())
            assert all(c["recorded"] and c["observed_call"] for c in capabilities["circuits"])
            for circuit in (
                "set_active", "get_active", "assert_active",
                "set_amount", "get_amount", "add_amount", "active_equals", "plus_amount",
                "subtract_amount", "multiply_amount",
            ):
                for extension in ("prover", "verifier"):
                    assert (chunked_cell_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (chunked_cell_proof / "zkir" / f"{circuit}.{extension}").is_file()
            check_chunked_cell_consumer(chunked_cell_proof, base)
            constructor_list_proof = base / "constructor-list-proof"
            run(compiler, "--target", "rust", str(CONSTRUCTOR_LIST_SOURCE), str(constructor_list_proof))
            check_manifest(constructor_list_proof)
            for circuit in ("item_count", "history_count", "first_item", "drop_first", "clear_items"):
                for extension in ("prover", "verifier"):
                    assert (constructor_list_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (constructor_list_proof / "zkir" / f"{circuit}.{extension}").is_file()
            enum_cell_proof = base / "enum-cell-proof"
            run(compiler, "--target", "rust", str(RECORDED_ENUM_SOURCE), str(enum_cell_proof))
            check_manifest(enum_cell_proof)
            for extension in ("prover", "verifier"):
                assert (enum_cell_proof / "keys" / f"choose.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (enum_cell_proof / "zkir" / f"choose.{extension}").is_file()
            tiny_proof = base / "tiny-proof"
            run(compiler, "--target", "rust", str(TINY_SOURCE), str(tiny_proof))
            check_manifest(tiny_proof)
            for circuit in ("clear", "set", "get"):
                for extension in ("prover", "verifier"):
                    assert (tiny_proof / "keys" / f"{circuit}.{extension}").is_file()
                for extension in ("zkir", "bzkir"):
                    assert (tiny_proof / "zkir" / f"{circuit}.{extension}").is_file()
            nested_map_shape_proof = base / "nested-map-shape-proof"
            run(compiler, "--target", "rust", str(NESTED_MAP_SHAPE_SOURCE), str(nested_map_shape_proof))
            check_manifest(nested_map_shape_proof)
            for extension in ("prover", "verifier"):
                assert (nested_map_shape_proof / "keys" / f"check_nested_empty.{extension}").is_file()
            for extension in ("zkir", "bzkir"):
                assert (nested_map_shape_proof / "zkir" / f"check_nested_empty.{extension}").is_file()
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                str(proof), str(cell_proof), str(cell_read_proof), str(witness_proof), str(nested_proof),
                str(nested_witness_proof), str(set_proof), str(set_oracle_proof),
                str(map_proof), str(constructor_map_proof), str(list_proof), str(constructor_list_proof),
                str(enum_cell_proof),
                str(tiny_proof),
                str(nested_map_shape_proof),
                str(list_shapes_proof),
                str(merkle_proof),
                str(historic_merkle_proof),
                str(vector_key_proof),
                str(counter_parameter_proof),
                str(composite_key_proof),
                str(chunked_set_proof),
                str(chunked_list_proof),
                str(chunked_map_proof),
                str(chunked_cell_proof),
                str(unsigned_proofs["uints"]),
                str(unsigned_proofs["wide-uint"]),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--composite-cell", str(composite_cell_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--merkle-verify", str(merkle_verify_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--persistent-commit", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--closed-pure-field", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--conditional-field", str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--pure-field-arguments", str(internal_pure_call_proof), str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--conditional-assert-eq", str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--conditional-set", str(set_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--asset-writable", str(asset_writable_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--stateful-pure-return", str(stateful_pure_call_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--assert-witness", str(assert_witness_proof),
            )
    print("compactc target boundary and manifest: passed")


if __name__ == "__main__":
    main()
