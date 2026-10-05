// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_adt_set_qualified_coin_info_fixture::ledger_contract;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/adt-set-qualified-coin-info.json"
    ))
    .unwrap()
}
fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new()
        .insert(
            EntryPointBuf(b"test_QualifiedShieldedCoinInfo".to_vec()),
            ContractOperation::new(None),
        )
        .insert(
            EntryPointBuf(b"test_ShieldedCoinInfo".to_vec()),
            ContractOperation::new(None),
        );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}
fn context(mock_indices: bool) -> runtime::context::CircuitContext<()> {
    let mut context = ledger_contract::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    if mock_indices {
        let mut nonce = [0; 32];
        nonce[..5].copy_from_slice(b"nonce");
        let mut color = [0; 32];
        color[..5].copy_from_slice(b"color");
        let recipient = runtime::ledger::coin_recipient_from_compact(
            false,
            runtime::FixedBytes::new([0; 32]),
            runtime::FixedBytes::new([0; 32]),
        );
        for value in [1, 2] {
            let coin = runtime::ledger::coin_info_from_compact(
                runtime::FixedBytes::new(nonce),
                runtime::FixedBytes::new(color),
                value,
            );
            context.query.call_context.com_indices = context
                .query
                .call_context
                .com_indices
                .insert(coin.commitment(&recipient), 0);
        }
    }
    context
}
fn assert_gas(actual: runtime::context::RunningCost, expected: &serde_json::Value) {
    let actual = serde_json::to_value(actual).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = expected
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
        assert_eq!(actual[dimension].as_u64().unwrap(), sum, "{dimension}");
    }
}
#[test]
fn qualified_coin_set_lifecycle_matches_original_typescript_and_replays() {
    let expected = oracle();
    let row = &expected[0];
    let initial = ledger_contract::initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        row["initialStateHex"]
    );
    let native = ledger_contract::test_QualifiedShieldedCoinInfo(context(false)).unwrap();
    let recorded =
        ledger_contract::recorded::test_QualifiedShieldedCoinInfo(context(false)).unwrap();
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_gas(native.gas_cost, &row["queryLog"]);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        row["afterStateHex"]
    );
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 0);
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
        recorded.execution.context.query.state.get_ref()
    );
}
#[test]
fn insert_coin_requires_indices_and_matches_mock_source_context() {
    let expected = oracle();
    let error = ledger_contract::recorded::test_ShieldedCoinInfo(context(false))
        .err()
        .unwrap();
    assert!(error.to_string().contains("Coin commitment not found"));
    assert!(
        expected[1]["error"]
            .as_str()
            .unwrap()
            .contains("Coin commitment not found")
    );
    // The original source requires index zero for both distinct commitments.
    // This mocks that call context solely to check source-level parity.
    let native = ledger_contract::test_ShieldedCoinInfo(context(true)).unwrap();
    let recorded = ledger_contract::recorded::test_ShieldedCoinInfo(context(true)).unwrap();
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_gas(native.gas_cost, &expected[2]["queryLog"]);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        expected[2]["afterStateHex"]
    );
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 0);
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
        recorded.execution.context.query.state.get_ref()
    );
}
