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
// suppression used by set_size_fixture.rs.
#![allow(clippy::unit_arg)]
//
// hmt_default_fixture.compact byte-parity test — validates A21
// (HistoricMerkleTree.insertIndexDefault circuit-body shape). Before
// A21 the codegen rejected any circuit body that called
// `t.insertIndexDefault(idx)` with `circuit-body-emission: no walker
// shape matched`, because the vm-code contains the bounded-index
// branch sequence (`lt`, `branch`, `jmp`, `swap`, `pop`) and a
// `(rt-null value_type)` default that the `vminstr->builder-call` and
// `vm-cell-elem->rust` lowerings hadn't seen before.
//
// After A21, the I3a path renders the full chain end-to-end:
//
//   .idx_at_index(0u8, true)   // descend into HMT array
//   .idx_at_index(0u8, true)   // descend into the BoundedMerkleTree
//   .push(false, new_cell(idx))
//   .push(true, new_cell(leaf_hash(default leaf)))
//   .ins(false, 2)
//   .idx_at_index(1u8, true)   // descend into first_free Cell
//   .push(false, new_cell(idx))
//   .addi(1)
//   .dup(1).dup(1).lt()        // index + 1 > old_first_free?
//   .branch(2).pop().jmp(2)    //   if yes: drop old counter
//   .swap(0).pop()             //   if no:  swap-then-drop new candidate
//   .ins(false, 1)
//   .idx_at_index(2u8, true).dup(2).idx_at_index(0u8, false).root()
//   .push(true, StateValue::Null).ins(false, 1).ins(true, 2)
//
// Three snapshots — init → after add_default(0) → after add_default(2)
// — exercise the post-init taken-branch arm twice (once from zero, once
// from one) so a divergence between Rust and TS surfaces at the
// specific step that triggered it. A pre-A21 regression would prevent
// compilation; a post-A21 regression on any of the new ops would
// surface as a byte mismatch at step 2 or step 3.

use compact_contract_hmt_default_fixture::Contract;
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
struct HmtDefaultFixtureTsState {
    #[serde(rename = "afterInit")]
    after_init: StepSnapshot,
    #[serde(rename = "afterAddDefault0")]
    after_add_default_0: StepSnapshot,
    #[serde(rename = "afterAddDefault2")]
    after_add_default_2: StepSnapshot,
}

impl HmtDefaultFixtureTsState {
    fn load(path: impl AsRef<Path>) -> Self {
        let raw = std::fs::read_to_string(path).expect("read fixture");
        serde_json::from_str(&raw).expect("parse fixture")
    }
}

fn fixture() -> HmtDefaultFixtureTsState {
    HmtDefaultFixtureTsState::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/hmt-default-fixture-ts-state.json"
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

/// Build a `ContractState` envelope around a (possibly mutated) `ChargedState`.
/// `hmt_default_fixture` exports a single circuit — `add_default` — so the
/// operations map registers just that entry to match the TS reference
/// (which derives the operations map from the same `@circuit`
/// annotations).
fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, midnight_storage::DefaultDB> =
        HashMap::new();
    operations = operations.insert(
        EntryPointBuf(b"add_default".to_vec()),
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
fn hmt_default_fixture_init_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");
    let envelope = make_envelope(result.current_contract_state.clone());
    assert_step_bytes_eq("init", &envelope, &ts_ref.after_init);
}

#[test]
fn hmt_default_fixture_add_default_0_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let circ_ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);
    let after_0 = contract
        .add_default(circ_ctx, 0u64)
        .expect("add_default(0)");
    let envelope = make_envelope(after_0.context.current_query_context.state.clone());
    assert_step_bytes_eq(
        "after_add_default_0",
        &envelope,
        &ts_ref.after_add_default_0,
    );
}

#[test]
fn hmt_default_fixture_add_default_2_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let circ_ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);
    let after_0 = contract
        .add_default(circ_ctx, 0u64)
        .expect("add_default(0)");
    let after_2 = contract
        .add_default(after_0.context, 2u64)
        .expect("add_default(2)");
    let envelope = make_envelope(after_2.context.current_query_context.state.clone());
    assert_step_bytes_eq(
        "after_add_default_2",
        &envelope,
        &ts_ref.after_add_default_2,
    );
}
