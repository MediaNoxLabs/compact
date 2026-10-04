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

use compact_rust_bug11_oracle_fixture::ledger_contract::{
    initial_state, recorded, set_medium, set_tiny, set_wide,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::recording::RecordedCircuitResult;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["set_tiny", "set_medium", "set_wide"] {
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

fn normalized_verify_ops(recorded: &RecordedCircuitResult<(), ()>) -> serde_json::Value {
    fn normalize(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("alignment")
                    && let Some(serde_json::Value::Array(chunks)) = object.get_mut("value")
                {
                    for chunk in chunks {
                        if let serde_json::Value::Array(bytes) = chunk {
                            let bytes = bytes
                                .iter()
                                .map(|byte| byte.as_u64().unwrap() as u8)
                                .collect::<Vec<_>>();
                            *chunk = serde_json::json!({ "bytesHex": hex::encode(bytes) });
                        }
                    }
                }
                for child in object.values_mut() {
                    normalize(child);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    normalize(child);
                }
            }
            _ => {}
        }
    }
    let mut ops = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    normalize(&mut ops);
    ops
}

fn assert_single_query_capture(
    recorded: &RecordedCircuitResult<(), ()>,
    expected: &serde_json::Value,
) {
    let queries = expected["queries"].as_array().unwrap();
    assert_eq!(queries.len(), 1);
    assert_eq!(expected["reportedGas"], queries[0]["gasCost"]);
    let gas = &recorded.execution.gas_cost;
    assert_eq!(
        gas.read_time.into_picoseconds().to_string(),
        expected["reportedGas"]["readTime"]
    );
    assert_eq!(
        gas.compute_time.into_picoseconds().to_string(),
        expected["reportedGas"]["computeTime"]
    );
    assert_eq!(
        gas.bytes_written.to_string(),
        expected["reportedGas"]["bytesWritten"]
    );
    assert_eq!(
        gas.bytes_deleted.to_string(),
        expected["reportedGas"]["bytesDeleted"]
    );
    let ops = normalized_verify_ops(recorded);
    assert_eq!(ops, expected["publicTranscript"]);
    let tags = ops
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op.as_object().unwrap().keys().next().unwrap().as_str())
        .collect::<Vec<_>>();
    assert_eq!(serde_json::json!(tags), queries[0]["opTags"]);
    assert_eq!(expected["privateOutputCount"], 0);
    assert!(recorded.execution.private_transcript_outputs.is_empty());
}

#[test]
fn exact_bug11_oracle_matches_non_power_of_two_uint_cell_widths() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/bug11-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let tiny = set_tiny(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::BoundedUint::<99>::new(99).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(tiny.context.query.state.get_ref().clone()),
        oracle["afterTiny99"]
    );
    let medium = set_medium(
        tiny.context,
        runtime::BoundedUint::<69999>::new(69999).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(medium.context.query.state.get_ref().clone()),
        oracle["afterMedium69999"]
    );
    let wide = set_wide(
        medium.context,
        runtime::BoundedUint::<4999999999>::new(4999999999).unwrap(),
    )
    .unwrap();
    let state = wide.context.query.state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterWide4999999999"]);
    let tiny = runtime::ledger::read_root_cell::<runtime::BoundedUint<99>, _>(state, 0).unwrap();
    let medium =
        runtime::ledger::read_root_cell::<runtime::BoundedUint<69999>, _>(state, 1).unwrap();
    let wide =
        runtime::ledger::read_root_cell::<runtime::BoundedUint<4999999999>, _>(state, 2).unwrap();
    assert_eq!(tiny.value().to_string(), oracle["values"]["tiny"]);
    assert_eq!(medium.value().to_string(), oracle["values"]["medium"]);
    assert_eq!(wide.value().to_string(), oracle["values"]["wide"]);
    assert!(runtime::BoundedUint::<99>::new(100).is_err());
    assert!(runtime::BoundedUint::<69999>::new(70000).is_err());
    assert!(runtime::BoundedUint::<4999999999>::new(5000000000).is_err());
}

#[test]
fn non_power_of_two_uint_writes_record_typescript_parity_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/bug11-oracle.json"
    ))
    .unwrap();
    let initial = || {
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    let mut native_context = initial();
    let mut recorded_context = initial();
    macro_rules! check_write {
        ($name:ident, $value:expr, $state:literal) => {{
            let native = $name(native_context, $value).unwrap();
            let recorded = recorded::$name(recorded_context, $value).unwrap();
            let replay = recorded
                .public
                .initial()
                .query(
                    recorded.public.verify_ops(),
                    None,
                    &recorded.execution.context.cost_model,
                )
                .unwrap();
            assert_single_query_capture(&recorded, &oracle["calls"][stringify!($name)]);
            assert_eq!(native.gas_cost, recorded.execution.gas_cost);
            assert_eq!(recorded.execution.gas_cost, replay.gas_cost);
            assert_eq!(
                native.context.query.effects,
                recorded.execution.context.query.effects
            );
            assert_eq!(native.context.query.effects, replay.context.effects);
            for state in [
                native.context.query.state.get_ref(),
                recorded.execution.context.query.state.get_ref(),
                replay.context.state.get_ref(),
            ] {
                assert_eq!(state_hex(state.clone()), oracle[$state]);
            }
            native_context = native.context;
            recorded_context = recorded.execution.context;
        }};
    }
    check_write!(
        set_tiny,
        runtime::BoundedUint::<99>::new(99).unwrap(),
        "afterTiny99"
    );
    check_write!(
        set_medium,
        runtime::BoundedUint::<69999>::new(69999).unwrap(),
        "afterMedium69999"
    );
    check_write!(
        set_wide,
        runtime::BoundedUint::<4999999999>::new(4999999999).unwrap(),
        "afterWide4999999999"
    );
    assert_eq!(
        native_context.query.state.get_ref(),
        recorded_context.query.state.get_ref()
    );
}
