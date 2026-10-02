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

use compact_rust_inline_type_scope_oracle_fixture::ledger_contract::{
    checkAggScope, checkNoCollisionScope, checkScalarScope, initial_state, recorded, setHash,
};
use midnight_compact_runtime::context::{CircuitContext, CircuitResult, ConstructorContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{CompactError, Field, FixedBytes, FixedVector, persistent_hash};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "setHash",
        "checkScalarScope",
        "checkAggScope",
        "checkNoCollisionScope",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn vector2(a: u64, b: u64) -> FixedVector<Field, 2> {
    FixedVector::new([Field::from(a), Field::from(b)])
}

fn execute_scenario<F>(hash: FixedBytes<32>, check: F) -> String
where
    F: FnOnce(CircuitContext<()>) -> Result<CircuitResult<(), ()>, CompactError>,
{
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let seeded = setHash(context, hash).unwrap();
    let checked = check(seeded.context).unwrap();
    state_hex(checked.context.query.state.get_ref().clone())
}

#[test]
fn internal_helper_formal_scopes_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/inline-type-scope-oracle.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );

    let scalar_hash = persistent_hash(Field::from(5_u64));
    assert_eq!(
        execute_scenario(scalar_hash, |context| checkScalarScope(
            context,
            vector2(9, 10),
            Field::from(5_u64),
        )),
        oracle["scalarStateHex"]
    );
    assert_eq!(
        execute_scenario(
            persistent_hash(FixedVector::new([
                Field::from(1_u64),
                Field::from(2_u64),
                Field::from(3_u64),
                Field::from(4_u64),
            ])),
            |context| checkAggScope(
                context,
                vector2(7, 8),
                FixedVector::new([
                    Field::from(1_u64),
                    Field::from(2_u64),
                    Field::from(3_u64),
                    Field::from(4_u64),
                ]),
            ),
        ),
        oracle["aggStateHex"]
    );
    assert_eq!(
        execute_scenario(persistent_hash(vector2(3, 4)), |context| {
            checkNoCollisionScope(context, vector2(3, 4))
        }),
        oracle["noCollisionStateHex"]
    );

    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let seeded = setHash(context, scalar_hash).unwrap();
    let error = checkScalarScope(seeded.context, vector2(9, 10), Field::from(6_u64))
        .err()
        .unwrap();
    assert_eq!(error.to_string(), oracle["scalarMismatch"]);
}

#[test]
fn bytes_cell_recording_matches_native_and_replays() {
    let hash = persistent_hash(Field::from(5_u64));
    let native = setHash(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        hash,
    )
    .unwrap();
    let recorded = recorded::setHash(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        hash,
    )
    .unwrap();
    assert_eq!(recorded.public.verify_ops().len(), 3);
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        state_hex(native.context.query.state.get_ref().clone())
    );
    let replay = recorded
        .public
        .initial()
        .query(recorded.public.verify_ops(), None, &INITIAL_COST_MODEL)
        .unwrap();
    assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        state_hex(native.context.query.state.get_ref().clone())
    );
}
