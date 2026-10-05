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

use compact_rust_qualified_coin_set_oracle_fixture::{ledger_contract, types};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{CoinInfo, CoinRecipient, ContractAddress, DefaultDB, StateValue};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/qualified-coin-set-oracle.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new()
        .insert(
            EntryPointBuf(b"insert_coin".to_vec()),
            ContractOperation::new(None),
        )
        .insert(
            EntryPointBuf(b"contains".to_vec()),
            ContractOperation::new(None),
        );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn coin() -> types::ShieldedCoinInfo {
    let mut nonce = [0; 32];
    nonce[..5].copy_from_slice(b"nonce");
    let mut color = [0; 32];
    color[..5].copy_from_slice(b"color");
    types::ShieldedCoinInfo {
        nonce: runtime::FixedBytes::new(nonce),
        color: runtime::FixedBytes::new(color),
        value: runtime::BoundedUint::new(42).unwrap(),
    }
}

fn recipient(left: bool) -> types::Either {
    let mut user_key = [0; 32];
    user_key[0] = 7;
    types::Either {
        is_left: left,
        left: types::ZswapCoinPublicKey {
            bytes: runtime::FixedBytes::new(if left { user_key } else { [0; 32] }),
        },
        right: types::ContractAddress {
            bytes: runtime::FixedBytes::new([0; 32]),
        },
    }
}

fn ledger_carriers(
    coin: &types::ShieldedCoinInfo,
    recipient: &types::Either,
) -> (CoinInfo, CoinRecipient) {
    (
        runtime::ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value()),
        runtime::ledger::coin_recipient_from_compact(
            recipient.is_left,
            recipient.left.bytes,
            recipient.right.bytes,
        ),
    )
}

fn gas_matches(
    actual: runtime::context::RunningCost,
    queries: &serde_json::Value,
    reported: &serde_json::Value,
) {
    let actual = serde_json::to_value(actual).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = queries
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
        assert_eq!(actual[dimension].as_u64().unwrap(), expected, "{dimension}");
        assert_eq!(
            actual[dimension].as_u64().unwrap(),
            reported[dimension]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        );
    }
}

#[test]
fn qualified_coin_insert_matches_typescript_with_allocated_indices_and_rejects_missing() {
    let expected = oracle();
    for (case, left, index) in [("right7", false, 7_u64), ("left11", true, 11_u64)] {
        let coin = coin();
        let recipient = recipient(left);
        let initial = ledger_contract::initial_state(ConstructorContext::new(())).unwrap();
        assert_eq!(
            state_hex(initial.ledger_state.get_ref().clone()),
            expected[case]["initialStateHex"]
        );
        let mut context = initial.into_circuit_context(ContractAddress::default());
        let (info, target) = ledger_carriers(&coin, &recipient);
        context.query.call_context.com_indices = context
            .query
            .call_context
            .com_indices
            .insert(info.commitment(&target), index);
        let inserted = ledger_contract::insert_coin(context, coin.clone(), recipient).unwrap();
        assert_eq!(
            serde_json::to_value(&inserted.context.query.effects).unwrap(),
            expected[case]["effects"]
        );
        let qualified = types::QualifiedShieldedCoinInfo {
            nonce: coin.nonce,
            color: coin.color,
            value: coin.value,
            mt_index: runtime::BoundedUint::new(index as u128).unwrap(),
        };
        assert_eq!(
            state_hex(inserted.context.query.state.get_ref().clone()),
            expected[case]["afterStateHex"]
        );
        assert_eq!(inserted.private_transcript_outputs.len(), 0);
        assert_eq!(expected[case]["privateTranscriptCount"], 0);
        gas_matches(
            inserted.gas_cost,
            &expected[case]["insertQueries"],
            &expected[case]["insertGas"],
        );
        assert_eq!(
            expected[case]["insertQueries"][0]["opTags"],
            serde_json::json!([
                "idx", "dup", "push", "idx", "push", "swap", "concat", "push", "ins", "ins"
            ]),
        );
        let member = ledger_contract::contains(inserted.context, qualified).unwrap();
        assert!(member.result);
        assert_eq!(expected[case]["member"], true);
        gas_matches(
            member.gas_cost,
            &expected[case]["memberQueries"],
            &expected[case]["memberGas"],
        );
    }
    let missing = ledger_contract::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let error = ledger_contract::insert_coin(missing, coin(), recipient(false))
        .err()
        .unwrap();
    assert!(error.to_string().contains("Coin commitment not found"));
    assert!(
        expected["missing"]["error"]
            .as_str()
            .unwrap()
            .contains("Coin commitment not found")
    );
    assert_eq!(expected["missing"]["queries"].as_array().unwrap().len(), 0);

    let initial = ledger_contract::initial_state(ConstructorContext::new(())).unwrap();
    let mut wrong_set = initial.into_circuit_context(ContractAddress::default());
    let (info, target) = ledger_carriers(&coin(), &recipient(false));
    wrong_set.query.call_context.com_indices = wrong_set
        .query
        .call_context
        .com_indices
        .insert(info.commitment(&target), 7);
    let error = runtime::slots::SetSlot::<bool>::new(&[0])
        .insert_coin(wrong_set, info, target)
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("alignment is not QualifiedShieldedCoinInfo")
    );
}
