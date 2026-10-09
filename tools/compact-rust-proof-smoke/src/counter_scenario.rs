// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Shared proof scenario for the exact mixed-storage and Counter-only sources.
use super::*;
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{CircuitContext, CircuitResult, ConstructorResult};

type Amount = BoundedUint<65535>;

pub(super) trait CounterScenario {
    fn initial_state() -> Result<ConstructorResult<u64>, CompactError>;
    fn native(
        context: CircuitContext<u64>,
        amount: Amount,
        remove: Amount,
    ) -> Result<CircuitResult<u64, Field>, CompactError>;
    fn recorded(
        context: CircuitContext<u64>,
        amount: Amount,
        remove: Amount,
    ) -> Result<RecordedCircuitResult<u64, Field>, CompactError>;
    fn prepared(
        observed: &ObservedContractState,
        amount: Amount,
        remove: Amount,
        verifier: VerifierKey,
    ) -> Result<ContractCallPrototype<DefaultDB>, Box<dyn Error>>;
}

pub(super) fn validate_ir(actual: &[u8], expected: &str) -> Result<(), Box<dyn Error>> {
    let actual: serde_json::Value = serde_json::from_slice(actual)?;
    let expected: serde_json::Value = serde_json::from_str(expected)?;
    if actual != expected {
        return Err("Counter proof artifact IR differs from the generated source fixture".into());
    }
    Ok(())
}

pub(super) fn run<S: CounterScenario>(
    root: &Path,
    expected_ir: &str,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    // Input pairing check before key reads; not authenticated key provenance.
    validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        expected_ir,
    )?;
    for (index, amount, remove, expected) in [(0_u64, 3_u16, 2_u16, 1_u64), (1, 10, u16::MAX, 0)] {
        let mut rng = StdRng::seed_from_u64(0x0512_0000 + index);
        let initial = S::initial_state()?;
        let deploy = make_deploy(
            root,
            "adjust",
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
        let amount = BoundedUint::<65535>::new(amount.into())?;
        let remove = BoundedUint::<65535>::new(remove.into())?;
        let native = S::native(observed.circuit_context(7_u64), amount, remove)?;
        let recorded = S::recorded(observed.circuit_context(7_u64), amount, remove)?;
        if native.result != Field::from(expected)
            || recorded.execution.result != native.result
            || native.context.query.state != recorded.execution.context.query.state
            || native.context.query.effects != recorded.execution.context.query.effects
            || native.gas_cost != recorded.execution.gas_cost
            || native.context.private_state != 7
            || recorded.execution.context.private_state != 7
            || !native.private_transcript_outputs.is_empty()
            || !recorded.execution.private_transcript_outputs.is_empty()
        {
            return Err("Counter native/recorded scenario mismatch".into());
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let input = AlignedValue::concat(&[AlignedValue::from(amount), AlignedValue::from(remove)]);
        let manual = check_generated_trace(root, "adjust", recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/adjust.verifier"),
        )?))?;
        let prepared = S::prepared(&observed, amount, remove, verifier)?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("Counter observed call differs from direct recording".into());
        }
        check_transaction(root, "adjust", deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err("Counter ledger application changed the expected state".into());
            }
            Ok(())
        })?;
        println!(
            "{label} branch {index} proved, verified, and ledger-applied under shared unbalanced smoke policy"
        );
    }
    Ok(())
}
