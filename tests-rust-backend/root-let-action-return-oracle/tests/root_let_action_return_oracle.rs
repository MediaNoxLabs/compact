// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_root_let_action_return_oracle_fixture::ledger_contract::{
    Contract, PublicStateView, initial_state, step,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::Field;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/root-let-action-return-oracle.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"step".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn root_let_action_runs_once_and_returns_independent_parameter() {
    let expected = oracle();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        expected["initialStateHex"]
    );
    let mut context = initial.into_circuit_context(ContractAddress::default());
    for (name, echo, stored) in [("first", 9_u64, 1_u64), ("second", 13, 2)] {
        let reference = &expected[name];
        let result = step(context, Field::from(echo)).unwrap();
        assert_eq!(result.result, Field::from(echo));
        assert_eq!(reference["echo"], echo.to_string());
        assert_eq!(reference["result"], echo.to_string());
        assert_eq!(result.private_transcript_outputs.len(), 0);
        assert_eq!(
            serde_json::to_value(&result.context.query.effects).unwrap(),
            reference["effects"],
            "{name}: ledger effects",
        );
        assert_eq!(
            state_hex(result.context.query.state.get_ref().clone()),
            reference["afterStateHex"],
            "{name}: serialized state",
        );
        assert_eq!(
            PublicStateView::from(&result).stored().unwrap(),
            Field::from(stored)
        );
        let queries = reference["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 2, "{name}: one read and one write");
        assert_eq!(
            queries[0]["opTags"],
            serde_json::json!(["dup", "idx", "popeq"])
        );
        assert_eq!(
            queries[1]["opTags"],
            serde_json::json!(["push", "push", "ins"])
        );
        let native_gas = serde_json::to_value(result.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            // The TypeScript reportedGas for this existing root-Let shape
            // includes only its final write query. Use the full ordered query
            // meter to compare the actual execution cost.
            let ts_gas: u64 = queries
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(
                native_gas[dimension].as_u64().unwrap(),
                ts_gas,
                "{name}: {dimension}"
            );
        }
        context = result.context;
    }
}

#[test]
fn root_let_recording_matches_typescript_native_and_replay_across_two_calls() {
    let expected = oracle();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let mut native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let mut recorded_context = initial.into_circuit_context(ContractAddress::default());
    for (name, echo) in [("first", 9_u64), ("second", 13)] {
        let reference = &expected[name];
        let native = step(native_context, Field::from(echo)).unwrap();
        let recorded = Contract::default()
            .recording
            .step(recorded_context, Field::from(echo))
            .unwrap();
        assert_eq!(native.result, Field::from(echo), "{name}: native return");
        assert_eq!(
            recorded.execution.result, native.result,
            "{name}: recorded return"
        );
        assert_eq!(
            native.gas_cost, recorded.execution.gas_cost,
            "{name}: full gas"
        );
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{name}: ledger effects"
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{name}: ledger state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            reference["afterStateHex"],
            "{name}: serialized state"
        );
        assert_eq!(
            serde_json::to_value(&recorded.execution.context.query.effects).unwrap(),
            reference["effects"],
            "{name}: TS ledger effects"
        );
        assert_eq!(native.private_transcript_outputs.len(), 0);
        assert_eq!(recorded.execution.private_transcript_outputs.len(), 0);
        assert_eq!(reference["privateTranscriptCount"], 0);
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            reference["publicTranscript"],
            "{name}: exact recorded program"
        );
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(
            replay.context.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{name}: replay state"
        );
        assert_eq!(
            replay.context.effects, native.context.query.effects,
            "{name}: replay effects"
        );
        let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let ts_replay: u64 = reference["replayGas"][dimension]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                replay_gas[dimension], ts_replay,
                "{name}: TS replay gas {dimension}"
            );
        }
        let gas = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let ts_total: u64 = reference["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(gas[dimension], ts_total, "{name}: TS query gas {dimension}");
        }
        native_context = native.context;
        recorded_context = recorded.execution.context;
    }
}
