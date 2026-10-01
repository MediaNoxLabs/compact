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

use compact_rust_wide_uint_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, readWide, writeWide,
};
use compact_rust_wide_uint_oracle_fixture::pure_circuits::maxWide;
use midnight_compact_runtime::WideUint;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

type Uint248 = WideUint<{ (1_u128 << 120) - 1 }, { u128::MAX }>;

struct OracleWitness;

impl Witnesses<u64> for OracleWitness {
    fn nextWide(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, Uint248) {
        (
            *context.private_state + 1,
            Uint248::from_le_bytes(&[0xff; 31]).unwrap(),
        )
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["readWide", "writeWide"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn wide_uint_cell_and_witness_transcript_match_typescript() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/wide-uint-oracle.json"
    ))
    .unwrap();
    let maximum = Uint248::from_le_bytes(&[0xff; 31]).unwrap();
    assert_eq!(maxWide().unwrap(), maximum);
    assert_eq!(maximum.as_field().as_le_bytes().len(), 32);
    assert_eq!(maximum.as_le_bytes().len(), 31);
    assert_eq!(
        reference["maxWide"],
        "452312848583266388373324160190187140051835877600158453279131187530910662655"
    );

    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let write = writeWide(context, &OracleWitness).unwrap();
    assert_eq!(
        write.context.private_state,
        reference["privateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        reference["afterWriteHex"]
    );
    let outputs = reference["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(write.private_transcript_outputs.len(), outputs.len());
    let atoms = write.private_transcript_outputs[0]
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        outputs[0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&write.private_transcript_outputs[0].alignment).unwrap(),
        outputs[0]["alignment"]
    );
    assert_eq!(readWide(write.context).unwrap().result, maximum);
    assert_eq!(reference["read"], reference["maxWide"]);
}
