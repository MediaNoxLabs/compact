// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original source reveal from explicit prior-state fixtures, not funded setup.
use super::micro_dao_reveal_support as support;
use super::*;
use compact_rust_test_center_micro_dao_fixture::{ledger_contract as c, ledger_slots as slots};
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (i, (ballot, round)) in [(true, 0u64), (false, 7u64)].into_iter().enumerate() {
        let seeded = support::seeded(round, ballot, Some(round))?;
        let private = seeded.private_state.clone();
        let mut rng = StdRng::seed_from_u64(0x0192_0000 + i as u64);
        let deploy = make_deploy(
            root,
            "vote_reveal",
            seeded.query.state.get_ref().clone(),
            &mut rng,
        )?;
        let address = deploy.address();
        let observed = ObservedContractState::new(
            address,
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native = c::vote_reveal(
            observed.circuit_context(private.clone()),
            &support::Witness::new(round, support::Mode::Normal),
        )?;
        let witness = support::Witness::new(round, support::Mode::Normal);
        let recorded =
            c::recorded::vote_reveal(observed.circuit_context(private.clone()), &witness)?;
        if native.gas_cost != recorded.execution.gas_cost
            || native.context.private_state != recorded.execution.context.private_state
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || native.context.query.state != recorded.execution.context.query.state
            || native.context.query.effects != recorded.execution.context.query.effects
            || recorded.execution.private_transcript_outputs.len() != 5
            || *witness.calls.borrow() != ["state", "secret", "vote", "path", "advance"]
        {
            return Err("original reveal native/recorded mismatch".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "vote_reveal", recorded, ())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/vote_reveal.verifier"),
        )?))?;
        let generated = c::Contract::from(support::Witness::new(round, support::Mode::Normal));
        let prepared = generated
            .recording()
            .vote_reveal_call(&observed, private.clone())?
            .prepare(verifier, Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("original reveal typed/direct call differs".into());
        }
        check_transaction(root, "vote_reveal", deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected
                || slots::yes.inspect(state.data.get_ref())? != u64::from(ballot)
                || slots::no.inspect(state.data.get_ref())? != u64::from(!ballot)
                || slots::round.inspect(state.data.get_ref())? != round
            {
                return Err("applied reveal tally/nullifier/round mismatch".into());
            }
            let applied = ObservedContractState::new(
                address,
                state.clone(),
                Observation {
                    transaction_hash: [0; 32],
                    block_hash: [0; 32],
                    block_height: 0,
                },
            );
            let repeated = c::recorded::vote_reveal(
                applied.circuit_context(private.clone()),
                &support::Witness::new(round, support::Mode::Normal),
            )
            .err()
            .ok_or("repeated reveal unexpectedly succeeded")?;
            if !repeated.to_string().contains("Attempted to double vote") {
                return Err("applied reveal nullifier guard differs".into());
            }
            Ok(())
        })?;
        println!(
            "original microDAO.vote_reveal ballot={ballot} round={round} proved, verified and ledger-applied; applied-state duplicate rejected; prior-state fixture and shared unbalanced smoke policy, no funded setup claim"
        );
    }
    Ok(())
}
