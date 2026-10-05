// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original Coracle moves from explicit prior states, not funded game starts.
use super::coracle_guess_support as support;
use super::*;
use compact_rust_test_center_coracle_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (i, red) in [true, false].into_iter().enumerate() {
        let settings = support::Settings {
            red,
            ..Default::default()
        };
        let seeded = support::seeded(&settings)?;
        let private = seeded.private_state.clone();
        let mut rng = StdRng::seed_from_u64(0x0193_0000 + i as u64);
        let deploy = make_deploy(
            root,
            "guess",
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
        let position = Field::from(if red { 4u64 } else { 9u64 });
        let native = c::guess(
            observed.circuit_context(private.clone()),
            &support::Witness::new(settings.clone()),
            position,
        )?;
        let witness = support::Witness::new(settings.clone());
        let recorded = c::recorded::guess(
            observed.circuit_context(private.clone()),
            &witness,
            position,
        )?;
        if native.gas_cost != recorded.execution.gas_cost
            || native.context.private_state != recorded.execution.context.private_state
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
            || native.context.query.state != recorded.execution.context.query.state
            || native.context.query.effects != recorded.execution.context.query.effects
            || recorded.execution.private_transcript_outputs.len() != 3
            || *witness.calls.borrow() != ["secret", "secret", "board"]
        {
            return Err("original Coracle guess native/recorded mismatch".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "guess", recorded, position)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/guess.verifier"),
        )?))?;
        let generated = c::Contract::from(support::Witness::new(settings.clone()));
        let prepared = generated
            .recording()
            .guess_call(&observed, private.clone(), position)?
            .prepare(verifier, Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("original Coracle typed/direct call differs".into());
        }
        check_transaction(root, "guess", deploy, prepared, &mut rng, |state| {
            let guessed = slots::last_guess.inspect(state.data.get_ref())?;
            if state.data.get_ref() != &expected
                || !guessed.is_some
                || guessed.value != position
                || slots::state.inspect(state.data.get_ref())?
                    != if red {
                        types::State::blue_turn
                    } else {
                        types::State::red_turn
                    }
            {
                return Err("applied Coracle guess/turn differs".into());
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
            let repeated = c::recorded::guess(
                applied.circuit_context(private.clone()),
                &support::Witness::new(settings.clone()),
                position,
            )
            .err()
            .ok_or("repeated same-color move unexpectedly succeeded")?;
            let expected = if red {
                "Not Red's turn"
            } else {
                "Not Blue's turn"
            };
            if !repeated.to_string().contains(expected) {
                return Err("applied Coracle turn guard differs".into());
            }
            Ok(())
        })?;
        println!(
            "original Coracle.guess red={red} proved, verified and ledger-applied; applied-state repeated-turn rejected; explicit prior-state fixture and shared unbalanced smoke policy, no funded game setup claim"
        );
    }
    Ok(())
}
