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

// PS = () on this fixture (no private state), mirroring tiny/set-size.
#![allow(clippy::unit_arg)]
//
// bug11_fixture.compact byte-parity test — locks down Bug-11 codegen.
//
// Bug-11 (2026-06-29): pre-fix the Rust walker rejected any non-literal
// write into a `Uint<L..U>` ledger field whose upper bound `U` lay off
// the power-of-2-minus-1 ladder (255/65535/u32::MAX/u64::MAX/u128::MAX).
// The Compact language spec sanctions arbitrary `Uint<L..U>` upper
// bounds, and the TS backend handles them via
// `CompactTypeUnsignedInteger(maxValue, byte_len).toValue(v)` with
// arbitrary `byte_len`.
//
// This fixture drives three write circuits in sequence, each writing
// into a field with a different on-state byte_len:
//
//   1. set_tiny(42)         — `Uint<0..100>`, byte_len=1 (matches u8)
//   2. set_medium(65_000)   — `Uint<0..70000>`, byte_len=3 (non-pow2)
//   3. set_wide(4_500_000_000) — `Uint<0..5_000_000_000>`, byte_len=5 (non-pow2)
//
// Byte-parity is asserted against a TS-produced reference at every step.
// A regression that re-broke the walker rejection would fail the test
// at the post-init step (since the constructor seeds two
// `new_cell_bounded_uint` cells), and a regression that picked the
// wrong byte_len on the write path would surface on step 2 or 3.

use compact_contract_bug11_fixture::Contract;
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
struct Bug11FixtureTsState {
    #[serde(rename = "afterInit")]
    after_init: StepSnapshot,
    #[serde(rename = "afterSetTiny")]
    after_set_tiny: StepSnapshot,
    #[serde(rename = "afterSetMedium")]
    after_set_medium: StepSnapshot,
    #[serde(rename = "afterSetWide")]
    after_set_wide: StepSnapshot,
}

impl Bug11FixtureTsState {
    fn load(path: impl AsRef<Path>) -> Self {
        let raw = std::fs::read_to_string(path).expect("read fixture");
        serde_json::from_str(&raw).expect("parse fixture")
    }
}

fn fixture() -> Bug11FixtureTsState {
    Bug11FixtureTsState::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/bug11-fixture-ts-state.json"
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

/// `bug11_fixture` exports three circuits — `set_tiny`, `set_medium`,
/// `set_wide`. The TS reference's serialised state depends on the
/// operations map enumerating exactly the same three names (in the same
/// alphabetical order TS uses for HashMap iteration).
fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, midnight_storage::DefaultDB> =
        HashMap::new();
    operations = operations.insert(
        EntryPointBuf(b"set_tiny".to_vec()),
        ContractOperation::new(None),
    );
    operations = operations.insert(
        EntryPointBuf(b"set_medium".to_vec()),
        ContractOperation::new(None),
    );
    operations = operations.insert(
        EntryPointBuf(b"set_wide".to_vec()),
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
fn bug11_fixture_init_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");
    let envelope = make_envelope(result.current_contract_state.clone());
    assert_step_bytes_eq("init", &envelope, &ts_ref.after_init);
}

#[test]
fn bug11_fixture_set_tiny_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let circ_ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);
    let after_tiny = contract.set_tiny(circ_ctx, 42u8).expect("set_tiny");
    let envelope = make_envelope(after_tiny.context.current_query_context.state.clone());
    assert_step_bytes_eq("set_tiny", &envelope, &ts_ref.after_set_tiny);
}

#[test]
fn bug11_fixture_set_medium_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let circ_ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);
    let after_tiny = contract.set_tiny(circ_ctx, 42u8).expect("set_tiny");
    let after_medium = contract
        .set_medium(after_tiny.context, 65_000u32)
        .expect("set_medium");
    let envelope = make_envelope(after_medium.context.current_query_context.state.clone());
    assert_step_bytes_eq("set_medium", &envelope, &ts_ref.after_set_medium);
}

#[test]
fn bug11_fixture_set_wide_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let circ_ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);
    let after_tiny = contract.set_tiny(circ_ctx, 42u8).expect("set_tiny");
    let after_medium = contract
        .set_medium(after_tiny.context, 65_000u32)
        .expect("set_medium");
    let after_wide = contract
        .set_wide(after_medium.context, 4_500_000_000u64)
        .expect("set_wide");
    let envelope = make_envelope(after_wide.context.current_query_context.state.clone());
    assert_step_bytes_eq("set_wide", &envelope, &ts_ref.after_set_wide);
}
