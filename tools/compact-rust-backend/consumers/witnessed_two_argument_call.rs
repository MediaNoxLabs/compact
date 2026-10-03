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

use compact_contract_witness_cell_write::ledger_contract::{
    Contract, LedgerView, Witnesses, initial_state,
};
use compact_contract_witness_cell_write::runtime::Field;
use compact_contract_witness_cell_write::runtime::context::{ConstructorContext, WitnessContext};
use compact_contract_witness_cell_write::runtime::ledger::{
    ContractAddress, ContractState, DefaultDB,
};
use compact_contract_witness_cell_write::runtime::transaction::{
    Observation, ObservedCallError, ObservedContractState, decode_verifier_key,
};

struct Secret;

impl Witnesses<u64> for Secret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        let private = *context.private_state;
        assert_eq!(context.ledger.cell().unwrap(), Field::from(0_u64));
        (private + 1, seed + Field::from(private))
    }
}

#[test]
fn witnessed_two_argument_call_borrows_witnesses_from_generated_crate() {
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    let mut state = ContractState::<DefaultDB>::default();
    state.data = initial.ledger_state;
    let observed = ObservedContractState::new(
        ContractAddress::default(),
        state,
        Observation {
            transaction_hash: [1; 32],
            block_hash: [2; 32],
            block_height: 3,
        },
    );
    let verifier = decode_verifier_key(
        &std::fs::read(std::env::var("COMPACT_RUST_WITNESS_OFFSET_VERIFIER").unwrap()).unwrap(),
    )
    .unwrap();
    let contract = Contract::from(Secret);
    let call = contract
        .recording()
        .write_offset_call(&observed, 7_u64, Field::from(2_u64), Field::from(5_u64))
        .unwrap();
    assert_eq!(call.recorded().execution.context.private_state, 8);
    assert_eq!(call.recorded().execution.private_transcript_outputs.len(), 1);
    assert!(matches!(
        call.prepare(verifier, Field::from(0_u64)),
        Err(ObservedCallError::MissingOperation(name)) if name == "write_offset"
    ));
}
