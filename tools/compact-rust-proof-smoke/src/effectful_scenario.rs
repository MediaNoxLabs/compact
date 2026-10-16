// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Shared two-branch proof scenario; adapters select their actual generated crate.
//! Every run uses the existing trace preparation and official ledger proof gates.
use super::*;
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{CircuitContext, CircuitResult};

pub(super) trait ChooseScenario {
    fn initial_context() -> Result<CircuitContext<u64>, CompactError>;
    fn native(
        context: CircuitContext<u64>,
        next: Field,
    ) -> Result<CircuitResult<u64, Field>, CompactError>;
    fn recorded(
        context: CircuitContext<u64>,
        next: Field,
    ) -> Result<RecordedCircuitResult<u64, Field>, CompactError>;
    fn prepared(
        observed: &ObservedContractState,
        next: Field,
        verifier: VerifierKey,
    ) -> Result<ContractCallPrototype<DefaultDB>, Box<dyn Error>>;
}

pub(super) fn run<S: ChooseScenario>(root: &Path, label: &str) -> Result<(), Box<dyn Error>> {
    for (index, primed, next, expected, private) in
        [(0, false, 9_u64, 0_u64, 0_u64), (1, true, 8, 109, 1)]
    {
        let mut rng = StdRng::seed_from_u64(0x0182_0000 + index);
        let initial = S::initial_context()?;
        let state = if primed {
            S::native(initial, Fr::from(9_u64))?
                .context
                .query
                .state
                .get_ref()
                .clone()
        } else {
            initial.query.state.get_ref().clone()
        };
        let deploy = make_deploy(root, "choose", state, &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native = S::native(observed.circuit_context(0_u64), Fr::from(next))?;
        let recorded = S::recorded(observed.circuit_context(0_u64), Fr::from(next))?;
        if recorded.execution.result != Fr::from(expected)
            || recorded.execution.result != native.result
            || recorded.execution.context.private_state != private
            || recorded.execution.context.private_state != native.context.private_state
            || recorded.execution.private_transcript_outputs.len() != private as usize
            || recorded.execution.private_transcript_outputs != native.private_transcript_outputs
            || recorded.execution.gas_cost != native.gas_cost
            || recorded.execution.context.query.state != native.context.query.state
            || recorded.execution.context.query.effects != native.context.query.effects
        {
            return Err(
                format!("{label}: branch result, state, gas, effects or witness mismatch").into(),
            );
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, "choose", recorded, Fr::from(next))?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/choose.verifier"),
        )?))?;
        let prepared = S::prepared(&observed, Fr::from(next), verifier)?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err(format!("{label}: observed call differs from direct recording").into());
        }
        check_transaction(root, "choose", deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err(format!("{label}: branch ledger application changed state").into());
            }
            Ok(())
        })?;
        println!(
            "{label} branch {index} proved, verified, and ledger-applied under shared unbalanced smoke policy"
        );
    }
    Ok(())
}
