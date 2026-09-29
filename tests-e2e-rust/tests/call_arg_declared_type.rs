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

//
// call_arg_declared_type_fixture.compact executing gate (#91).
//
// A call argument whose type already is the callee's formal reaches the
// emitter without a `safe-cast`, and used to render with no expected type.
// Some of those renders fail `cargo build` (`[0, 1]` for a `[Fr; 2]`
// formal, #21), but `persistentCommit<Field>(0 as Field, o)` emitted
// `persistent_commit(&0, ..)`, which builds: Rust infers `i32` and commits
// to its 4 bytes. Only the committed VALUE shows that, so every circuit in
// the fixture writes its result to the ledger and this test compares the
// state bytes after each one against the TS target's capture
// (fixtures/call-arg-declared-type-fixture-ts-state.json).

use std::collections::BTreeMap;

use compact_contract_call_arg_declared_type_fixture::{Contract, Ledger, Witnesses};
use midnight_compact_runtime::*;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use midnight_storage::DefaultDB;
use serde::Deserialize;

#[derive(Deserialize)]
struct TsReference {
    #[serde(rename = "afterInit")]
    after_init: Step,
    circuits: BTreeMap<String, Step>,
}

#[derive(Deserialize)]
struct Step {
    #[serde(rename = "stateHex")]
    state_hex: String,
}

impl Step {
    fn state_bytes(&self) -> Vec<u8> {
        hex::decode(&self.state_hex).expect("decode fixture hex")
    }
}

fn fixture() -> TsReference {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/call-arg-declared-type-fixture-ts-state.json"
    ))
    .expect("read call-arg-declared-type fixture");
    serde_json::from_str(&raw).expect("parse call-arg-declared-type fixture")
}

/// Mirrors `witnesses` in fixtures/capture-call-arg-declared-type-fixture.mjs:
/// `sumWitness` returns `v[0] + v[1]`, so the argument's value is written to
/// the ledger by `witnessConst`.
struct FixtureWitnesses;

impl Witnesses<()> for FixtureWitnesses {
    fn sum_witness<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>, v: [Fr; 2]) -> ((), Fr) {
        ((), v[0] + v[1])
    }
}

type Fixture = Contract<(), FixtureWitnesses>;
type Run = fn(&Fixture, CircuitContext<()>) -> ChargedState<DefaultDB>;

/// `(Compact name, method)` of an exported circuit, run for its resulting
/// ledger state.
macro_rules! circuit {
    ($name:literal, $method:ident) => {
        ($name, |c: &Fixture, ctx| {
            c.$method(ctx)
                .expect($name)
                .context
                .current_query_context
                .state
        })
    };
}

/// Every exported circuit, by its Compact name (the operations-map key and
/// the capture's key). MUST match `CIRCUITS` in
/// fixtures/capture-call-arg-declared-type-fixture.mjs.
const CIRCUITS: &[(&str, Run)] = &[
    circuit!("commitSmall", commit_small),
    circuit!("commitU128", commit_u128),
    circuit!("commitFieldOnly", commit_field_only),
    circuit!("pureBodyVec", pure_body_vec),
    circuit!("pureBodyFieldOnly", pure_body_field_only),
    circuit!("bridgeTupleIntoVec", bridge_tuple_into_vec),
    circuit!("bridgeVecIntoTuple", bridge_vec_into_tuple),
    circuit!("witnessConst", witness_const),
    circuit!("witnessBare", witness_bare),
    circuit!("pureFromImpure", pure_from_impure),
    circuit!("impureConst", impure_const),
    circuit!("impureBare", impure_bare),
    circuit!("impureInIfArm", impure_in_if_arm),
    circuit!("inlinedAssert", inlined_assert),
    circuit!("hashPersistentVec", hash_persistent_vec),
    circuit!("hashTransientVec", hash_transient_vec),
];

fn contract() -> Fixture {
    Contract::new(FixtureWitnesses)
}

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

/// The TS `initialState()` envelope: one operation per exported circuit.
fn state_bytes(data: ChargedState<DefaultDB>) -> Vec<u8> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for (name, _) in CIRCUITS {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let envelope = ContractState {
        data,
        operations,
        maintenance_authority: ContractMaintenanceAuthority::default(),
        balance: Default::default(),
    };
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");
    buf
}

fn assert_state_parity(rust: ChargedState<DefaultDB>, ts: &Step, label: &str) {
    let rust_bytes = state_bytes(rust);
    let ts_bytes = ts.state_bytes();
    assert_eq!(
        rust_bytes,
        ts_bytes,
        "{label}: Rust state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        rust_bytes.len(),
        hex::encode(&rust_bytes),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

/// The capture and `CIRCUITS` name the same circuits, so no circuit goes
/// unchecked on either side.
#[test]
fn capture_covers_every_circuit() {
    let ts = fixture();
    let captured: Vec<&str> = ts.circuits.keys().map(String::as_str).collect();
    let mut listed: Vec<&str> = CIRCUITS.iter().map(|(name, _)| *name).collect();
    listed.sort_unstable();
    assert_eq!(captured, listed);
}

#[test]
fn call_arg_declared_type_init_byte_parity() {
    let ts = fixture();
    let init = contract().initial_state(ctor_ctx()).expect("initial_state");
    assert_state_parity(init.current_contract_state, &ts.after_init, "constructor");
}

/// Each circuit runs from a fresh post-init state; its ledger writes must
/// equal the TS target's byte-for-byte.
#[test]
fn call_arg_declared_type_circuit_byte_parity() {
    let ts = fixture();
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    for (name, run) in CIRCUITS {
        let state = run(
            &contract,
            CircuitContext::new(init.current_contract_state.clone(), ()),
        );
        assert_state_parity(state, &ts.circuits[*name], name);
    }
}
