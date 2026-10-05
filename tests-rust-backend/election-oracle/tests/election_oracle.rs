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

impl Witnesses<u64> for FixedWitness {
    fn private_secret_key(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, runtime::FixedBytes<32>) {
        (
            *context.private_state + 1,
            runtime::FixedBytes::new([7; 32]),
        )
    }

    fn private_state(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, PrivateState) {
        (*context.private_state, PrivateState::default())
    }

    fn private_state_advance(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, ()) {
        (*context.private_state, ())
    }

    fn private_vote_record(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        _vote: PermissibleVotes,
    ) -> (u64, ()) {
        (*context.private_state, ())
    }

    fn private_vote(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, PermissibleVotes) {
        (*context.private_state, PermissibleVotes::default())
    }

    fn context_eligible_voters_path_of(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        _key: runtime::FixedBytes<32>,
    ) -> (u64, MaybeCompact1) {
        (*context.private_state, MaybeCompact1::default())
    }

    fn context_committed_votes_path_of(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        _key: runtime::FixedBytes<32>,
    ) -> (u64, MaybeCompact1) {
        (*context.private_state, MaybeCompact1::default())
    }
}

#[derive(Default)]
struct LedgerWitness {
    calls: std::cell::RefCell<Vec<&'static str>>,
}

impl Witnesses<u64> for LedgerWitness {
    fn private_secret_key(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, runtime::FixedBytes<32>) {
        self.calls.borrow_mut().push("secret");
        (
            *context.private_state + 1,
            runtime::FixedBytes::new([7; 32]),
        )
    }

    fn private_state(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, PrivateState) {
        (*context.private_state, PrivateState::default())
    }

    fn private_state_advance(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, ()) {
        (*context.private_state, ())
    }

    fn private_vote_record(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        _vote: PermissibleVotes,
    ) -> (u64, ()) {
        (*context.private_state, ())
    }

    fn private_vote(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, PermissibleVotes) {
        (*context.private_state, PermissibleVotes::default())
    }

    fn context_eligible_voters_path_of(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        key: runtime::FixedBytes<32>,
    ) -> (u64, MaybeCompact1) {
        self.calls.borrow_mut().push("path");
        // The view dereferences to the local ledger8 tree inspection API.
        // It does not issue a metered VM query, matching the TS snapshot oracle.
        let path = context
            .ledger
            .eligible_voters()
            .unwrap()
            .find_path_for_leaf(key);
        let result = match path {
            Some(path) => MaybeCompact1 {
                is_some: true,
                value:
                    compact_rust_election_oracle_fixture::types::MerkleTreePath::from_ledger_path(
                        path,
                    )
                    .unwrap(),
            },
            None => MaybeCompact1::default(),
        };
        (*context.private_state + 1, result)
    }

    fn context_committed_votes_path_of(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        _key: runtime::FixedBytes<32>,
    ) -> (u64, MaybeCompact1) {
        (*context.private_state, MaybeCompact1::default())
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

#[test]
fn recorded_topic_preserves_authority_phase_opaque_state_and_private_effects() {
    use compact_rust_election_oracle_fixture::ledger_contract::recorded;
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/election-topic-oracle.json"
    ))
    .unwrap();
    let initial = |authority| {
        initial_state(
            ConstructorContext::new(10_u64),
            runtime::FixedBytes::new(authority),
        )
        .unwrap()
        .into_circuit_context(ContractAddress::default())
    };
    for scenario in oracle["scenarios"].as_array().unwrap() {
        let topic = runtime::OpaqueString::from(scenario["topic"].as_str().unwrap());
        let native = set_topic(initial(AUTHORITY), &FixedWitness, topic.clone()).unwrap();
        let recorded = recorded::set_topic(initial(AUTHORITY), &FixedWitness, topic).unwrap();
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref()
        );
        assert_eq!(
            recorded.execution.context.private_state,
            scenario["privateState"].as_u64().unwrap()
        );
        assert_eq!(
            recorded.execution.private_transcript_outputs.len(),
            scenario["privateOutputs"].as_u64().unwrap() as usize
        );
        assert_eq!(
            recorded.execution.private_transcript_outputs,
            vec![runtime::fab::AlignedValue::from(runtime::FixedBytes::new(
                [7; 32]
            ))]
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            scenario["stateHex"]
        );
        for (output, expected) in recorded
            .execution
            .private_transcript_outputs
            .iter()
            .zip(scenario["privateTranscriptOutputs"].as_array().unwrap())
        {
            let atoms: Vec<_> = output.value.0.iter().map(|atom| &atom.0).collect();
            assert_eq!(serde_json::to_value(atoms).unwrap(), expected["value"]);
            assert_eq!(
                serde_json::to_value(&output.alignment).unwrap(),
                expected["alignment"]
            );
        }
        let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
        let queries = scenario["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 3);
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = queries
                .iter()
                .map(|query| {
                    query["gasCost"][key]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(gas[key], sum);
        }
        let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        for operation in program.as_array_mut().unwrap() {
            if let Some(pop) = operation.get_mut("popeq") {
                pop["result"] = serde_json::Value::Null;
            }
        }
        let expected: Vec<_> = queries
            .iter()
            .flat_map(|query| query["program"].as_array().unwrap().iter().cloned())
            .collect();
        assert_eq!(program, serde_json::json!(expected));
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                replay_gas[key],
                scenario["replayGas"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            );
        }
        assert_eq!(
            state_hex(replay.context.state.get_ref().clone()),
            scenario["stateHex"]
        );
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
    }
    assert_eq!(
        recorded::set_topic(initial([0; 32]), &FixedWitness, "denied".into())
            .err()
            .unwrap()
            .to_string(),
        oracle["wrongAuthority"]
    );
    let configured = set_topic(initial(AUTHORITY), &FixedWitness, "ready".into()).unwrap();
    let advanced = advance(configured.context, &FixedWitness).unwrap();
    assert_eq!(
        recorded::set_topic(advanced.context, &FixedWitness, "denied".into())
            .err()
            .unwrap()
            .to_string(),
        oracle["wrongPhase"]
    );
}

#[test]
fn recorded_advance_preserves_all_phase_transitions_and_optional_presence() {
    use compact_rust_election_oracle_fixture::ledger_contract::recorded;
    use compact_rust_election_oracle_fixture::types::PublicState;
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/election-advance-oracle.json"
    ))
    .unwrap();
    let initial = |authority| {
        initial_state(
            ConstructorContext::new(10_u64),
            runtime::FixedBytes::new(authority),
        )
        .unwrap()
        .into_circuit_context(ContractAddress::default())
    };
    for scenario in oracle["scenarios"].as_array().unwrap() {
        let seeded = || {
            let mut context = set_topic(
                initial(AUTHORITY),
                &FixedWitness,
                runtime::OpaqueString::from(scenario["topic"].as_str().unwrap()),
            )
            .unwrap()
            .context;
            for _ in 0..scenario["step"].as_u64().unwrap() {
                context = advance(context, &FixedWitness).unwrap().context;
            }
            context
        };
        let native = advance(seeded(), &FixedWitness).unwrap();
        let recorded = recorded::advance(seeded(), &FixedWitness).unwrap();
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref()
        );
        assert_eq!(
            recorded.execution.context.private_state,
            scenario["privateState"].as_u64().unwrap()
        );
        assert_eq!(recorded.execution.private_transcript_outputs.len(), 1);
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            scenario["stateHex"]
        );
        let expected_phase = match scenario["phase"].as_u64().unwrap() {
            1 => PublicState::commit,
            2 => PublicState::reveal,
            3 => PublicState::r#final,
            _ => unreachable!(),
        };
        let value: PublicState =
            runtime::ledger::read_cell(match recorded.execution.context.query.state.get_ref() {
                StateValue::Array(fields) => fields.get(1).unwrap(),
                _ => unreachable!(),
            })
            .unwrap();
        assert_eq!(value, expected_phase);
        for (output, expected) in recorded
            .execution
            .private_transcript_outputs
            .iter()
            .zip(scenario["privateTranscriptOutputs"].as_array().unwrap())
        {
            let atoms: Vec<_> = output.value.0.iter().map(|atom| &atom.0).collect();
            assert_eq!(serde_json::to_value(atoms).unwrap(), expected["value"]);
            assert_eq!(
                serde_json::to_value(&output.alignment).unwrap(),
                expected["alignment"]
            );
        }
        let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
        let queries = scenario["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 4);
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = queries
                .iter()
                .map(|query| {
                    query["gasCost"][key]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(gas[key], sum);
        }
        let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        for operation in program.as_array_mut().unwrap() {
            if let Some(pop) = operation.get_mut("popeq") {
                pop["result"] = serde_json::Value::Null;
            }
        }
        let expected: Vec<_> = queries
            .iter()
            .flat_map(|query| query["program"].as_array().unwrap().iter().cloned())
            .collect();
        assert_eq!(program, serde_json::json!(expected));
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                replay_gas[key],
                scenario["replayGas"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            );
        }
        assert_eq!(
            state_hex(replay.context.state.get_ref().clone()),
            scenario["stateHex"]
        );
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
    }
    assert_eq!(
        recorded::advance(initial([0; 32]), &FixedWitness)
            .err()
            .unwrap()
            .to_string(),
        oracle["wrongAuthority"]
    );
    assert_eq!(
        recorded::advance(initial(AUTHORITY), &FixedWitness)
            .err()
            .unwrap()
            .to_string(),
        oracle["absentTopic"]
    );
}

#[test]
fn recorded_add_voter_uses_live_tree_witness_and_preserves_two_private_outputs() {
    use compact_rust_election_oracle_fixture::ledger_contract::recorded;
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/election-add-voter-oracle.json"
    ))
    .unwrap();
    let initial = |authority| {
        initial_state(
            ConstructorContext::new(10_u64),
            runtime::FixedBytes::new(authority),
        )
        .unwrap()
        .into_circuit_context(ContractAddress::default())
    };
    for scenario in oracle["scenarios"].as_array().unwrap() {
        let value = scenario["value"].as_u64().unwrap() as u8;
        let seeded = || {
            let context = initial(AUTHORITY);
            if value == 9 {
                add_voter(
                    context,
                    &LedgerWitness::default(),
                    runtime::FixedBytes::new([8; 32]),
                )
                .unwrap()
                .context
            } else {
                context
            }
        };
        let native_witness = LedgerWitness::default();
        let witness = LedgerWitness::default();
        let native = add_voter(
            seeded(),
            &native_witness,
            runtime::FixedBytes::new([value; 32]),
        )
        .unwrap();
        let recorded =
            recorded::add_voter(seeded(), &witness, runtime::FixedBytes::new([value; 32])).unwrap();
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            scenario["witnessCalls"]
        );
        assert_eq!(*native_witness.calls.borrow(), *witness.calls.borrow());
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref()
        );
        assert_eq!(
            recorded.execution.context.private_state,
            scenario["privateState"].as_u64().unwrap()
        );
        assert_eq!(recorded.execution.private_transcript_outputs.len(), 2);
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            scenario["stateHex"]
        );
        for (output, expected) in recorded
            .execution
            .private_transcript_outputs
            .iter()
            .zip(scenario["privateTranscriptOutputs"].as_array().unwrap())
        {
            let atoms: Vec<_> = output.value.0.iter().map(|atom| &atom.0).collect();
            assert_eq!(serde_json::to_value(atoms).unwrap(), expected["value"]);
            assert_eq!(
                serde_json::to_value(&output.alignment).unwrap(),
                expected["alignment"]
            );
        }
        let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
        let queries = scenario["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 3);
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = queries
                .iter()
                .map(|query| {
                    query["gasCost"][key]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(gas[key], sum);
        }
        let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        for operation in program.as_array_mut().unwrap() {
            if let Some(pop) = operation.get_mut("popeq") {
                pop["result"] = serde_json::Value::Null;
            }
        }
        let expected: Vec<_> = queries
            .iter()
            .flat_map(|query| query["program"].as_array().unwrap().iter().cloned())
            .collect();
        assert_eq!(program, serde_json::json!(expected));
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                replay_gas[key],
                scenario["replayGas"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            );
        }
        assert_eq!(
            state_hex(replay.context.state.get_ref().clone()),
            scenario["stateHex"]
        );
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
        let witness = LedgerWitness::default();
        assert_eq!(
            recorded::add_voter(native.context, &witness, runtime::FixedBytes::new([8; 32]))
                .err()
                .unwrap()
                .to_string(),
            oracle["duplicate"]["error"]
        );
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            oracle["duplicate"]["witnessCalls"]
        );
    }
    let witness = LedgerWitness::default();
    assert_eq!(
        recorded::add_voter(
            initial([0; 32]),
            &witness,
            runtime::FixedBytes::new([8; 32])
        )
        .err()
        .unwrap()
        .to_string(),
        oracle["wrongAuthority"]["error"]
    );
    assert_eq!(
        serde_json::to_value(&*witness.calls.borrow()).unwrap(),
        oracle["wrongAuthority"]["witnessCalls"]
    );
    let configured = set_topic(initial(AUTHORITY), &FixedWitness, "ready".into()).unwrap();
    let advanced = advance(configured.context, &FixedWitness).unwrap();
    let witness = LedgerWitness::default();
    assert_eq!(
        recorded::add_voter(
            advanced.context,
            &witness,
            runtime::FixedBytes::new([8; 32])
        )
        .err()
        .unwrap()
        .to_string(),
        oracle["wrongPhase"]["error"]
    );
    assert_eq!(
        serde_json::to_value(&*witness.calls.borrow()).unwrap(),
        oracle["wrongPhase"]["witnessCalls"]
    );
}

#[path = "../support/commit.rs"]
mod commit_support;

#[test]
fn recorded_commit_preserves_ballots_membership_and_short_circuit_witnesses() {
    use commit_support::{Mode, Private, Witness, seeded};
    use compact_rust_election_oracle_fixture::ledger_contract::{recorded, vote_commit};
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/election-commit-oracle.json"
    ))
    .unwrap();
    for scenario in oracle["scenarios"].as_array().unwrap() {
        let ballot = if scenario["ballot"] == 0 {
            PermissibleVotes::yes
        } else {
            PermissibleVotes::no
        };
        let native_witness = Witness::default();
        let witness = Witness::default();
        let native = vote_commit(
            seeded(false, false, false).unwrap(),
            &native_witness,
            ballot,
        )
        .unwrap();
        let recorded =
            recorded::vote_commit(seeded(false, false, false).unwrap(), &witness, ballot).unwrap();
        assert_eq!(*native_witness.calls.borrow(), *witness.calls.borrow());
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            scenario["witnessCalls"]
        );
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
        assert_eq!(
            native.context.private_state,
            recorded.execution.context.private_state
        );
        let private = &recorded.execution.context.private_state;
        assert_eq!(
            serde_json::json!({"phase":private.phase,"ballot":private.ballot,"calls":private.calls}),
            scenario["privateState"]
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref()
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            scenario["stateHex"]
        );
        assert_eq!(recorded.execution.private_transcript_outputs.len(), 5);
        for (output, expected) in recorded
            .execution
            .private_transcript_outputs
            .iter()
            .zip(scenario["privateTranscriptOutputs"].as_array().unwrap())
        {
            let atoms: Vec<_> = output.value.0.iter().map(|atom| &atom.0).collect();
            assert_eq!(serde_json::to_value(atoms).unwrap(), expected["value"]);
            assert_eq!(
                serde_json::to_value(&output.alignment).unwrap(),
                expected["alignment"]
            );
        }
        let queries = scenario["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 5);
        let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = queries
                .iter()
                .map(|query| {
                    query["gasCost"][key]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(gas[key], sum);
        }
        let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        for op in program.as_array_mut().unwrap() {
            if let Some(pop) = op.get_mut("popeq") {
                pop["result"] = serde_json::Value::Null;
            }
        }
        let expected: Vec<_> = queries
            .iter()
            .flat_map(|query| query["program"].as_array().unwrap().iter().cloned())
            .collect();
        assert_eq!(program, serde_json::json!(expected));
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                replay_gas[key],
                scenario["replayGas"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            );
        }
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            state_hex(replay.context.state.get_ref().clone()),
            scenario["stateHex"]
        );
    }
    for name in [
        "wrongPhase",
        "wrongPrivate",
        "duplicate",
        "missingPath",
        "wrongRoot",
        "wrongLeaf",
        "malformed",
    ] {
        let make = || {
            let mut context = seeded(
                name == "missingPath",
                name == "wrongLeaf",
                name == "wrongPhase",
            )
            .unwrap();
            if name == "wrongPrivate" {
                context.private_state.phase = 1;
            }
            if name == "duplicate" {
                context = vote_commit(context, &Witness::default(), PermissibleVotes::yes)
                    .unwrap()
                    .context;
                context.private_state = Private::default();
            }
            context
        };
        let mode = match name {
            "wrongRoot" => Mode::WrongRoot,
            "wrongLeaf" => Mode::WrongLeaf,
            "malformed" => Mode::Malformed,
            _ => Mode::Normal,
        };
        let native_witness = Witness {
            mode,
            ..Default::default()
        };
        let witness = Witness {
            mode,
            ..Default::default()
        };
        let native = vote_commit(make(), &native_witness, PermissibleVotes::yes)
            .err()
            .unwrap()
            .to_string();
        let recorded = recorded::vote_commit(make(), &witness, PermissibleVotes::yes)
            .err()
            .unwrap()
            .to_string();
        assert_eq!(native, recorded, "{name}");
        if name == "malformed" {
            assert!(recorded.contains("Merkle path depth"));
            assert!(
                oracle[name]["error"]
                    .as_str()
                    .unwrap()
                    .starts_with("type error:")
            );
        } else {
            assert_eq!(recorded, oracle[name]["error"], "{name}");
        }
        assert_eq!(*native_witness.calls.borrow(), *witness.calls.borrow());
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            oracle[name]["witnessCalls"],
            "{name}"
        );
    }
}

#[test]
fn recorded_reveal_preserves_ballots_membership_counters_and_short_circuit_witnesses() {
    use commit_support::{Mode, Private, Witness, reveal_seeded};
    use compact_rust_election_oracle_fixture::ledger_contract::{recorded, vote_reveal};
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/election-reveal-oracle.json"
    ))
    .unwrap();
    for scenario in oracle["scenarios"].as_array().unwrap() {
        let ballot = if scenario["ballot"] == 0 {
            PermissibleVotes::yes
        } else {
            PermissibleVotes::no
        };
        let native_witness = Witness::default();
        let witness = Witness::default();
        let native = vote_reveal(
            reveal_seeded(ballot, false, false, false).unwrap(),
            &native_witness,
        )
        .unwrap();
        let recorded = recorded::vote_reveal(
            reveal_seeded(ballot, false, false, false).unwrap(),
            &witness,
        )
        .unwrap();
        assert_eq!(*native_witness.calls.borrow(), *witness.calls.borrow());
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            scenario["witnessCalls"]
        );
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
        assert_eq!(
            native.context.private_state,
            recorded.execution.context.private_state
        );
        let private = &recorded.execution.context.private_state;
        assert_eq!(
            serde_json::json!({"phase":private.phase,"ballot":private.ballot,"calls":private.calls}),
            scenario["privateState"]
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref()
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            scenario["stateHex"]
        );
        let state = &recorded.execution.context.query.state;
        assert_eq!(
            compact_rust_election_oracle_fixture::ledger_slots::tally_yes
                .inspect(state.get_ref())
                .unwrap(),
            if ballot == PermissibleVotes::yes {
                1
            } else {
                0
            }
        );
        assert_eq!(
            compact_rust_election_oracle_fixture::ledger_slots::tally_no
                .inspect(state.get_ref())
                .unwrap(),
            if ballot == PermissibleVotes::no { 1 } else { 0 }
        );
        assert_eq!(recorded.execution.private_transcript_outputs.len(), 5);
        for (output, expected) in recorded
            .execution
            .private_transcript_outputs
            .iter()
            .zip(scenario["privateTranscriptOutputs"].as_array().unwrap())
        {
            let atoms: Vec<_> = output.value.0.iter().map(|atom| &atom.0).collect();
            assert_eq!(serde_json::to_value(atoms).unwrap(), expected["value"]);
            assert_eq!(
                serde_json::to_value(&output.alignment).unwrap(),
                expected["alignment"]
            );
        }
        let queries = scenario["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 5);
        let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = queries
                .iter()
                .map(|query| {
                    query["gasCost"][key]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(gas[key], sum);
        }
        let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        for op in program.as_array_mut().unwrap() {
            if let Some(pop) = op.get_mut("popeq") {
                pop["result"] = serde_json::Value::Null;
            }
        }
        let expected: Vec<_> = queries
            .iter()
            .flat_map(|query| query["program"].as_array().unwrap().iter().cloned())
            .collect();
        assert_eq!(program, serde_json::json!(expected));
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                replay_gas[key],
                scenario["replayGas"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            );
        }
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            state_hex(replay.context.state.get_ref().clone()),
            scenario["stateHex"]
        );
    }
    for name in [
        "wrongPhase",
        "wrongPrivate",
        "duplicate",
        "repeated",
        "missingPath",
        "wrongRoot",
        "wrongLeaf",
        "malformed",
    ] {
        let make = || {
            let mut context = reveal_seeded(
                PermissibleVotes::yes,
                name == "missingPath",
                name == "wrongPhase",
                name == "wrongLeaf",
            )
            .unwrap();
            if name == "wrongPrivate" {
                context.private_state.phase = 0;
            }
            if name == "duplicate" || name == "repeated" {
                context = vote_reveal(context, &Witness::default()).unwrap().context;
                if name == "duplicate" {
                    context.private_state = Private {
                        phase: 1,
                        ..Default::default()
                    };
                }
            }
            context
        };
        let mode = match name {
            "wrongRoot" => Mode::WrongRoot,
            "wrongLeaf" => Mode::WrongLeaf,
            "malformed" => Mode::Malformed,
            _ => Mode::Normal,
        };
        let native_witness = Witness {
            mode,
            ..Default::default()
        };
        let witness = Witness {
            mode,
            ..Default::default()
        };
        let native = vote_reveal(make(), &native_witness)
            .err()
            .unwrap()
            .to_string();
        let recorded = recorded::vote_reveal(make(), &witness)
            .err()
            .unwrap()
            .to_string();
        assert_eq!(native, recorded, "{name}");
        if name == "malformed" {
            assert!(recorded.contains("Merkle path depth"));
            assert!(
                oracle[name]["error"]
                    .as_str()
                    .unwrap()
                    .starts_with("type error:")
            );
        } else {
            assert_eq!(recorded, oracle[name]["error"], "{name}");
        }
        assert_eq!(*native_witness.calls.borrow(), *witness.calls.borrow());
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            oracle[name]["witnessCalls"],
            "{name}"
        );
    }
}
