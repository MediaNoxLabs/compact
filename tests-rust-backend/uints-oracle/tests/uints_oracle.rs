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

use compact_rust_uints_oracle_fixture::ledger_contract::{initial_state, recorded, set_byte};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::ops::Op;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    operations = operations.insert(
        EntryPointBuf(b"set_byte".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_uints_oracle_matches_typescript_state_and_width() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/uints-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let set255 = set_byte(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::BoundedUint::<255>::new(255).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(set255.context.query.state.get_ref().clone()),
        oracle["afterSet255"]
    );
    let value = runtime::ledger::read_root_cell::<runtime::BoundedUint<255>, _>(
        set255.context.query.state.get_ref(),
        0,
    )
    .unwrap();
    assert_eq!(value.value().to_string(), oracle["byteAfterSet255"]);
    assert!(runtime::BoundedUint::<255>::new(256).is_err());
    let set0 = set_byte(set255.context, runtime::BoundedUint::<255>::new(0).unwrap()).unwrap();
    assert_eq!(
        state_hex(set0.context.query.state.get_ref().clone()),
        oracle["afterSet0"]
    );
}

#[test]
fn recorded_uint_cell_write_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/uints-oracle.json"
    ))
    .unwrap();
    let mut native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let mut recorded_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());

    for (value, state_key, gas_key, transcript_key, private_key) in [
        (
            255,
            "afterSet255",
            "set255Gas",
            "set255Transcript",
            "set255PrivateTranscriptCount",
        ),
        (
            0,
            "afterSet0",
            "set0Gas",
            "set0Transcript",
            "set0PrivateTranscriptCount",
        ),
    ] {
        let value = runtime::BoundedUint::<255>::new(value).unwrap();
        let native = set_byte(native_context, value).unwrap();
        let call = recorded::set_byte(recorded_context, value).unwrap();
        let replay = call
            .public
            .initial()
            .query(
                call.public.verify_ops(),
                None,
                &call.execution.context.cost_model,
            )
            .unwrap();

        assert!(matches!(
            call.public.verify_ops(),
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
            oracle[transcript_key],
            serde_json::json!([
                { "kind": "push", "storage": false },
                { "kind": "push", "storage": true },
                { "kind": "ins", "cached": false, "n": 1 },
            ])
        );
        assert_eq!(oracle[private_key], 0);
        assert!(call.execution.private_transcript_outputs.is_empty());
        let gas = &oracle[gas_key];
        assert_eq!(
            call.execution
                .gas_cost
                .read_time
                .into_picoseconds()
                .to_string(),
            gas["readTime"]
        );
        assert_eq!(
            call.execution
                .gas_cost
                .compute_time
                .into_picoseconds()
                .to_string(),
            gas["computeTime"]
        );
        assert_eq!(
            call.execution.gas_cost.bytes_written.to_string(),
            gas["bytesWritten"]
        );
        assert_eq!(
            call.execution.gas_cost.bytes_deleted.to_string(),
            gas["bytesDeleted"]
        );
        assert_eq!(native.gas_cost, call.execution.gas_cost);
        assert_eq!(call.execution.gas_cost, replay.gas_cost);
        assert_eq!(
            native.context.query.effects,
            call.execution.context.query.effects
        );
        assert_eq!(native.context.query.effects, replay.context.effects);
        for state in [
            native.context.query.state.get_ref(),
            call.execution.context.query.state.get_ref(),
            replay.context.state.get_ref(),
        ] {
            assert_eq!(state_hex(state.clone()), oracle[state_key]);
        }
        native_context = native.context;
        recorded_context = call.execution.context;
    }
}
