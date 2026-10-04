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

use compact_rust_counter_parameter_fixture::ledger_contract::{
    decrement_by, increment_by, initial_state, recorded, reset_round,
};
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::DefaultDB;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_counter};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::ops::Op;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["decrement_by", "increment_by", "reset_round"] {
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

#[test]
fn generated_counter_uses_bounded_parameter() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let amount = BoundedUint::<65535>::new(7).unwrap();
    let result = increment_by(context, amount).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 7);

    let result = increment_by(result.context, BoundedUint::<65535>::new(2).unwrap()).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 9);

    let result = decrement_by(result.context, BoundedUint::<65535>::new(5).unwrap()).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 4);
    let result = reset_round(result.context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 0);
    assert!(decrement_by(result.context, BoundedUint::<65535>::new(1).unwrap()).is_err());
}

#[test]
fn recorded_counter_parameter_replays_the_generated_amount() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let recorded = recorded::increment_by(context, BoundedUint::<65535>::new(7).unwrap()).unwrap();
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(recorded.public.verify_ops().len(), 3);
    for state in [
        recorded.execution.context.query.state.get_ref(),
        replay.context.state.get_ref(),
    ] {
        let StateValue::Array(fields) = state else {
            panic!("expected ledger field array")
        };
        assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 7);
    }
}

#[test]
fn recorded_reset_matches_native_and_replays_from_a_nonzero_counter() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/counter-parameter-reset-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["before"]
    );
    let native = increment_by(
        initial.into_circuit_context(ContractAddress::default()),
        BoundedUint::<65535>::new(7).unwrap(),
    )
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let recorded = increment_by(
        initial.into_circuit_context(ContractAddress::default()),
        BoundedUint::<65535>::new(7).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle["afterIncrement"]
    );
    let native = reset_round(native.context).unwrap();
    let recorded = recorded::reset_round(recorded.context).unwrap();
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();

    assert_eq!(recorded.public.verify_ops().len(), 3);
    assert!(matches!(
        recorded.public.verify_ops(),
        [
            Op::Push { storage: false, .. },
            Op::Push { storage: true, .. },
            Op::Ins {
                cached: false,
                n: 1
            },
        ]
    ));
    assert_eq!(
        oracle["resetTranscript"],
        serde_json::json!([
            { "kind": "push", "storage": false },
            { "kind": "push", "storage": true },
            { "kind": "ins", "cached": false, "n": 1 },
        ])
    );
    let reported_gas = &oracle["resetGas"];
    assert_eq!(
        recorded
            .execution
            .gas_cost
            .read_time
            .into_picoseconds()
            .to_string(),
        reported_gas["readTime"]
    );
    assert_eq!(
        recorded
            .execution
            .gas_cost
            .compute_time
            .into_picoseconds()
            .to_string(),
        reported_gas["computeTime"]
    );
    assert_eq!(
        recorded.execution.gas_cost.bytes_written.to_string(),
        reported_gas["bytesWritten"]
    );
    assert_eq!(
        recorded.execution.gas_cost.bytes_deleted.to_string(),
        reported_gas["bytesDeleted"]
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(recorded.execution.gas_cost, replay.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(native.context.query.effects, replay.context.effects);
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    assert_eq!(
        native.context.query.state.get_ref(),
        replay.context.state.get_ref()
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle["afterReset"]
    );
    assert_eq!(oracle["before"], oracle["afterReset"]);
    let StateValue::Array(fields) = replay.context.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 0);
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(oracle["resetPrivateTranscriptCount"], 0);
}
