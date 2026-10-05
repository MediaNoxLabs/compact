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

//! Prove both ballots through membership guards and ordered private callbacks.
use super::*;
use compact_rust_election_oracle_fixture::{ledger_contract as contract, types::PermissibleVotes};
#[path = "../../../tests-rust-backend/election-oracle/support/commit.rs"]
mod support;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (index, ballot) in [PermissibleVotes::yes, PermissibleVotes::no]
        .into_iter()
        .enumerate()
    {
        let seeded = support::seeded(false, false, false)?;
        let mut rng = StdRng::seed_from_u64(0x162 + index as u64);
        let deploy = make_deploy(
            root,
            "vote$commit",
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
        let native = contract::vote_commit(
            observed.circuit_context(support::Private::default()),
            &support::Witness::default(),
            ballot,
        )?;
        let witness = support::Witness::default();
        let recorded = contract::recorded::vote_commit(
            observed.circuit_context(support::Private::default()),
            &witness,
            ballot,
        )?;
        if native.gas_cost != recorded.execution.gas_cost
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || recorded.execution.private_transcript_outputs.len() != 5
            || native.context.private_state != recorded.execution.context.private_state
            || recorded.execution.context.private_state.phase != 1
            || recorded.execution.context.private_state.ballot != index as u8
            || recorded.execution.context.private_state.calls != 5
            || *witness.calls.borrow() != ["state", "record", "secret", "path", "advance"]
            || native.context.query.state.get_ref()
                != recorded.execution.context.query.state.get_ref()
            || native.context.query.effects != recorded.execution.context.query.effects
        {
            return Err("commit recording differs from native".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "vote$commit", recorded, ballot)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/vote$commit.verifier"),
        )?))?;
        let generated = contract::Contract::from(support::Witness::default());
        let prepared = generated
            .recording()
            .vote_commit_call(&observed, support::Private::default(), ballot)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("typed commit call differs from recording".into());
        }
        check_transaction(root, "vote$commit", deploy, prepared, &mut rng, |applied| {
            if applied.data.get_ref() != &expected {
                return Err("applied commitment/nullifier differs from native".into());
            }
            if contract::PublicStateView::from(applied)
                .committed_votes()?
                .first_free()?
                .value()
                != 1
            {
                return Err("committed ballot missing".into());
            }
            Ok(())
        })?;
        println!("election ballot {index} committed, proved and ledger-applied");
    }
    Ok(())
}
