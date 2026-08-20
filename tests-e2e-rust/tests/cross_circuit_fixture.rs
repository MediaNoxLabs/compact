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
// cross_circuit_fixture.compact byte-parity test (E5 of the M3.5 plan).
//
// Drives the generated cross-circuit-fixture crate through initial_state()
// and asserts the serialized ContractState matches the TS reference fixture
// captured by fixtures/capture-cross-circuit-fixture.mjs.
//
// The fixture exists to exercise the body walker's `impure-exported`
// classification: `reset_and_set` is an exported impure circuit whose
// body invokes `reset()` (also exported impure) via the
// `self.reset(ctx, ...)?` method-call shape with context threading. The
// byte-parity assertion below only checks the initial_state seed; the
// `reset_and_set` body is emitted (and compiled) but not invoked here.

use compact_contract_cross_circuit_fixture::Contract;
use compact_runtime::*;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use tests_e2e_rust::SmallFixtureTsReference;

fn fixture() -> SmallFixtureTsReference {
    SmallFixtureTsReference::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/cross-circuit-fixture-ts-state.json"
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

/// `cross_circuit_fixture` exports two impure circuits: `reset_and_set`
/// then `reset` (insertion order matches the TS-side fixture).
fn make_envelope(
    data: ChargedState<midnight_storage::DefaultDB>,
) -> ContractState<midnight_storage::DefaultDB> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, midnight_storage::DefaultDB> =
        HashMap::new();
    operations = operations.insert(
        EntryPointBuf(b"reset_and_set".to_vec()),
        ContractOperation::new(None, None),
    );
    operations = operations.insert(
        EntryPointBuf(b"reset".to_vec()),
        ContractOperation::new(None, None),
    );
    ContractState {
        data,
        operations,
        maintenance_authority: ContractMaintenanceAuthority::default(),
        balance: Default::default(),
    }
}

#[test]
fn cross_circuit_fixture_init_byte_parity() {
    let ts_ref = fixture();
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let result = contract.initial_state(ctor_ctx()).expect("initial_state");

    let envelope = make_envelope(result.current_contract_state.clone());
    let mut buf = Vec::new();
    tagged_serialize(&envelope, &mut buf).expect("tagged_serialize");

    let ts_bytes = ts_ref.after_init.state_bytes();
    assert_eq!(
        buf,
        ts_bytes,
        "Rust state bytes differ from TS reference\n\nRust ({} B): {}\n\nTS   ({} B): {}",
        buf.len(),
        hex::encode(&buf),
        ts_bytes.len(),
        hex::encode(&ts_bytes),
    );
}

/// A27 gas-accounting regression: `reset_and_set(v)` calls `reset()` then
/// writes `n`, so its reported gas must include the callee `reset()`'s cost —
/// not just its own write. Before the fix, the non-streamed impure-call
/// lowering rebound `ctx` but dropped `_cr.gas_cost`, so the circuit returned
/// only the final write's gas and under-reported successful transactions.
///
/// `reset_and_set` performs everything `reset` does (via the `reset()` call)
/// PLUS an extra `n.write`, so its gas must strictly exceed `reset`'s. With the
/// bug the two were ~equal (the callee's query gas was discarded).
#[test]
fn reset_and_set_accumulates_callee_gas() {
    let contract: Contract<(), NoWitnesses> = Contract::new(NoWitnesses);
    let deployed = contract.initial_state(ctor_ctx()).expect("initial_state");
    let state = deployed.current_contract_state.clone();

    let make_cctx = || CircuitContext {
        current_query_context: QueryContext::new(state.clone(), ContractAddress::default()),
        current_private_state: (),
        current_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    };

    let reset_gas = contract.reset(make_cctx()).expect("reset").gas_cost;
    let ras_gas = contract
        .reset_and_set(make_cctx(), 7u64)
        .expect("reset_and_set")
        .gas_cost;

    assert!(
        ras_gas > reset_gas,
        "reset_and_set gas ({ras_gas:?}) must strictly exceed reset gas ({reset_gas:?}); \
         the callee reset()'s gas was dropped (A27 regression)"
    );
}
