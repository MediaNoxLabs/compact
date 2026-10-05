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

use compact_rust_effectful_return_oracle_fixture::ledger_contract::{
    LedgerView, PublicStateView, Witnesses, choose, initial_state,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::Field;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

struct Mark;

impl Witnesses<u64> for Mark {
    fn mark(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, Field) {
        (*context.private_state + 1, Field::from(77_u64))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"choose".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn conditional_returns_keep_ordered_effects_and_skip_the_other_branch() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/effectful-return-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(0_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["initialStateHex"]
    );
    let mut context = initial.into_circuit_context(ContractAddress::default());
    for (name, next, expected_result, expected_state, expected_unused, expected_private) in [
        ("chosen", 9_u64, 0_u64, 9_u64, 0_u64, 0_u64),
        ("unchosen", 8_u64, 109_u64, 10_u64, 77_u64, 1_u64),
    ] {
        let expected = &oracle[name];
        let result = choose(context, &Mark, Field::from(next)).unwrap();
        assert_eq!(result.result, Field::from(expected_result));
        assert_eq!(expected["result"], expected_result.to_string());
        assert_eq!(result.context.private_state, expected_private);
        assert_eq!(expected["privateState"], expected_private);
        assert_eq!(
            result.private_transcript_outputs.len(),
            expected_private as usize
        );
        assert_eq!(expected["privateTranscriptCount"], expected_private);
        assert_eq!(
            PublicStateView::from(&result).state().unwrap(),
            Field::from(expected_state)
        );
        assert_eq!(
            PublicStateView::from(&result).unused().unwrap(),
            Field::from(expected_unused)
        );
        assert_eq!(
            state_hex(result.context.query.state.get_ref().clone()),
            expected["afterStateHex"],
            "{name}: serialized state"
        );
        assert_eq!(
            serde_json::to_value(&result.context.query.effects).unwrap(),
            expected["effects"],
            "{name}: ledger effects"
        );
        let queries = expected["queries"].as_array().unwrap();
        assert_eq!(
            queries.len(),
            4,
            "{name}: prior read/write, condition read, chosen write"
        );
        let native_gas = serde_json::to_value(result.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
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
            assert_eq!(native_gas[dimension], ts_gas, "{name}: {dimension}");
        }
        context = result.context;
    }
}
