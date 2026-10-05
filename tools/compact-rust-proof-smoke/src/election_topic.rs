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
