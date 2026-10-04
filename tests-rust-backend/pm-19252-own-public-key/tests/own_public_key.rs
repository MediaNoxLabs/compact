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

use compact_rust_pm_19252_own_public_key_fixture::ledger_contract::{
    Contract, initial_state, test1,
};
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{ConstructorContext, RunningCost};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let contract = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn native_public_key_matches_typescript_without_user_witness_or_proof() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/pm-19252-own-public-key.json"
    ))
    .unwrap();
    assert_eq!(oracle["proofRequired"], false);
    assert_eq!(oracle["stateBefore"], oracle["stateAfter"]);
    assert_eq!(oracle["publicTranscript"], serde_json::json!([]));
    let key: [u8; 32] = oracle["coinPublicKey"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_u64().unwrap() as u8)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let before = state_hex(context.query.state.get_ref().clone());
    assert_eq!(before, oracle["stateBefore"]);
    let output = Contract::default()
        .test1(context.with_coin_public_key_bytes(key))
        .unwrap();
    assert_eq!(output.result, ());
    assert_eq!(oracle["result"], serde_json::json!([]));
    assert_eq!(output.gas_cost, RunningCost::ZERO);
    assert_eq!(
        serde_json::to_value(output.gas_cost.read_time).unwrap(),
        serde_json::json!(
            oracle["gasCost"]["readTime"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        )
    );
    assert_eq!(
        serde_json::to_value(output.gas_cost.compute_time).unwrap(),
        serde_json::json!(
            oracle["gasCost"]["computeTime"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        )
    );
    assert_eq!(
        output.gas_cost.bytes_written.to_string(),
        oracle["gasCost"]["bytesWritten"]
    );
    assert_eq!(
        output.gas_cost.bytes_deleted.to_string(),
        oracle["gasCost"]["bytesDeleted"]
    );
    assert_eq!(
        state_hex(output.context.query.state.get_ref().clone()),
        oracle["stateAfter"]
    );
    assert_eq!(output.context.private_state, ());
    assert_eq!(output.private_transcript_outputs.len(), 1);
    let transcript = &output.private_transcript_outputs[0];
    let atoms = transcript
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        oracle["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&transcript.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
}

#[test]
fn missing_execution_coin_key_is_an_explicit_error() {
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    assert!(matches!(
        test1(context),
        Err(CompactError::MissingCoinPublicKey)
    ));
}
