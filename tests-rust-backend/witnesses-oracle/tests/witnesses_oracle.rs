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

use compact_rust_witnesses_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, pull,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

struct OracleWitness;

impl Witnesses<u64> for OracleWitness {
    fn fetch_field(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, runtime::Field) {
        assert_eq!(*context.private_state, 7);
        assert_eq!(*context.contract_address, ContractAddress::default());
        assert_eq!(context.ledger.v().unwrap(), runtime::Field::default());
        (8, runtime::Field::from(42_u64))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"pull".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_witness_oracle_matches_state_private_state_and_transcript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witnesses-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let after = pull(
        initial.into_circuit_context(ContractAddress::default()),
        &OracleWitness,
    )
    .unwrap();
    assert_eq!(
        state_hex(after.context.query.state.get_ref().clone()),
        oracle["afterPull"]
    );
    let value = runtime::ledger::read_root_cell::<runtime::Field, _>(
        after.context.query.state.get_ref(),
        0,
    )
    .unwrap();
    let oracle_value = oracle["valueAfterPull"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert_eq!(value, runtime::Field::from(oracle_value));
    assert_eq!(after.context.private_state, oracle["privateStateAfterPull"]);
    assert_eq!(after.private_transcript_outputs.len(), 1);
    let transcript = &after.private_transcript_outputs[0];
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

#[path = "../../support/oracle_recorded_trace.rs"]
mod direct_trace;

struct TraceWitness(u64);
impl Witnesses<u64> for TraceWitness {
    fn fetch_field(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, runtime::Field) {
        assert_eq!(*context.private_state, 7);
        assert_eq!(context.ledger.v().unwrap(), runtime::Field::default());
        (8, runtime::Field::from(self.0))
    }
}

#[test]
fn exported_pull_recording_matches_witness_privacy_and_complete_typescript_traces() {
    use compact_rust_witnesses_oracle_fixture::ledger_contract::recorded;
    let rows = direct_trace::cases("witnesses_oracle", 2);
    for (row, value) in rows.iter().zip([42, 0]) {
        assert_eq!(row["id"], format!("witnesses_oracle/pull/witness-{value}"));
        assert_eq!(row["privateBefore"], 7);
        let context = initial_state(ConstructorContext::new(7u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let native_context = initial_state(ConstructorContext::new(7u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let native = pull(native_context, &TraceWitness(value)).unwrap();
        let recorded = recorded::pull(context, &TraceWitness(value)).unwrap();
        assert_eq!(native.context.private_state, row["privateAfter"]);
        direct_trace::assert_trace(&native, &recorded, row, state_hex);
    }
}
