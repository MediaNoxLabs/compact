// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// `passing a unit value to a function` fires throughout these tests
// because PS = () (no private state) on this fixture, mirroring the
// suppression used by tiny.rs.
#![allow(clippy::unit_arg)]
//
// set_size_fixture.compact byte-parity test — validates A20 (read-no-arg
// adt-op vm-code lowering). Before A20 the codegen emitted a hardcoded
// `dup → idx → popeq` template for every no-arg ADT read regardless of
// the underlying vm-code, silently miscompiling Set.size / Set.isEmpty /
// Map.size / Map.isEmpty / List.isEmpty / List.length to read the
// container cell as the result AlignedValue. A20 routes the adt-op's
// actual vm-code through `expand-vm-code` so the gather chain reflects
// the real instruction sequence — for both isEmpty calls below:
//
//   `.dup(0).idx_at_index(N,false).size().push(false, new_cell(0u64))
//    .eq().popeq(true)`
//
// `check_set_empty` and `check_map_empty` exercise the same chain on
// different containers (Set vs Map), confirming A20 covers both. A
// pre-A20 regression would prevent the Boolean flag from flipping
// correctly (either the gather chain crashes on Cell-vs-Map type
// mismatch or it decodes the wrong AlignedValue), surfacing as a byte
// mismatch on the post-circuit step.

use compact_contract_set_size_fixture::Contract;
use compact_runtime::*;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct StepSnapshot {
    #[serde(rename = "stateHex")]
    state_hex: String,
}

impl StepSnapshot {
    fn state_bytes(&self) -> Vec<u8> {
        hex::decode(&self.state_hex).expect("decode hex")
    }
}

#[derive(Deserialize, Debug)]
struct SetSizeFixtureTsState {
    #[serde(rename = "afterInit")]
    after_init: StepSnapshot,
    #[serde(rename = "afterCheckSetEmpty")]
    after_check_set_empty: StepSnapshot,
    #[serde(rename = "afterCheckMapEmpty")]
    after_check_map_empty: StepSnapshot,
}

impl SetSizeFixtureTsState {
    fn load(path: impl AsRef<Path>) -> Self {
        let raw = std::fs::read_to_string(path).expect("read fixture");
        serde_json::from_str(&raw).expect("parse fixture")
    }
}

fn fixture() -> SetSizeFixtureTsState {
    SetSizeFixtureTsState::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/set-size-fixture-ts-state.json"
    ))
}

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

/// Build a ContractState envelope around a (possibly mutated) ChargedState.
/// set_size_fixture exports two circuits — `check_set_empty` and
/// `check_map_empty` — so the operations map must register both for the
/// envelope to serialize to the same bytes as the TS reference (which
/// derives the operations map from the same `@circuit` annotations).
fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, midnight_storage::DefaultDB> =
        HashMap::new();
    operations = operations.insert(
        EntryPointBuf(b"check_set_empty".to_vec()),
        ContractOperation::new(None),
    );
    operations = operations.insert(
        EntryPointBuf(b"check_map_empty".to_vec()),
        ContractOperation::new(None),
    );
    ContractState {
        data,
        operations,
        maintenance_authority: ContractMaintenanceAuthority::default(),
        balance: Default::default(),
    }
}

fn assert_step_bytes_eq(
    label: &str,
    state: &ContractState<midnight_storage::DefaultDB>,
    expected: &StepSnapshot,
) {
    let mut buf = Vec::new();
    tagged_serialize(state, &mut buf).expect("tagged_serialize");
    let ts_bytes = expected.state_bytes();
    assert_eq!(
        buf,
        ts_bytes,
        "[{label}] Rust state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        buf.len(),
        hex::encode(&buf),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

#[test]
fn set_size_fixture_init_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");
    let envelope = make_envelope(result.current_contract_state.clone());
    assert_step_bytes_eq("init", &envelope, &ts_ref.after_init);
}

#[test]
fn set_size_fixture_check_set_empty_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let circ_ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);
    let checked = contract.check_set_empty(circ_ctx).expect("check_set_empty");
    let envelope = make_envelope(checked.context.current_query_context.state.clone());
    assert_step_bytes_eq("check_set_empty", &envelope, &ts_ref.after_check_set_empty);
}

#[test]
fn set_size_fixture_check_map_empty_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let circ_ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);
    let after_set = contract.check_set_empty(circ_ctx).expect("check_set_empty");
    let after_map = contract
        .check_map_empty(after_set.context)
        .expect("check_map_empty");
    let envelope = make_envelope(after_map.context.current_query_context.state.clone());
    assert_step_bytes_eq("check_map_empty", &envelope, &ts_ref.after_check_map_empty);
}
