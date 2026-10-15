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
import sys
import tempfile
import tomllib

import forge_circuits

from target_checks import (
    application_checks, collection_proofs, intent_checks, native_targets, proof_artifacts,
    recording_checks, scalar_checks, shared_runtime, shielded,
)


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "examples/rust_backend/counter.compact"
TEST_CENTER_COUNTER_SOURCE = ROOT / "test-center/test-contracts/counter.compact"
TEST_CENTER_BBOARD_SOURCE = ROOT / "test-center/test-contracts/bboard.compact"
ADT_SET_QUALIFIED_SOURCE = ROOT / "examples/adt/tests/set_qualified_coin_info.compact"
QUALIFIED_COIN_ORACLE_SOURCE = ROOT / "examples/rust_backend/qualified_coin_set_oracle.compact"
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
WITNESS_VECTOR_ACTION_SOURCE = ROOT / "examples/rust_backend/witness_vector_action.compact"
INTERNAL_PURE_CALL_SOURCE = ROOT / "examples/rust_backend/internal_pure_call.compact"
STATEFUL_PURE_CALL_SOURCE = ROOT / "examples/rust_backend/stateful_pure_call.compact"
FIELD_CAST_UINT128_SOURCE = ROOT / "examples/rust_backend/field_cast_uint128.compact"
WIDENING_ARITH_SOURCE = ROOT / "examples/rust_backend/widening_arith_oracle.compact"
MIXED_WIDTH_OPERAND_SOURCE = ROOT / "examples/rust_backend/mixed_width_operand_oracle.compact"
TERNARY_COND_SOURCE = ROOT / "examples/rust_backend/ternary_cond_oracle.compact"
NESTED_STATEFUL_TERNARY_SOURCE = ROOT / "examples/rust_backend/nested_stateful_ternary.compact"
ASSET_REGISTRY_SOURCE = ROOT / "examples/rust_backend/asset_registry_oracle.compact"
SCHNORR_ATTEST_SOURCE = ROOT / "examples/rust_backend/schnorr_attest_oracle.compact"
HISTORIC_MERKLE_SOURCE = ROOT / "examples/rust_backend/hmt_insert_oracle.compact"
HISTORIC_MERKLE_DEFAULT_SOURCE = ROOT / "examples/rust_backend/hmt_default_oracle.compact"
ELECTION_SOURCE = ROOT / "examples/rust_backend/election_oracle.compact"
ZEROCASH_SOURCE = ROOT / "examples/rust_backend/zerocash_oracle.compact"
INLINE_TYPE_SCOPE_SOURCE = ROOT / "examples/rust_backend/inline_type_scope_oracle.compact"
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
LET_RETURN_SOURCE = ROOT / "examples/rust_backend/let_return_oracle.compact"
ROOT_LET_ACTION_RETURN_SOURCE = ROOT / "examples/rust_backend/root_let_action_return_oracle.compact"
EFFECTFUL_RETURN_SOURCE = ROOT / "examples/rust_backend/effectful_return_oracle.compact"
CORACLE_SOURCE = ROOT / "test-center/test-contracts/coracle.compact"
SHIELDED_RECEIVE_SOURCE = ROOT / "examples/rust_backend/shielded_receive_oracle.compact"
SHIELDED_SEND_SOURCE = ROOT / "examples/rust_backend/shielded_send_oracle.compact"
TEST_CENTER_WELCOME_SOURCE = ROOT / "test-center/test-contracts/welcome.compact"
WITNESS_CELL_SOURCE = ROOT / "examples/rust_backend/witness_cell_write.compact"
ASSERT_WITNESS_SOURCE = ROOT / "examples/rust_backend/assert_witness.compact"
MERKLE_WITNESS_SOURCE = ROOT / "examples/rust_backend/merkle_path_witness.compact"
LIST_SHAPES_SOURCE = ROOT / "examples/rust_backend/witness_list_shapes.compact"
NESTED_COUNTER_SOURCE = ROOT / "examples/rust_backend/stateful_circuit_call.compact"
NESTED_WITNESS_SOURCE = ROOT / "examples/rust_backend/nested_witness_call_oracle.compact"
ALIAS_SOURCE = ROOT / "examples/rust_backend/aliases_oracle.compact"
FIELD_TO_BYTES32_SOURCE = ROOT / "examples/rust_backend/field_to_bytes32_oracle.compact"


def run(*arguments: str, cwd: Path = ROOT, env: dict[str, str] | None = None) -> None:
    subprocess.run(forge_circuits.command(arguments, os.environ if env is None else env),
                   cwd=cwd, env=env, check=True)


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
    shared_runtime.check(
        compiler, base, ROOT,
        (
            ("counter", SOURCE),
            ("field_add", PURE_SOURCE),
            ("cell_boolean", CELL_SOURCE),
            ("set_oracle", SET_SOURCE),
            ("map_boolean_field", MAP_BOOLEAN_SOURCE),
            ("nested_map_oracle", NESTED_MAP_SOURCE),
            ("list_field", LIST_SOURCE),
            ("merkle_tree_oracle", MERKLE_SOURCE),
            ("hmt_insert_oracle", HISTORIC_MERKLE_SOURCE),
            ("vector_key_adt", VECTOR_KEY_SOURCE),
        ),
        assets=ROOT / "tools/compact-rust-backend/consumers/shared_runtime",
        run=run,
        execute=subprocess.run,
        check_manifest=check_manifest,
        environ=os.environ,
    )


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


def check_coracle_source(compiler: str, base: Path) -> None:
    manifest = json.loads((ROOT / "tools/compact-rust-backend/parity_positive_test_center_coracle_sources.json").read_text())
    expected, = manifest["positive_sources"]
    output = base / "coracle"
    run(compiler, "--target", "rust", "--skip-zk", str(CORACLE_SOURCE), str(output))
    assert (output / "contract/lib.rs").is_file()
    info = json.loads((output / "compiler/contract-info.json").read_text())
    assert [{key: row[key] for key in ("name", "pure", "proof")}
            for row in info["circuits"]] == expected["proof_circuits"]
    report = json.loads((output / "contract/rust-capabilities.json").read_text())
    assert report["schema_version"] == 3
    assert [row["name"] for row in report["circuits"]] == expected["expected_capability_circuits"]
    assert {row["name"] for row in report["circuits"] if row["recorded"]} == set(expected["expected_recorded_circuits"])
    for row in report["circuits"]:
        if row["name"] in expected["expected_recorded_circuits"]:
            assert row["proof_required"] and row["recorded"] and row["observed_call"]
            assert row["recording_status"] == "available"
            continue
        gap = expected["expected_recording_gaps"][row["name"]]
        assert row["proof_required"] and not row["recorded"] and not row["observed_call"]
        assert row["recording_status"] == "unavailable"
        assert {key: row["recording_unavailable"][key] for key in gap} == gap
        assert row["observed_call_unavailable"]["code"] == "recording_unavailable"
    strict = subprocess.run(
        [compiler, "--target", "rust", "--rust-require-recording", "--skip-zk",
         str(CORACLE_SOURCE), str(base / "coracle-strict")],
        cwd=ROOT, capture_output=True, text=True)
    assert strict.returncode == 0, strict.stderr
    strict_report = json.loads((base / "coracle-strict/contract/rust-capabilities.json").read_text())
    assert {row["name"] for row in strict_report["circuits"] if row["recorded"] and row["observed_call"]} == set(expected["expected_recorded_circuits"])


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


def check_witness_composition(compiler: str, base: Path) -> None:
    """An unrelated retained witness must not change a supported circuit's APIs."""
    for label, source, fixture, calls in [
        ("field", ROOT_LET_ACTION_RETURN_SOURCE, "root-let-action-return-oracle", [
            "compare!(step, r::Field::from(9_u64));",
            "compare!(step, r::Field::from(13_u64));",
        ]),
        ("counter", ROOT / "examples/rust_backend/counter_less_than_oracle.compact", "counter-less-than-oracle", [
            "compare!(nested);", "compare!(short_circuit, true);", "compare!(short_circuit, false);",
            "compare!(compare, r::BoundedUint::new(7).unwrap());",
            "compare!(compare, r::BoundedUint::new(3).unwrap());",
            "compare!(checked, r::BoundedUint::new(7).unwrap());",
            "assert!(c::recorded::checked(composed_context(), r::BoundedUint::new(3).unwrap()).is_err());",
            "assert!(b::recorded::checked(original_context(), br::BoundedUint::new(3).unwrap()).is_err());",
        ]),
    ]:
        composed_source = base / f"{label}-composed.compact"
        composed_source.write_text(source.read_text() + "\nwitness unrelated_value(): Field;\nexport circuit unrelated(): Field { return disclose(unrelated_value()); }\n")
        output = base / f"{label}-generated"
        run(compiler, "--target", "rust", "--rust-runtime-root", str(ROOT), "--rust-require-recording", "--skip-zk", str(composed_source), str(output))
        info = json.loads((output / "compiler/contract-info.json").read_text())
        report = json.loads((output / "contract/rust-capabilities.json").read_text())
        ir = json.loads((output / "contract/compact-rust-ir.json").read_text())
        assert [w["name"] for w in ir["witnesses"]] == ["unrelated_value"]
        assert {c["name"] for c in info["circuits"]} == {c["name"] for c in report["circuits"]}
        for row in report["circuits"]:
            metadata = next(c for c in info["circuits"] if c["name"] == row["name"])
            assert row["proof_required"] == metadata["proof"]
            if row["name"] == "unrelated":
                assert not row["proof_required"] and row["recording_status"] == "not_applicable"
            else:
                assert row["proof_required"] and row["recorded"] and row["observed_call"]
        consumer = base / f"{label}-consumer"
        (consumer / "src").mkdir(parents=True)
        package = tomllib.loads((output / "contract/Cargo.toml").read_text())["package"]["name"]
        original = ROOT / "tests-rust-backend" / fixture
        original_package = tomllib.loads((original / "Cargo.toml").read_text())["package"]["name"]
        (consumer / "Cargo.toml").write_text(f'''[package]
name = "witness-composition-{label}"
version = "0.1.0"
edition = "2024"
[workspace]
[dependencies]
composed = {{ package = "{package}", path = "{output / 'contract'}" }}
original = {{ package = "{original_package}", path = "{original}" }}
serde_json = "1"
''')
        rust = r'''#![cfg(test)]
use composed::{ledger_contract as c, runtime as r};
use original::{ledger_contract as b, runtime as br};
fn composed_context() -> r::context::CircuitContext<()> {
 c::initial_state(r::context::ConstructorContext::new(())).unwrap().into_circuit_context(r::ledger::ContractAddress::default())
}
fn original_context() -> br::context::CircuitContext<()> {
 b::initial_state(br::context::ConstructorContext::new(())).unwrap().into_circuit_context(br::ledger::ContractAddress::default())
}
macro_rules! compare { ($name:ident $(,$arg:expr)*) => {{
 let native = c::$name(composed_context() $(,$arg)*).unwrap();
 let recorded = c::recorded::$name(composed_context() $(,$arg)*).unwrap();
 let baseline = b::recorded::$name(original_context() $(,$arg)*).unwrap();
 assert_eq!(native.result, recorded.execution.result);
 assert_eq!(native.result, baseline.execution.result);
 assert_eq!(native.gas_cost, recorded.execution.gas_cost);
 assert_eq!(native.gas_cost, baseline.execution.gas_cost);
 assert_eq!(native.context.query.state, recorded.execution.context.query.state);
 assert_eq!(native.context.query.state, baseline.execution.context.query.state);
 assert_eq!(native.context.query.effects, baseline.execution.context.query.effects);
 assert!(recorded.execution.private_transcript_outputs.is_empty());
 assert_eq!(serde_json::to_value(recorded.public.verify_ops()).unwrap(), serde_json::to_value(baseline.public.verify_ops()).unwrap());
 let replay = recorded.public.initial().query(recorded.public.verify_ops(),None,&recorded.execution.context.cost_model).unwrap();
 assert_eq!(replay.context.state, native.context.query.state);
}}; }
#[test] fn unrelated_export_does_not_change_recorded_semantics() {
'''+"\n".join(calls)+"\n}\n"
        (consumer / "src/lib.rs").write_text(rust)
        run("cargo", "+1.99.0", "test", "--offline", "--manifest-path", str(consumer / "Cargo.toml"))
        print(json.dumps({"composition": label, "source_sha256": hashlib.sha256(composed_source.read_bytes()).hexdigest(), "proof_apis": sum(c["proof_required"] for c in report["circuits"]), "retained_unrelated_witness": True}))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--consumer", action="store_true", help="build and run a separate consumer")
    parser.add_argument("--proof", action="store_true", help="generate ZKIR and proving keys")
    parser.add_argument("--witness-composition", action="store_true", help="check unrelated witness composition preserves recording")
    parser.add_argument("--stateful-assert", action="store_true", help="check native stateful assertions and complete original micro-dao Cargo admission")
    parser.add_argument("--wide-add", action="store_true", help="check bounded native wide addition and original-source progress")
    parser.add_argument("--transient-receive-send", action="store_true", help="check full immediate-send recording; optional strict transient proof")
    parser.add_argument("--canonical-output-order", action="store_true", help="check two-output recording; optional strict canonical-allocation proofs")
    parser.add_argument("--terminal-lexical-return", action="store_true", help="check terminal lexical recorded returns and optional four strict Dust proofs")
    parser.add_argument("--micro-dao-vote-commit", action="store_true", help="check original vote_commit recording and optional funded strict yes/no proofs")
    parser.add_argument("--micro-dao-cash-out", action="store_true", help="check original cash_out recording; optional strict historical payout proof")
    parser.add_argument("--micro-dao-buy-in", action="store_true", help="check original buy_in recording; optional strict empty/occupied mint proofs")
    parser.add_argument("--micro-dao-set-topic", action="store_true", help="check original set_topic recording; optional funded strict empty/occupied proofs")
    parser.add_argument("--micro-dao-advance", action="store_true", help="check original microDAO advance recording; optional strict Dust proof")
    parser.add_argument("--micro-dao-reveal", action="store_true", help="check original microDAO reveal recording; optional selective proof")
    parser.add_argument("--micro-dao-token", action="store_true", help="check original microDAO token recording; optional selective proof")
    parser.add_argument("--composite-zswap", action="store_true", help="check composite intent recording and optional original-zero/funded-transfer proofs")
    parser.add_argument("--field-observation", action="store_true", help="check read-only Field composite recording and empty-call boundary")
    parser.add_argument("--stateful-struct", action="store_true", help="check ordered composite recording and original micro-dao native admission")
    parser.add_argument("--kernel-shielded-effects", action="store_true", help="check typed Kernel recording and optional call/funded-mint proofs")
    parser.add_argument("--shielded-receive", action="store_true", help="check standard-library receiveShielded recording and optional exact-offer proof")
    parser.add_argument("--shielded-merge", action="store_true", help="check qualified and immediate merge recording and optional strict funded proofs")
    parser.add_argument("--shielded-send", action="store_true", help="check standard-library qualified sendShielded recording and optional strict proof")
    parser.add_argument("--native-zswap-intents", action="store_true", help="check recorded Zswap intent admission and optional strict funded transfer proof")
    parser.add_argument("--qualified-coin-cell", action="store_true",
                        help="check recorded qualified-coin Cell write and offer-backed proof")
    parser.add_argument("--counter-less-than", action="store_true",
                        help="check typed Counter comparison and nested-read recording")
    parser.add_argument("--literal-bytes-field", action="store_true",
                        help="check canonical literal Bytes-to-Field normalization and refusals")
    parser.add_argument("--field-to-bytes32", action="store_true",
                        help="run the focused explicit Field-to-Bytes<32> source gate")
    parser.add_argument("--test-center-bboard", action="store_true",
                        help="compile the complete bboard source and check recorded/observed parity")
    parser.add_argument("--adt-set-qualified", action="store_true",
                        help="compile the original qualified Set source and check typed coin insertion capability")
    parser.add_argument("--coracle-withdraw", action="store_true", help="check original Coracle withdraw recording; optional strict funded proof")
    parser.add_argument("--coracle-concede", action="store_true", help="check original Coracle concede recording; optional strict funded proof")
    parser.add_argument("--coracle-start", action="store_true", help="check original two-wallet Coracle start recording; optional strict red/blue proof")
    parser.add_argument("--coracle-guess", action="store_true", help="check original Coracle guess recording; optional selective proof")
    parser.add_argument("--coracle-root-let", action="store_true",
                        help="check root Let action extraction and complete Coracle native acceptance")
    parser.add_argument("--effectful-return", action="store_true",
                        help="check ordered terminal return plans and complete Coracle native acceptance")
    args = parser.parse_args()
    # Captured Rust errors are asserted below; runner color settings must not split their text.
    os.environ["CARGO_TERM_COLOR"] = "never"
    # Proof-smoke cargo invocations also need the isolated consumer target when
    # this check runs directly rather than through local_parity_gate.py.
    os.environ.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/compactc-consumer"))
    compiler = os.environ.get("COMPACTC", "compactc")
    with tempfile.TemporaryDirectory(prefix="compactc-target-") as temporary, \
            forge_circuits.test_environment(Path(temporary) / "forge"):
        base = Path(temporary)
        if args.witness_composition:
            check_witness_composition(compiler, base)
            return
        if args.stateful_assert:
            recording_checks.check_stateful_assert(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.wide_add:
            scalar_checks.check_wide_add(
                compiler, base,
                root=ROOT,
                run=run,
                capture=subprocess.run,
            )
            return
        if args.transient_receive_send:
            intent_checks.check_transient_receive_send(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.canonical_output_order:
            intent_checks.check_canonical_output_order(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.terminal_lexical_return:
            recording_checks.check_terminal_lexical_return(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.micro_dao_token or args.micro_dao_reveal or args.micro_dao_advance or args.micro_dao_set_topic or args.micro_dao_buy_in or args.micro_dao_cash_out or args.micro_dao_vote_commit:
            source = ROOT / "test-center/test-contracts/micro-dao.compact"
            selected = "vote_commit" if args.micro_dao_vote_commit else "cash_out" if args.micro_dao_cash_out else "buy_in" if args.micro_dao_buy_in else "set_topic" if args.micro_dao_set_topic else "advance" if args.micro_dao_advance else "vote_reveal" if args.micro_dao_reveal else "dao_voting_token"
            output = base / "micro-dao-selected"
            run(compiler, "--target", "rust", "--skip-zk", str(source), str(output))
            info = json.loads((output / "compiler/contract-info.json").read_text())
            report = json.loads((output / "contract/rust-capabilities.json").read_text())
            assert len(info["circuits"]) == 11 and sum(c["proof"] for c in info["circuits"]) == 7
            assert {c["name"] for c in report["circuits"]} == {c["name"] for c in info["circuits"] if c["proof"]}
            for c in report["circuits"]:
                assert c["proof_required"] and c["recorded"] == (c["name"] in {"advance", "vote_reveal", "dao_voting_token", "set_topic", "buy_in", "cash_out", "vote_commit"})
                assert c["observed_call"] == c["recorded"]
            strict = subprocess.run([compiler, "--target", "rust", "--rust-require-recording", "--skip-zk", str(source), str(base / "strict-dao")], cwd=ROOT, capture_output=True, text=True)
            assert strict.returncode == 0, strict.stderr
            if args.proof:
                (output / "keys").mkdir(exist_ok=True)
                run("zkir", "compile", str(output / f"zkir/{selected}.zkir"), str(output / f"keys/{selected}.prover"), str(output / f"keys/{selected}.verifier"))
                run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--micro-dao-vote-commit" if args.micro_dao_vote_commit else "--micro-dao-cash-out" if args.micro_dao_cash_out else "--micro-dao-buy-in" if args.micro_dao_buy_in else "--micro-dao-set-topic" if args.micro_dao_set_topic else "--micro-dao-advance" if args.micro_dao_advance else "--micro-dao-reveal" if args.micro_dao_reveal else "--micro-dao-token", str(output))
            print("original microDAO: advance, vote_reveal, dao_voting_token, set_topic, buy_in, cash_out and vote_commit recorded, no proof-required gaps retained")
            return
        if args.composite_zswap:
            intent_checks.check_composite_zswap(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.field_observation:
            recording_checks.check_field_observation(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.stateful_struct:
            recording_checks.check_stateful_struct(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
                capture=subprocess.run,
            )
            return
        if args.kernel_shielded_effects:
            intent_checks.check_kernel_shielded_effects(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
                capture=subprocess.run,
            )
            return
        if args.shielded_receive:
            shielded.check_receive(
                compiler, base, SHIELDED_RECEIVE_SOURCE, proof=args.proof, run=run
            )
            return
        if args.shielded_merge:
            shielded.check_merge(
                compiler, base, ROOT / "examples/rust_backend/shielded_merge_oracle.compact", proof=args.proof, run=run
            )
            return
        if args.shielded_send:
            shielded.check_send(
                compiler, base, SHIELDED_SEND_SOURCE, proof=args.proof, run=run
            )
            return
        if args.native_zswap_intents:
            intent_checks.check_native_zswap_intents(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.effectful_return:
            recording_checks.check_effectful_return(
                compiler, base,
                effectful_return_source=EFFECTFUL_RETURN_SOURCE,
                test_center_welcome_source=TEST_CENTER_WELCOME_SOURCE,
                test_center_bboard_source=TEST_CENTER_BBOARD_SOURCE,
                election_source=ELECTION_SOURCE,
                proof_requested=args.proof,
                run=run,
                check_coracle_source=check_coracle_source,
            )
            return
        if args.qualified_coin_cell:
            intent_checks.check_qualified_coin_cell(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.counter_less_than:
            scalar_checks.check_counter_less_than(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.literal_bytes_field:
            scalar_checks.check_literal_bytes_field(
                compiler, base,
                root=ROOT,
                proof_requested=args.proof,
                run=run,
                python_executable=sys.executable,
            )
            return
        if args.coracle_start:
            application_checks.check_coracle_start(
                compiler, base,
                proof_requested=args.proof,
                run=run,
                check_coracle_source=check_coracle_source,
            )
            return
        if args.coracle_concede:
            application_checks.check_coracle_concede(
                compiler, base,
                proof_requested=args.proof,
                run=run,
                check_coracle_source=check_coracle_source,
            )
            return
        if args.coracle_withdraw:
            application_checks.check_coracle_withdraw(
                compiler, base,
                proof_requested=args.proof,
                run=run,
                check_coracle_source=check_coracle_source,
            )
            return
        if args.coracle_guess:
            application_checks.check_coracle_guess(
                compiler, base,
                proof_requested=args.proof,
                run=run,
                check_coracle_source=check_coracle_source,
            )
            return
        if args.coracle_root_let:
            application_checks.check_coracle_root_let(
                compiler, base,
                root_let_action_return_source=ROOT_LET_ACTION_RETURN_SOURCE,
                test_center_welcome_source=TEST_CENTER_WELCOME_SOURCE,
                proof_requested=args.proof,
                run=run,
                check_coracle_source=check_coracle_source,
            )
            return
        if args.adt_set_qualified:
            intent_checks.check_adt_set_qualified(
                compiler, base,
                root=ROOT,
                adt_set_qualified_source=ADT_SET_QUALIFIED_SOURCE,
                qualified_coin_oracle_source=QUALIFIED_COIN_ORACLE_SOURCE,
                proof_requested=args.proof,
                run=run,
                capture=subprocess.run,
            )
            return
        if args.test_center_bboard:
            application_checks.check_test_center_bboard(
                compiler, base,
                test_center_bboard_source=TEST_CENTER_BBOARD_SOURCE,
                proof_requested=args.proof,
                run=run,
            )
            return
        if args.field_to_bytes32:
            scalar_checks.check_field_to_bytes32(
                compiler, base,
                root=ROOT,
                field_to_bytes32_source=FIELD_TO_BYTES32_SOURCE,
                proof_requested=args.proof,
                run=run,
                capture=subprocess.run,
            )
            return
        run(sys.executable, str(Path(__file__).resolve()), "--qualified-coin-cell",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--native-zswap-intents",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--kernel-shielded-effects",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--shielded-receive",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--shielded-merge",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--shielded-send",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--stateful-struct")
        run(sys.executable, str(Path(__file__).resolve()), "--field-observation",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--composite-zswap",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--micro-dao-token",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--micro-dao-reveal",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--transient-receive-send",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--canonical-output-order",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--terminal-lexical-return",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--micro-dao-advance",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--micro-dao-set-topic",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--micro-dao-buy-in",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--micro-dao-cash-out",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--micro-dao-vote-commit",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--coracle-guess",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--coracle-concede",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--coracle-start",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--coracle-withdraw",
            *(["--proof"] if args.proof else []))
        run(sys.executable, str(Path(__file__).resolve()), "--wide-add")
        run(sys.executable, str(Path(__file__).resolve()), "--stateful-assert")
        run(sys.executable, str(Path(__file__).resolve()), "--witness-composition")
        native_outputs = native_targets.check(
            compiler, base,
            root=ROOT, counter_source=SOURCE, pure_source=PURE_SOURCE,
            witness_cell_source=WITNESS_CELL_SOURCE,
            constructor_map_source=CONSTRUCTOR_MAP_SOURCE,
            alias_source=ALIAS_SOURCE, pm_19252_source=PM_19252_SOURCE,
            native_key_value_source=NATIVE_KEY_VALUE_SOURCE,
            run=run, capture=subprocess.run, check_manifest=check_manifest,
        )
        if args.consumer:
            run(compiler, "--target", "rust", "--skip-zk", str(PURE_SOURCE), str(native_outputs.pure))
            check_consumer(native_outputs.legacy_rust / "contract", native_outputs.pure / "contract", base / "consumer")
            check_shared_runtime_consumer(compiler, base)
            check_witness_consumer(compiler, base)
            check_merkle_witness_consumer(compiler, base)
            check_list_shapes_consumer(compiler, base)
        if args.proof:
            proof = base / "proof"
            run(compiler, "--target", "rust", "--rust-require-recording", str(SOURCE), str(proof))
            check_manifest(proof)
            for circuit in ("increment", "read_round"):
                proof_artifacts.require_circuit_files(proof, circuit)
            check_observed_call_consumer(proof, base)
            test_center_counter_proof = base / "test-center-counter-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(TEST_CENTER_COUNTER_SOURCE), str(test_center_counter_proof))
            check_manifest(test_center_counter_proof)
            test_center_capabilities = json.loads(
                (test_center_counter_proof / "contract/rust-capabilities.json").read_text())
            assert [(row["name"], row["proof_required"], row["recording_status"])
                    for row in test_center_capabilities["circuits"]] == [
                ("increment", True, "available")]
            proof_artifacts.require_circuit_files(test_center_counter_proof, 'increment')
            counter_parameter_proof = base / "counter-parameter-proof"
            run(compiler, "--target", "rust", "--rust-require-recording", str(COUNTER_PARAMETER_SOURCE), str(counter_parameter_proof))
            check_manifest(counter_parameter_proof)
            for circuit in ("increment_by", "reset_round"):
                proof_artifacts.require_circuit_files(counter_parameter_proof, circuit)
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
                    proof_artifacts.require_circuit_files(unsigned_proof, circuit)
            cell_proof = base / "cell-proof"
            run(compiler, "--target", "rust", str(CELL_SOURCE), str(cell_proof))
            check_manifest(cell_proof)
            proof_artifacts.require_circuit_files(cell_proof, 'set_flag')
            root_let_proof = base / "root-let-action-return-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(ROOT_LET_ACTION_RETURN_SOURCE), str(root_let_proof))
            run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--root-let-action-return", str(root_let_proof))
            struct_proof = base / "stateful-struct-proof"
            run(compiler,"--target","rust",str(ROOT / "examples/rust_backend/stateful_struct_oracle.compact"),str(struct_proof))
            run("cargo","run","--quiet","-p","compact-rust-proof-smoke","--","--stateful-struct",str(struct_proof))
            assertion_proof = base / "stateful-assert-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(ROOT / "examples/rust_backend/stateful_assert_oracle.compact"), str(assertion_proof))
            run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--", "--stateful-assert", str(assertion_proof))
            effectful_proof = base / "effectful-return-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(EFFECTFUL_RETURN_SOURCE), str(effectful_proof))
            run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--effectful-return", str(effectful_proof))
            counter_less_than_proof = base / "counter-less-than-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(ROOT / "examples/rust_backend/counter_less_than_oracle.compact"), str(counter_less_than_proof))
            run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--counter-less-than", str(counter_less_than_proof))
            literal_bytes_field_proof = base / "literal-bytes-field-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(ROOT / "examples/rust_backend/literal_bytes_field_oracle.compact"),
                str(literal_bytes_field_proof))
            run("cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--literal-bytes-field", str(literal_bytes_field_proof))
            cell_read_proof = base / "cell-read-proof"
            run(compiler, "--target", "rust", str(CELL_READ_SOURCE), str(cell_read_proof))
            check_manifest(cell_read_proof)
            proof_artifacts.require_circuit_files(cell_read_proof, 'read_flag')
            composite_cell_proof = base / "composite-cell-proof"
            run(compiler, "--target", "rust", "--rust-require-recording", str(CELL_STRUCT_SOURCE), str(composite_cell_proof))
            check_manifest(composite_cell_proof)
            for circuit in ("set_record", "read_record"):
                proof_artifacts.require_circuit_files(composite_cell_proof, circuit)
            witness_proof = base / "witness-proof"
            run(compiler, "--target", "rust", str(WITNESS_CELL_SOURCE), str(witness_proof))
            check_manifest(witness_proof)
            check_observed_witness_call_consumer(witness_proof, base)
            for circuit in ("write_twice", "write_offset", "write_nested_twice"):
                proof_artifacts.require_circuit_files(witness_proof, circuit)
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
            proof_artifacts.require_circuit_files(assert_witness_proof, 'checked_write')
            nested_proof = base / "nested-proof"
            run(compiler, "--target", "rust", str(NESTED_COUNTER_SOURCE), str(nested_proof))
            check_manifest(nested_proof)
            for circuit in ("bump_twice", "add_twice"):
                proof_artifacts.require_circuit_files(nested_proof, circuit)
            nested_witness_proof = base / "nested-witness-proof"
            run(compiler, "--target", "rust", str(NESTED_WITNESS_SOURCE), str(nested_witness_proof))
            check_manifest(nested_witness_proof)
            for circuit in ("outer", "outerValue", "outerValue2", "outerValueExpr"):
                proof_artifacts.require_circuit_files(nested_witness_proof, circuit)
            collections = collection_proofs.prepare(
                compiler, base,
                set_source=SET_BOOLEAN_SOURCE,
                set_oracle_source=SET_SOURCE,
                map_source=MAP_BOOLEAN_SOURCE,
                constructor_map_source=CONSTRUCTOR_MAP_SOURCE,
                list_source=LIST_SOURCE,
                list_shapes_source=LIST_SHAPES_SOURCE,
                run=run, check_manifest=check_manifest,
                check_observed_map_call_consumer=check_observed_map_call_consumer,
            )
            zerocash_mint_proof = base / "zerocash-mint-proof"
            run(compiler, "--target", "rust", str(ZEROCASH_SOURCE), str(zerocash_mint_proof))
            check_manifest(zerocash_mint_proof)
            capabilities = json.loads((zerocash_mint_proof / "contract/rust-capabilities.json").read_text())
            for name in ("zerocash_mint", "spend"):
                circuit = next(row for row in capabilities["circuits"] if row["name"] == name)
                assert circuit["proof_required"] and circuit["recorded"] and circuit["observed_call"]
                proof_artifacts.require_circuit_files(zerocash_mint_proof, name)
            bboard_proof = base / "bboard-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(TEST_CENTER_BBOARD_SOURCE), str(bboard_proof))
            check_manifest(bboard_proof)
            election_topic_proof = base / "election-topic-proof"
            run(compiler, "--target", "rust", str(ELECTION_SOURCE), str(election_topic_proof))
            check_manifest(election_topic_proof)
            election_capabilities = json.loads(
                (election_topic_proof / "contract/rust-capabilities.json").read_text()
            )
            for circuit in ("set_topic", "advance", "add_voter", "vote$commit", "vote$reveal"):
                row = next(row for row in election_capabilities["circuits"] if row["name"] == circuit)
                assert row["proof_required"] and row["recorded"] and row["observed_call"]
                proof_artifacts.require_circuit_files(election_topic_proof, circuit)
            inline_type_scope_proof = base / "inline-type-scope-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(INLINE_TYPE_SCOPE_SOURCE), str(inline_type_scope_proof))
            check_manifest(inline_type_scope_proof)
            capabilities = json.loads(
                (inline_type_scope_proof / "contract/rust-capabilities.json").read_text()
            )
            expected_inline = {
                "setHash", "checkScalarScope", "checkAggScope", "checkNoCollisionScope"
            }
            assert {row["name"] for row in capabilities["circuits"]} == expected_inline
            for circuit in capabilities["circuits"]:
                assert circuit["proof_required"] and circuit["recorded"] \
                    and circuit["observed_call"]
                if circuit["name"] != "setHash":
                    proof_artifacts.require_circuit_files(inline_type_scope_proof, circuit['name'])
            merkle_proof = base / "merkle-proof"
            run(compiler, "--target", "rust", "--rust-require-recording", str(MERKLE_SOURCE), str(merkle_proof))
            check_manifest(merkle_proof)
            capabilities = json.loads((merkle_proof / "contract/rust-capabilities.json").read_text())
            by_name = {circuit["name"]: circuit for circuit in capabilities["circuits"]}
            for circuit in ("append", "place", "place_default", "full", "append_hash", "place_hash", "known", "reset_tree"):
                assert by_name[circuit]["recorded"] and by_name[circuit]["observed_call"]
                proof_artifacts.require_circuit_files(merkle_proof, circuit)
            merkle_verify_proof = base / "merkle-verify-proof"
            run(compiler, "--target", "rust", str(MERKLE_VERIFY_SOURCE), str(merkle_verify_proof))
            check_manifest(merkle_verify_proof)
            capabilities = json.loads((merkle_verify_proof / "contract/rust-capabilities.json").read_text())
            by_name = {circuit["name"]: circuit for circuit in capabilities["circuits"]}
            for circuit in ("verify", "replace"):
                assert by_name[circuit]["recorded"] and by_name[circuit]["observed_call"]
                proof_artifacts.require_circuit_files(merkle_verify_proof, circuit)
            persistent_commit_proof = base / "persistent-commit-proof"
            run(compiler, "--target", "rust", str(PERSISTENT_COMMIT_SOURCE),
                str(persistent_commit_proof))
            check_manifest(persistent_commit_proof)
            capabilities = json.loads(
                (persistent_commit_proof / "contract/rust-capabilities.json").read_text()
            )
            for name in ("commitSmall", "commitU128", "commitFieldOnly",
                         "pureBodyFieldOnly", "pureBodyVec",
                         "bridgeTupleIntoVec", "bridgeVecIntoTuple",
                         "witnessBare",
                         "impureConst",
                         "hashPersistentVec", "hashTransientVec",
                         "pureFromImpure", "impureBare", "impureInIfArm",
                         "inlinedAssert"):
                capability = next(circuit for circuit in capabilities["circuits"]
                                  if circuit["name"] == name)
                assert capability["proof_required"] and capability["recorded"] \
                    and capability["observed_call"]
            proof_artifacts.require_circuit_files(persistent_commit_proof, 'commitSmall')
            proof_artifacts.require_circuit_files(persistent_commit_proof, 'bridgeTupleIntoVec')
            proof_artifacts.require_circuit_files(persistent_commit_proof, 'witnessBare')
            proof_artifacts.require_circuit_files(persistent_commit_proof, 'impureConst')
            for name in ("impureBare", "impureInIfArm", "inlinedAssert",
                         "hashPersistentVec", "hashTransientVec"):
                proof_artifacts.require_circuit_files(persistent_commit_proof, name)
            witness_vector_action_proof = base / "witness-vector-action-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(WITNESS_VECTOR_ACTION_SOURCE), str(witness_vector_action_proof))
            check_manifest(witness_vector_action_proof)
            capabilities = json.loads(
                (witness_vector_action_proof / "contract/rust-capabilities.json").read_text()
            )
            discard = next(circuit for circuit in capabilities["circuits"]
                           if circuit["name"] == "discardResult")
            assert discard["proof_required"] and discard["recorded"] \
                and discard["observed_call"]
            proof_artifacts.require_circuit_files(witness_vector_action_proof, 'discardResult')
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
            proof_artifacts.require_circuit_files(internal_pure_call_proof, 'save')
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
            proof_artifacts.require_circuit_files(stateful_pure_call_proof, 'save')
            unsigned_recording_proofs = {}
            for label, source, circuit in (
                ("field-cast-uint128", FIELD_CAST_UINT128_SOURCE, "save"),
                ("widening-arith", WIDENING_ARITH_SOURCE, "recordArea"),
                ("mixed-width-operand", MIXED_WIDTH_OPERAND_SOURCE, "recordMatching"),
            ):
                proof_output = base / f"{label}-proof"
                run(compiler, "--target", "rust", "--rust-require-recording", str(source), str(proof_output))
                check_manifest(proof_output)
                capabilities = json.loads((proof_output / "contract/rust-capabilities.json").read_text())
                capability = next(row for row in capabilities["circuits"] if row["name"] == circuit)
                assert capability["proof_required"] and capability["recorded"] and capability["observed_call"]
                proof_artifacts.require_circuit_files(proof_output, circuit)
                unsigned_recording_proofs[label] = proof_output
            ternary_cond_proof = base / "ternary-cond-proof"
            run(compiler, "--target", "rust", str(TERNARY_COND_SOURCE), str(ternary_cond_proof))
            check_manifest(ternary_cond_proof)
            capabilities = json.loads(
                (ternary_cond_proof / "contract/rust-capabilities.json").read_text()
            )
            for name in ("walkerConstAnnotated", "walkerWrite", "walkerCallPure", "streamCallPure",
                         "witnessArg", "streamCallWitness", "streamAssertEq",
                         "walkerNestedIf", "streamNestedIf",
                         "walkerNativeArg", "streamNativeArg", "streamVectorElement"):
                circuit = next(circuit for circuit in capabilities["circuits"]
                               if circuit["name"] == name)
                assert circuit["proof_required"] and circuit["recorded"] \
                    and circuit["observed_call"]
            for name in ("walkerConstAnnotated", "walkerWrite", "streamCallPure", "streamCallWitness", "streamAssertEq",
                         "walkerNestedIf", "streamNestedIf",
                         "walkerNativeArg", "streamNativeArg", "streamVectorElement"):
                proof_artifacts.require_circuit_files(ternary_cond_proof, name)
            nested_stateful_ternary_proof = base / "nested-stateful-ternary-proof"
            run(compiler, "--target", "rust", str(NESTED_STATEFUL_TERNARY_SOURCE),
                str(nested_stateful_ternary_proof))
            check_manifest(nested_stateful_ternary_proof)
            capabilities = json.loads(
                (nested_stateful_ternary_proof / "contract/rust-capabilities.json").read_text()
            )
            run_capability = next(row for row in capabilities["circuits"]
                                  if row["name"] == "run")
            assert run_capability["proof_required"] and run_capability["recorded"] \
                and run_capability["observed_call"]
            proof_artifacts.require_circuit_files(nested_stateful_ternary_proof, 'run')
            vector_map_proofs = []
            for stem in ("map_fn_oracle", "map_lambda_oracle"):
                output = base / f"{stem}-proof"
                run(compiler, "--target", "rust", "--rust-require-recording",
                    str(ROOT / "examples/rust_backend" / f"{stem}.compact"), str(output))
                check_manifest(output)
                proof_artifacts.require_circuit_files(output, 'ping')
                vector_map_proofs.append(output)
            unused_field_read_proofs = []
            for name in ("seven", "eight_a", "eight_b"):
                output = base / f"unused-field-read-{name}-proof"
                source = ROOT / "examples/bugs/pm-19252" / f"example_{name}.compact"
                run(compiler, "--target", "rust", "--rust-require-recording", str(source), str(output))
                check_manifest(output)
                capabilities = json.loads((output / "contract/rust-capabilities.json").read_text())
                assert all(row["recorded"] and row["observed_call"] for row in capabilities["circuits"])
                unused_field_read_proofs.append(output)
            let_return_proof = base / "let-return-proof"
            run(compiler, "--target", "rust", str(LET_RETURN_SOURCE), str(let_return_proof))
            check_manifest(let_return_proof)
            capabilities = json.loads(
                (let_return_proof / "contract/rust-capabilities.json").read_text()
            )
            replace_capability = next(row for row in capabilities["circuits"]
                                      if row["name"] == "replace")
            assert replace_capability["proof_required"] and replace_capability["recorded"] \
                and replace_capability["observed_call"]
            proof_artifacts.require_circuit_files(let_return_proof, 'replace')
            asset_writable_proof = base / "asset-writable-proof"
            run(compiler, "--target", "rust", str(ASSET_REGISTRY_SOURCE),
                str(asset_writable_proof))
            check_manifest(asset_writable_proof)
            capabilities = json.loads(
                (asset_writable_proof / "contract/rust-capabilities.json").read_text()
            )
            for circuit in ("setCustodian", "setRecord", "setCustodyGrant", "setWatch", "tag", "removeRecord", "assertGrantEffective",
                            "assertStoredRecordFresh"):
                capability = next(item for item in capabilities["circuits"]
                                  if item["name"] == circuit)
                assert capability["proof_required"] and capability["recorded"] \
                    and capability["observed_call"]
                proof_artifacts.require_circuit_files(asset_writable_proof, circuit)
            schnorr_proof = base / "schnorr-attestation-proof"
            run(compiler, "--target", "rust", "--rust-require-recording",
                str(SCHNORR_ATTEST_SOURCE), str(schnorr_proof))
            check_manifest(schnorr_proof)
            capabilities = json.loads((schnorr_proof / "contract/rust-capabilities.json").read_text())
            for circuit in ("verifyAttestation", "acceptAttestation"):
                capability = next(row for row in capabilities["circuits"] if row["name"] == circuit)
                assert capability["proof_required"] and capability["recorded"] \
                    and capability["observed_call"]
                proof_artifacts.require_circuit_files(schnorr_proof, circuit)
            historic_merkle_proof = base / "historic-merkle-proof"
            run(compiler, "--target", "rust", str(HISTORIC_MERKLE_SOURCE), str(historic_merkle_proof))
            check_manifest(historic_merkle_proof)
            capabilities = json.loads((historic_merkle_proof / "contract/rust-capabilities.json").read_text())
            by_name = {circuit["name"]: circuit for circuit in capabilities["circuits"]}
            for circuit in ("append", "place", "append_hash", "place_hash", "forget_history", "reset_tree", "full", "known"):
                assert by_name[circuit]["recorded"] and by_name[circuit]["observed_call"]
                proof_artifacts.require_circuit_files(historic_merkle_proof, circuit)
            historic_merkle_default_proof = base / "historic-merkle-default-proof"
            run(compiler, "--target", "rust", str(HISTORIC_MERKLE_DEFAULT_SOURCE), str(historic_merkle_default_proof))
            check_manifest(historic_merkle_default_proof)
            capabilities = json.loads((historic_merkle_default_proof / "contract/rust-capabilities.json").read_text())
            assert capabilities["circuits"][0]["name"] == "add_default"
            assert capabilities["circuits"][0]["recorded"] and capabilities["circuits"][0]["observed_call"]
            proof_artifacts.require_circuit_files(historic_merkle_default_proof, 'add_default')
            vector_key_proof = base / "vector-key-proof"
            run(compiler, "--target", "rust", str(VECTOR_KEY_SOURCE), str(vector_key_proof))
            check_manifest(vector_key_proof)
            proof_artifacts.require_circuit_files(vector_key_proof, 'setInsert')
            composite_key_proof = base / "composite-key-proof"
            run(compiler, "--target", "rust", str(COMPOSITE_KEY_SOURCE), str(composite_key_proof))
            check_manifest(composite_key_proof)
            for circuit in (
                "insert_vector", "insert_tuple", "insert_struct",
                "roundtrip_tuple", "roundtrip_struct", "record_twelve",
            ):
                proof_artifacts.require_circuit_files(composite_key_proof, circuit)
            check_observed_composite_key_consumer(composite_key_proof, base)
            chunked_set_proof = base / "chunked-set-proof"
            run(compiler, "--target", "rust", str(CHUNKED_SET_SOURCE), str(chunked_set_proof))
            check_manifest(chunked_set_proof)
            for circuit in ("insert_key", "roundtrip_key", "key_count", "empty"):
                proof_artifacts.require_circuit_files(chunked_set_proof, circuit)
            check_chunked_set_consumer(chunked_set_proof, base)
            chunked_list_proof = base / "chunked-list-proof"
            run(compiler, "--target", "rust", str(CHUNKED_LIST_SOURCE), str(chunked_list_proof))
            check_manifest(chunked_list_proof)
            for circuit in (
                "item_count", "items_empty", "first_item", "prepend", "drop_first", "clear_items",
            ):
                proof_artifacts.require_circuit_files(chunked_list_proof, circuit)
            check_chunked_list_consumer(chunked_list_proof, base)
            chunked_map_proof = base / "chunked-map-proof"
            run(compiler, "--target", "rust", str(CHUNKED_MAP_SOURCE), str(chunked_map_proof))
            check_manifest(chunked_map_proof)
            for circuit in (
                "put", "put_pair", "put_default", "has", "get", "remove_key",
                "table_size", "table_is_empty", "reset_table",
            ):
                proof_artifacts.require_circuit_files(chunked_map_proof, circuit)
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
                proof_artifacts.require_circuit_files(chunked_cell_proof, circuit)
            check_chunked_cell_consumer(chunked_cell_proof, base)
            constructor_list_proof = base / "constructor-list-proof"
            run(compiler, "--target", "rust", str(CONSTRUCTOR_LIST_SOURCE), str(constructor_list_proof))
            check_manifest(constructor_list_proof)
            for circuit in ("item_count", "history_count", "first_item", "drop_first", "clear_items"):
                proof_artifacts.require_circuit_files(constructor_list_proof, circuit)
            enum_cell_proof = base / "enum-cell-proof"
            run(compiler, "--target", "rust", str(RECORDED_ENUM_SOURCE), str(enum_cell_proof))
            check_manifest(enum_cell_proof)
            proof_artifacts.require_circuit_files(enum_cell_proof, 'choose')
            tiny_proof = base / "tiny-proof"
            run(compiler, "--target", "rust", str(TINY_SOURCE), str(tiny_proof))
            check_manifest(tiny_proof)
            for circuit in ("clear", "set", "get"):
                proof_artifacts.require_circuit_files(tiny_proof, circuit)
            nested_map_shape_proof = base / "nested-map-shape-proof"
            run(compiler, "--target", "rust", str(NESTED_MAP_SHAPE_SOURCE), str(nested_map_shape_proof))
            check_manifest(nested_map_shape_proof)
            proof_artifacts.require_circuit_files(nested_map_shape_proof, 'check_nested_empty')
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                str(proof), str(cell_proof), str(cell_read_proof), str(witness_proof), str(nested_proof),
                str(nested_witness_proof), str(collections.set), str(collections.set_oracle),
                str(collections.map), str(collections.constructor_map), str(collections.list), str(constructor_list_proof),
                str(enum_cell_proof),
                str(tiny_proof),
                str(nested_map_shape_proof),
                str(collections.list_shapes),
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
                "--test-center-counter", str(test_center_counter_proof),
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
                "--merkle-indexed", str(merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--merkle-hash", str(merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--merkle-root", str(merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--inline-type-scope", str(inline_type_scope_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--election-topic", str(election_topic_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--election-advance", str(election_topic_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--election-add-voter", str(election_topic_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--zerocash-mint", str(zerocash_mint_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--zerocash-spend", str(zerocash_mint_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--election-commit", str(election_topic_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--election-reveal", str(election_topic_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--bboard", str(bboard_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--merkle-reset", str(merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--historic-merkle-indexed", str(historic_merkle_proof), str(historic_merkle_default_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--historic-merkle-hash", str(historic_merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--historic-merkle-indexed-hash", str(historic_merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--historic-merkle-reset-history", str(historic_merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--historic-merkle-reset-tree", str(historic_merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--historic-merkle-known", str(historic_merkle_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--persistent-commit", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--witness-vector-bare", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--witness-vector-discard", str(witness_vector_action_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--closed-pure-field", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--field-pair-hash", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--impure-field-helper", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--pair-hash-cell", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--stateful-pair-hash", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--boolean-pair-assert", str(persistent_commit_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--conditional-field", str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--conditional-counter", str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--unsigned-sequential", str(unsigned_proofs["cross-circuit"]),
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
                "--nested-uint4", str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--nested-uint64-cell-counter", str(nested_stateful_ternary_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--annotated-uint8", str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--conditional-field-vector", str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--native-curve-arg", str(ternary_cond_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--conditional-set", str(collections.set),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--asset-writable", str(asset_writable_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--asset-custody-grant", str(asset_writable_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--asset-record-write", str(asset_writable_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--asset-watch-write", str(asset_writable_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--schnorr-attestation", str(schnorr_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--asset-removal", str(asset_writable_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--let-return", str(let_return_proof),
            )
            run("cargo", "run", "--quiet", "--locked", "-p", "compact-rust-proof-smoke", "--",
                "--unused-field-reads", *(str(output) for output in unused_field_read_proofs))
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--vector-map", *(str(output) for output in vector_map_proofs),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--asset-grant-effective", str(asset_writable_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--stateful-pure-return", str(stateful_pure_call_proof),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--wide-unsigned-field-cast", str(unsigned_recording_proofs["field-cast-uint128"]),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--closed-pure-unsigned-call", str(unsigned_recording_proofs["widening-arith"]),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--mixed-width-recording", str(unsigned_recording_proofs["mixed-width-operand"]),
            )
            run(
                "cargo", "run", "--quiet", "-p", "compact-rust-proof-smoke", "--",
                "--assert-witness", str(assert_witness_proof),
            )
    print("compactc target boundary and manifest: passed")


if __name__ == "__main__":
    main()
