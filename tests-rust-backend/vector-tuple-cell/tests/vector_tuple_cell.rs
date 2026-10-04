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

use compact_rust_vector_tuple_cell_fixture::ledger_contract::{
    initial_state, read_pair, read_values, recorded, set_pair, set_values,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::recording::RecordedCircuitResult;
use midnight_compact_runtime::{Field, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["set_values", "read_values", "set_pair", "read_pair"] {
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

fn assert_replay<Output>(call: &RecordedCircuitResult<(), Output>) {
    let replay = call
        .public
        .initial()
        .query(
            call.public.verify_ops(),
            None,
            &call.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.state.get_ref(),
        call.execution.context.query.state.get_ref()
    );
    assert_eq!(replay.context.effects, call.execution.context.query.effects);
    assert_eq!(replay.gas_cost, call.execution.gas_cost);
}

#[test]
fn vector_and_tuple_cells_match_typescript_state_and_reads() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/vector-tuple-cell.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let context = constructor.into_circuit_context(ContractAddress::default());
    let before_vector = read_values(context).unwrap();
    assert_eq!(
        before_vector.result,
        FixedVector::new([Field::from(0_u64); 3])
    );
    let before_pair = read_pair(before_vector.context).unwrap();
    assert_eq!(before_pair.result, (Field::from(0_u64), false));

    let vector = FixedVector::new([Field::from(3_u64), Field::from(5_u64), Field::from(8_u64)]);
    let write = set_values(before_pair.context, vector.clone()).unwrap();
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["afterVectorHex"]
    );
    let after_vector = read_values(write.context).unwrap();
    assert_eq!(after_vector.result, vector);

    let write = set_pair(after_vector.context, (Field::from(42_u64), true)).unwrap();
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["afterPairHex"]
    );
    let after_pair = read_pair(write.context).unwrap();
    assert_eq!(after_pair.result, (Field::from(42_u64), true));
}

#[test]
fn recorded_vector_and_tuple_cells_match_native_typescript_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/vector-tuple-cell.json"
    ))
    .unwrap();
    let native = initial_state(ConstructorContext::new(())).unwrap();
    let recording = initial_state(ConstructorContext::new(())).unwrap();
    let native = native.into_circuit_context(ContractAddress::default());
    let recording = recording.into_circuit_context(ContractAddress::default());

    let native_read = read_values(native).unwrap();
    let recorded_read = recorded::read_values(recording).unwrap();
    assert_replay(&recorded_read);
    assert_eq!(recorded_read.execution.result, native_read.result);
    assert_eq!(recorded_read.execution.gas_cost, native_read.gas_cost);

    let vector = FixedVector::new([Field::from(3_u64), Field::from(5_u64), Field::from(8_u64)]);
    let native_write = set_values(native_read.context, vector.clone()).unwrap();
    let recorded_write =
        recorded::set_values(recorded_read.execution.context, vector.clone()).unwrap();
    assert_replay(&recorded_write);
    assert_eq!(recorded_write.execution.gas_cost, native_write.gas_cost);
    assert_eq!(
        state_hex(
            recorded_write
                .execution
                .context
                .query
                .state
                .get_ref()
                .clone()
        ),
        oracle["afterVectorHex"]
    );

    let native_read = read_values(native_write.context).unwrap();
    let recorded_read = recorded::read_values(recorded_write.execution.context).unwrap();
    assert_replay(&recorded_read);
    assert_eq!(recorded_read.execution.result, vector);
    assert_eq!(recorded_read.execution.result, native_read.result);
    assert_eq!(recorded_read.execution.gas_cost, native_read.gas_cost);

    let pair = (Field::from(42_u64), true);
    let native_write = set_pair(native_read.context, pair).unwrap();
    let recorded_write = recorded::set_pair(recorded_read.execution.context, pair).unwrap();
    assert_replay(&recorded_write);
    assert_eq!(recorded_write.execution.gas_cost, native_write.gas_cost);
    assert_eq!(
        state_hex(
            recorded_write
                .execution
                .context
                .query
                .state
                .get_ref()
                .clone()
        ),
        oracle["afterPairHex"]
    );

    let native_read = read_pair(native_write.context).unwrap();
    let recorded_read = recorded::read_pair(recorded_write.execution.context).unwrap();
    assert_replay(&recorded_read);
    assert_eq!(recorded_read.execution.result, pair);
    assert_eq!(recorded_read.execution.result, native_read.result);
    assert_eq!(recorded_read.execution.gas_cost, native_read.gas_cost);
}
