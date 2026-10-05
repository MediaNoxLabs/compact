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

//! Prove both ballots through membership guards, selected tally and ordered private callbacks.
use super::election_membership_support as support;
use super::*;
use compact_rust_election_oracle_fixture::{ledger_contract as contract, types::PermissibleVotes};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (index, ballot) in [PermissibleVotes::yes, PermissibleVotes::no]
        .into_iter()
        .enumerate()
    {
        let seeded = support::reveal_seeded(ballot, false, false, false)?;
        let mut rng = StdRng::seed_from_u64(0x165 + index as u64);
        let deploy = make_deploy(
            root,
            "vote$reveal",
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
        let private = support::Private {
            phase: 1,
            ballot: index as u8,
            calls: 0,
        };
        let native = contract::vote_reveal(
            observed.circuit_context(private.clone()),
            &support::Witness::default(),
        )?;
        let witness = support::Witness::default();
        let recorded =
            contract::recorded::vote_reveal(observed.circuit_context(private.clone()), &witness)?;
        if native.gas_cost != recorded.execution.gas_cost
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || recorded.execution.private_transcript_outputs.len() != 5
            || native.context.private_state != recorded.execution.context.private_state
            || recorded.execution.context.private_state.phase != 2
            || recorded.execution.context.private_state.ballot != index as u8
            || recorded.execution.context.private_state.calls != 5
            || *witness.calls.borrow() != ["state", "secret", "vote", "path", "advance"]
            || native.context.query.state.get_ref()
                != recorded.execution.context.query.state.get_ref()
            || native.context.query.effects != recorded.execution.context.query.effects
        {
            return Err("reveal recording differs from native".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "vote$reveal", recorded, ())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/vote$reveal.verifier"),
        )?))?;
        let generated = contract::Contract::from(support::Witness::default());
        let prepared = generated
            .recording()
            .vote_reveal_call(&observed, private)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("typed reveal call differs from recording".into());
        }
        check_transaction(root, "vote$reveal", deploy, prepared, &mut rng, |applied| {
            if applied.data.get_ref() != &expected {
                return Err("applied tally/nullifier differs from native".into());
            }
            let view = contract::PublicStateView::from(applied);
            if view.tally_yes()?.value() != if index == 0 { 1 } else { 0 }
                || view.tally_no()?.value() != if index == 1 { 1 } else { 0 }
                || view.committed_votes()?.first_free()?.value() != 1
            {
                return Err("selected tally or commitment state mismatch".into());
            }
            Ok(())
        })?;
        println!("election ballot {index} revealed, proved and ledger-applied");
    }
    Ok(())
}
