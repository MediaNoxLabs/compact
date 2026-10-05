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

use std::cell::Cell;

use compact_rust_test_center_bboard_fixture::ledger_contract::{
    LedgerView, PublicStateView, Witnesses, initial_state, post, take_down,
};
use compact_rust_test_center_bboard_fixture::pure_circuits;
use compact_rust_test_center_bboard_fixture::types::STATE;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::{FixedBytes, OpaqueString};

struct SecretWitness(Cell<usize>);

impl Witnesses<u64> for SecretWitness {
    fn local_secret_key(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, FixedBytes<32>) {
        self.0.set(self.0.get() + 1);
        (*context.private_state, FixedBytes::new([7; 32]))
    }
}

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/test-center-bboard.json"
    ))
    .unwrap()
}

fn contract_state(state: StateValue<DefaultDB>) -> ContractState<DefaultDB> {
    let operations = HashMap::new()
        .insert(
            EntryPointBuf(b"post".to_vec()),
            ContractOperation::new(None),
        )
        .insert(
            EntryPointBuf(b"take_down".to_vec()),
            ContractOperation::new(None),
        );
    ContractState::new(state, operations, ContractMaintenanceAuthority::default())
}

fn state_hex(state: &ContractState<DefaultDB>) -> String {
    let mut bytes = Vec::new();
    tagged_serialize(state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_gas(actual: runtime::context::RunningCost, expected: &serde_json::Value) {
    let actual = serde_json::to_value(actual).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = expected["queries"]
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
        assert_eq!(actual[dimension].as_u64().unwrap(), total, "{dimension}");
    }
}

fn query_tags(expected: &serde_json::Value) -> Vec<Vec<String>> {
    expected["queries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|query| {
            query["opTags"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tag| tag.as_str().unwrap().to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn original_bboard_retains_unicode_message_before_ordered_clear() {
    let expected = oracle();
    let witness = SecretWitness(Cell::new(0));
    let initial = initial_state(ConstructorContext::new(5_u64)).unwrap();
    let initial_contract = contract_state(initial.ledger_state.get_ref().clone());
    assert_eq!(state_hex(&initial_contract), expected["initialStateHex"]);
    assert_eq!(
        PublicStateView::from(&initial_contract).state().unwrap(),
        STATE::vacant
    );
    assert_eq!(
        PublicStateView::from(&initial_contract)
            .instance()
            .unwrap()
            .value(),
        1
    );

    let empty_context =
        CircuitContext::from_contract_state(5_u64, ContractAddress::default(), &initial_contract);
    let error = take_down(empty_context, &witness).err().unwrap();
    assert!(format!("{error}").contains("Attempted to take down post from an empty board"));
    assert_eq!(witness.0.get(), 0);
    assert_eq!(expected["emptyTakeDown"]["success"], false);
    assert_eq!(
        expected["emptyTakeDown"]["afterStateHex"],
        expected["initialStateHex"]
    );
    assert_eq!(
        query_tags(&expected["emptyTakeDown"]),
        [vec!["dup", "idx", "popeq"]]
    );

    // The original native oracle fixes operation order and pre-clear return.
    // recording.rs separately compares full recorded programs and replay.
    assert_eq!(
        query_tags(&expected["post"]),
        [
            vec!["dup", "idx", "popeq"],
            vec!["dup", "idx", "popeq"],
            vec!["push", "push", "ins"],
            vec!["push", "push", "ins"],
            vec!["push", "push", "ins"],
        ]
    );
    assert_eq!(
        query_tags(&expected["takeDown"]),
        [
            vec!["dup", "idx", "popeq"],
            vec!["dup", "idx", "popeq"],
            vec!["dup", "idx", "popeq"],
            vec!["dup", "idx", "popeq"],
            vec!["push", "push", "ins"],
            vec!["idx", "addi", "ins"],
            vec!["push", "push", "ins"],
        ]
    );

    let message = expected["message"].as_str().unwrap();
    let post_context = initial.into_circuit_context(ContractAddress::default());
    let posted = post(post_context, &witness, OpaqueString::from(message)).unwrap();
    let _: () = posted.result;
    let posted_contract = contract_state(posted.context.query.state.get_ref().clone());
    assert_eq!(
        state_hex(&posted_contract),
        expected["post"]["afterStateHex"]
    );
    assert_eq!(
        PublicStateView::from(&posted_contract).state().unwrap(),
        STATE::occupied
    );
    assert_eq!(
        PublicStateView::from(&posted_contract)
            .instance()
            .unwrap()
            .value(),
        1
    );
    let posted_message = PublicStateView::from(&posted_contract).message().unwrap();
    assert!(posted_message.is_some);
    assert_eq!(posted_message.value, OpaqueString::from(message));
    let mut instance_bytes = [0_u8; 32];
    instance_bytes[0] = 1;
    let poster =
        pure_circuits::public_key(FixedBytes::new([7; 32]), FixedBytes::new(instance_bytes))
            .unwrap();
    assert_eq!(hex::encode(poster.into_array()), expected["publicKeyHex"]);
    assert_eq!(
        PublicStateView::from(&posted_contract).poster().unwrap(),
        poster
    );
    assert_eq!(witness.0.get(), 1);
    assert_eq!(posted.private_transcript_outputs.len(), 1);
    assert_eq!(expected["post"]["privateTranscriptCount"], 1);
    assert_gas(posted.gas_cost, &expected["post"]);

    let take_context = CircuitContext::from_contract_state(
        posted.context.private_state,
        ContractAddress::default(),
        &posted_contract,
    );
    let taken = take_down(take_context, &witness).unwrap();
    assert_eq!(taken.result, OpaqueString::from(message));
    assert_eq!(taken.result.0, expected["takeDown"]["result"]);
    let cleared_contract = contract_state(taken.context.query.state.get_ref().clone());
    assert_eq!(
        state_hex(&cleared_contract),
        expected["takeDown"]["afterStateHex"]
    );
    assert_eq!(
        PublicStateView::from(&cleared_contract).state().unwrap(),
        STATE::vacant
    );
    assert_eq!(
        PublicStateView::from(&cleared_contract)
            .instance()
            .unwrap()
            .value(),
        2
    );
    assert!(
        !PublicStateView::from(&cleared_contract)
            .message()
            .unwrap()
            .is_some
    );
    assert_eq!(witness.0.get(), 2);
    assert_eq!(taken.private_transcript_outputs.len(), 1);
    assert_eq!(expected["takeDown"]["privateTranscriptCount"], 1);
    assert_gas(taken.gas_cost, &expected["takeDown"]);
}
