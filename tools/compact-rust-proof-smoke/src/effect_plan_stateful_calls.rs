// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Two finite source-matched proofs for ordered, isolated stateful helper calls.
use super::*;
use compact_rust_effect_plan_stateful_calls_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots, runtime as r,
};
use std::cell::RefCell;

#[derive(Default)]
struct Witness(RefCell<Vec<(Field, u64)>>);
impl c::Witnesses<u64> for Witness {
    fn nextValue(
        &self,
        context: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        self.0.borrow_mut().push((seed, *context.private_state));
        (
            *context.private_state + 1,
            seed + Field::from(*context.private_state),
        )
    }
}
fn validate_ir(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let expected: serde_json::Value = serde_json::from_str(RETAINED_IR)?;
    let actual: serde_json::Value = serde_json::from_slice(bytes)?;
    if expected != actual {
        return Err("stateful helper source/proof IR mismatch".into());
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
    let mut rng = StdRng::seed_from_u64(0x0520_0000 + u64::from(selected));
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
    let first = Field::from(3_u64);
    let second = Field::from(5_u64);
    let native_witness = Witness::default();
    let recorded_witness = Witness::default();
    let native = c::run(
        observed.circuit_context(7_u64),
        &native_witness,
        selected,
        first,
        second,
        true,
    )?;
    let recorded = c::recorded::run(
        observed.circuit_context(7_u64),
        &recorded_witness,
        selected,
        first,
        second,
        true,
    )?;
    let last = if selected { 355_u64 } else { 357_u64 };
    let seeds = [3_u64, 5, 23, 67, 159, last - 12]
        .into_iter()
        .enumerate()
        .map(|(i, seed)| (Field::from(seed), 7 + i as u64))
        .collect::<Vec<_>>();
    let outputs = [10_u64, 13, 32, 77, 170, last]
        .map(|n| AlignedValue::from(Field::from(n)))
        .to_vec();
    if *native_witness.0.borrow() != seeds
        || *recorded_witness.0.borrow() != seeds
        || native.result != Field::from(last + 3)
        || recorded.execution.result != native.result
        || native.context.query.state != recorded.execution.context.query.state
        || native.context.query.effects != recorded.execution.context.query.effects
        || native.gas_cost != recorded.execution.gas_cost
        || native.context.private_state != 13
        || recorded.execution.context.private_state != 13
        || native.private_transcript_outputs != outputs
        || recorded.execution.private_transcript_outputs != outputs
    {
        return Err("stateful helper source native/recorded/witness oracle mismatch".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if ledger_slots::stored.inspect(&expected_state)? != Field::from(last)
        || ledger_slots::receipt.inspect(&expected_state)? != Field::from(last)
    {
        return Err("stateful helper storage oracle mismatch".into());
    }
    let input = AlignedValue::concat(&[
        AlignedValue::from(selected),
        AlignedValue::from(first),
        AlignedValue::from(second),
        AlignedValue::from(true),
    ]);
    let manual = check_generated_trace(root, "run", recorded, input)?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/run.verifier"),
    )?))?;
    let contract = c::Contract::from(Witness::default());
    let prepared = contract
        .recording()
        .run_call(&observed, 7_u64, selected, first, second, true)?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("stateful helper observed/direct preparation mismatch".into());
    }
    crate::source_preimage::check(root, "run", &prepared, Some(selected))?;
    check_transaction(root, "run", deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("stateful helper applied state mismatch".into());
        }
        Ok(())
    })?;
    println!(
        "stateful helpers selected={selected}: ordered six-witness call checked, proved, verified and ledger-applied"
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
            validate_ir(compact_rust_effect_plan_jubjub_source_fixture::RETAINED_IR.as_bytes())
                .is_err()
        );
    }
}
