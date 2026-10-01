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

use compact_rust_election_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, add_voter, advance, initial_state, set_topic,
};
use compact_rust_election_oracle_fixture::types::{MaybeCompact1, PermissibleVotes, PrivateState};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

const AUTHORITY: [u8; 32] = [
    0x33, 0xef, 0xf3, 0xd5, 0x7e, 0x66, 0xfd, 0x14, 0x2b, 0xb4, 0x08, 0xe4, 0x89, 0x44, 0xa4, 0xd6,
    0xb8, 0xf2, 0xdb, 0xf5, 0xc1, 0x80, 0x96, 0xf8, 0x27, 0xb0, 0x28, 0x3d, 0xbf, 0x91, 0x11, 0xc8,
];

struct FixedWitness;

impl Witnesses<()> for FixedWitness {
    fn private_secret_key(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), runtime::FixedBytes<32>) {
        ((), runtime::FixedBytes::new([7; 32]))
    }

    fn private_state(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), PrivateState) {
        ((), PrivateState::default())
    }

    fn private_state_advance(&self, _context: WitnessContext<'_, (), LedgerView<'_>>) -> ((), ()) {
        ((), ())
    }

    fn private_vote_record(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        _vote: PermissibleVotes,
    ) -> ((), ()) {
        ((), ())
    }

    fn private_vote(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), PermissibleVotes) {
        ((), PermissibleVotes::default())
    }

    fn context_eligible_voters_path_of(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        _key: runtime::FixedBytes<32>,
    ) -> ((), MaybeCompact1) {
        ((), MaybeCompact1::default())
    }

    fn context_committed_votes_path_of(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        _key: runtime::FixedBytes<32>,
    ) -> ((), MaybeCompact1) {
        ((), MaybeCompact1::default())
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "advance",
        "vote$reveal",
        "add_voter",
        "vote$commit",
        "set_topic",
    ] {
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

#[test]
fn election_owner_operations_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/election-oracle.json"
    ))
    .unwrap();
    let witness = FixedWitness;
    let initial = initial_state(
        ConstructorContext::new(()),
        runtime::FixedBytes::new(AUTHORITY),
    )
    .unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]["stateHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let topic = set_topic(context, &witness, runtime::OpaqueString::from("hello")).unwrap();
    assert_eq!(
        state_hex(topic.context.query.state.get_ref().clone()),
        oracle["afterSetTopic"]["stateHex"]
    );
    let advanced = advance(topic.context, &witness).unwrap();
    assert_eq!(
        state_hex(advanced.context.query.state.get_ref().clone()),
        oracle["afterAdvance"]["stateHex"]
    );
    let initial = initial_state(
        ConstructorContext::new(()),
        runtime::FixedBytes::new(AUTHORITY),
    )
    .unwrap();
    let voter = add_voter(
        initial.into_circuit_context(ContractAddress::default()),
        &witness,
        runtime::FixedBytes::new(AUTHORITY),
    )
    .unwrap();
    assert_eq!(
        state_hex(voter.context.query.state.get_ref().clone()),
        oracle["afterAddVoter"]["stateHex"]
    );
}
