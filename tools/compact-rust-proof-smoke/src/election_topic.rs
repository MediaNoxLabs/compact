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

//! Prove the original election authority and phase guarded optional topic write.
use super::*;
use compact_rust_election_oracle_fixture::ledger_contract as contract;
use compact_rust_election_oracle_fixture::types::{MaybeCompact1, PermissibleVotes, PrivateState};
use contract::{LedgerView, Witnesses};
use midnight_compact_runtime as runtime;
const AUTHORITY: [u8; 32] = [
    0x33, 0xef, 0xf3, 0xd5, 0x7e, 0x66, 0xfd, 0x14, 0x2b, 0xb4, 0x08, 0xe4, 0x89, 0x44, 0xa4, 0xd6,
    0xb8, 0xf2, 0xdb, 0xf5, 0xc1, 0x80, 0x96, 0xf8, 0x27, 0xb0, 0x28, 0x3d, 0xbf, 0x91, 0x11, 0xc8,
];

struct FixedWitness;

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

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let topic = runtime::OpaqueString::from("議題 🗳️");
    let initial =
        contract::initial_state(ConstructorContext::new(10_u64), FixedBytes::new(AUTHORITY))?;
    let mut rng = StdRng::seed_from_u64(0x155);
    let deploy = make_deploy(
        root,
        "set_topic",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let native = contract::set_topic(
        observed.circuit_context(10_u64),
        &FixedWitness,
        topic.clone(),
    )?;
    let recorded = contract::recorded::set_topic(
        observed.circuit_context(10_u64),
        &FixedWitness,
        topic.clone(),
    )?;
    if native.gas_cost != recorded.execution.gas_cost
        || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
        || recorded.execution.private_transcript_outputs.len() != 1
        || recorded.execution.context.private_state != 11
        || native.context.query.state.get_ref() != recorded.execution.context.query.state.get_ref()
        || native.context.query.effects != recorded.execution.context.query.effects
    {
        return Err("topic recording differs from native execution".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "set_topic", recorded, topic.clone())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/set_topic.verifier"),
    )?))?;
    let generated = contract::Contract::from(FixedWitness);
    let prepared = generated
        .recording()
        .set_topic_call(&observed, 10_u64, topic.clone())?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("typed topic call differs from manual recording".into());
    }
    check_transaction(root, "set_topic", deploy, prepared, &mut rng, |applied| {
        if applied.data.get_ref() != &expected_state {
            return Err("applied topic state differs from native".into());
        }
        let stored = contract::PublicStateView::from(applied).topic()?;
        if !stored.is_some || stored.value != topic {
            return Err("applied optional topic changed".into());
        }
        Ok(())
    })?;
    println!("election authority/phase guarded Unicode topic proved and applied through ledger-8");
    Ok(())
}

/// Exercise every source successor branch, including the stable final state.
pub(super) fn advance(root: &Path) -> Result<(), Box<dyn Error>> {
    use compact_rust_election_oracle_fixture::types::PublicState;
    for (step, expected_phase) in [
        PublicState::commit,
        PublicState::reveal,
        PublicState::r#final,
        PublicState::r#final,
    ]
    .into_iter()
    .enumerate()
    {
        let initial =
            contract::initial_state(ConstructorContext::new(10_u64), FixedBytes::new(AUTHORITY))?;
        let mut seeded = contract::set_topic(
            initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
            &FixedWitness,
            runtime::OpaqueString::from("議題 🗳️"),
        )?
        .context;
        for _ in 0..step {
            seeded = contract::advance(seeded, &FixedWitness)?.context;
        }
        let private = seeded.private_state;
        let mut rng = StdRng::seed_from_u64(0x158 + step as u64);
        let deploy = make_deploy(
            root,
            "advance",
            seeded.query.state.get_ref().clone(),
            &mut rng,
        )?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native = contract::advance(observed.circuit_context(private), &FixedWitness)?;
        let recorded =
            contract::recorded::advance(observed.circuit_context(private), &FixedWitness)?;
        if native.gas_cost != recorded.execution.gas_cost
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || recorded.execution.private_transcript_outputs.len() != 1
            || recorded.execution.context.private_state != private + 1
            || native.context.query.state.get_ref()
                != recorded.execution.context.query.state.get_ref()
            || native.context.query.effects != recorded.execution.context.query.effects
        {
            return Err("advance recording differs from native execution".into());
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "advance", recorded, ())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/advance.verifier"),
        )?))?;
        let generated = contract::Contract::from(FixedWitness);
        let prepared = generated
            .recording()
            .advance_call(&observed, private)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("typed advance call differs from manual recording".into());
        }
        check_transaction(root, "advance", deploy, prepared, &mut rng, |applied| {
            if applied.data.get_ref() != &expected_state {
                return Err("applied phase state differs from native".into());
            }
            let view = contract::PublicStateView::from(applied);
            if view.state()? != expected_phase || !view.topic()?.is_some {
                return Err("applied phase or optional topic changed".into());
            }
            Ok(())
        })?;
        println!("election advance transition {step} proved and applied through ledger-8");
    }
    Ok(())
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

pub(super) fn add_voter(root: &Path) -> Result<(), Box<dyn Error>> {
    for value in [8_u8, 9] {
        let initial =
            contract::initial_state(ConstructorContext::new(10_u64), FixedBytes::new(AUTHORITY))?;
        let mut seeded = initial.into_circuit_context(runtime::ledger::ContractAddress::default());
        if value == 9 {
            seeded =
                contract::add_voter(seeded, &LedgerWitness::default(), FixedBytes::new([8; 32]))?
                    .context;
        }
        let private = seeded.private_state;
        let mut rng = StdRng::seed_from_u64(0x160 + u64::from(value));
        let deploy = make_deploy(
            root,
            "add_voter",
            seeded.query.state.get_ref().clone(),
            &mut rng,
        )?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let pk = FixedBytes::new([value; 32]);
        let native = contract::add_voter(
            observed.circuit_context(private),
            &LedgerWitness::default(),
            pk,
        )?;
        let witness = LedgerWitness::default();
        let recorded =
            contract::recorded::add_voter(observed.circuit_context(private), &witness, pk)?;
        if *witness.calls.borrow() != ["path", "secret"]
            || native.gas_cost != recorded.execution.gas_cost
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || recorded.execution.private_transcript_outputs.len() != 2
            || recorded.execution.context.private_state != private + 2
            || native.context.query.state.get_ref()
                != recorded.execution.context.query.state.get_ref()
            || native.context.query.effects != recorded.execution.context.query.effects
        {
            return Err("voter recording differs from native execution".into());
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "add_voter", recorded, pk)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/add_voter.verifier"),
        )?))?;
        let generated = contract::Contract::from(LedgerWitness::default());
        let prepared = generated
            .recording()
            .add_voter_call(&observed, private, pk)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("typed voter call differs from recording".into());
        }
        check_transaction(root, "add_voter", deploy, prepared, &mut rng, |applied| {
            if applied.data.get_ref() != &expected_state {
                return Err("applied voter state differs from native".into());
            }
            let view = contract::PublicStateView::from(applied);
            let tree = view.eligible_voters()?;
            if tree.first_free()?.value() != u128::from(value - 7)
                || tree.find_path_for_leaf(pk).is_none()
            {
                return Err("applied voter path or index changed".into());
            }
            Ok(())
        })?;
        println!("election voter {value} proved and applied through ledger-8");
    }
    Ok(())
}
