// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-matched proofs for rich value boundaries and repeated owned values.
use super::*;
use compact_rust_effect_plan_rich_values_source_fixture::support::{Witness, reference};
use compact_rust_effect_plan_rich_values_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots, runtime as r,
};
fn validate_ir(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let expected: serde_json::Value = serde_json::from_str(RETAINED_IR)?;
    let actual: serde_json::Value = serde_json::from_slice(bytes)?;
    if expected != actual {
        return Err("rich value source/proof IR mismatch".into());
    }
    Ok(())
}
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    validate_ir(&std::fs::read(root.join("contract/compact-rust-ir.json"))?)?;
    for selected in [false, true] {
        run_case(root, selected)?;
    }
    Ok(())
}
fn run_case(root: &Path, selected: bool) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0524_0000 + u64::from(selected));
    let initial = c::initial_state(ConstructorContext::new(7_u64))?;
    let deploy = make_deploy(
        root,
        "run",
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
    let seed = Field::from(3_u64);
    let native_witness = Witness::default();
    let recorded_witness = Witness::default();
    let reference_witness = Witness::default();
    let native = c::run(
        observed.circuit_context(7_u64),
        &native_witness,
        selected,
        seed,
    )?;
    let recorded = c::recorded::run(
        observed.circuit_context(7_u64),
        &recorded_witness,
        selected,
        seed,
    )?;
    let reference = reference(
        observed.circuit_context(7_u64),
        selected,
        seed,
        &reference_witness,
    )?;
    let expected_result = Field::from(if selected { 30_u64 } else { 21_u64 });
    if *native_witness.calls.borrow() != *reference_witness.calls.borrow()
        || *recorded_witness.calls.borrow() != *reference_witness.calls.borrow()
        || native.result != expected_result
        || native.context.private_state != 8
        || native.private_transcript_outputs.len() != 1
    {
        return Err("rich source result/witness oracle mismatch".into());
    }
    for execution in [&recorded.execution, &reference.execution] {
        if execution.result != native.result
            || execution.context.query.state != native.context.query.state
            || execution.context.query.effects != native.context.query.effects
            || execution.gas_cost != native.gas_cost
            || execution.context.private_state != native.context.private_state
            || execution.private_transcript_outputs != native.private_transcript_outputs
        {
            return Err("rich source native/recorded/manual execution mismatch".into());
        }
    }
    if serde_json::to_value(recorded.public.verify_ops())?
        != serde_json::to_value(reference.public.verify_ops())?
    {
        return Err("rich source manual transcript mismatch".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    let stored = ledger_slots::stored.inspect(&expected_state)?;
    if stored.inner.amount != Field::from(if selected { 10_u64 } else { 7_u64 })
        || stored.inner.tag != r::FixedBytes::new(*b"gat")
        || ledger_slots::tag.inspect(&expected_state)? != stored.inner.tag
        || ledger_slots::phase.inspect(&expected_state)? != stored.phase
    {
        return Err("rich source storage oracle mismatch".into());
    }
    let input = AlignedValue::concat(&[AlignedValue::from(selected), AlignedValue::from(seed)]);
    let manual = check_generated_trace(root, "run", recorded, input)?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/run.verifier"),
    )?))?;
    let contract = c::Contract::from(Witness::default());
    let prepared = contract
        .recording()
        .run_call(&observed, 7_u64, selected, seed)?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("rich value observed/direct preparation mismatch".into());
    }
    crate::source_preimage::check(root, "run", &prepared, Some(selected))?;
    check_transaction(root, "run", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("rich value applied state mismatch".into());
        }
        Ok(())
    })?;
    println!(
        "rich values selected={selected}: rich values across witness, cells and helpers checked, proved, verified and ledger-applied"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_mismatched_source_before_keys() {
        validate_ir(RETAINED_IR.as_bytes()).unwrap();
        let mut altered: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        altered["stateful_circuits"][0]["name"] = "other".into();
        assert!(validate_ir(&serde_json::to_vec(&altered).unwrap()).is_err());
        assert!(
            validate_ir(
                compact_rust_effect_plan_stateful_calls_source_fixture::RETAINED_IR.as_bytes()
            )
            .is_err()
        );
    }
}
