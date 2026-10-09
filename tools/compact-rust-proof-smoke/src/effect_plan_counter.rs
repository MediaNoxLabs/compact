// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-matched Counter effect qualification under the shared unbalanced policy.
use super::*;
use compact_rust_effect_plan_counter_source_fixture::ledger_contract as contract;

fn validate_ir(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let actual: serde_json::Value = serde_json::from_slice(bytes)?;
    let expected: serde_json::Value =
        serde_json::from_str(compact_rust_effect_plan_counter_source_fixture::RETAINED_IR)?;
    if actual != expected {
        return Err("Counter proof artifact IR differs from the generated source fixture".into());
    }
    Ok(())
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Input identity check before keys are read; not authenticated key provenance.
    validate_ir(&std::fs::read(root.join("contract/compact-rust-ir.json"))?)?;
    for (index, amount, remove, expected) in [(0_u64, 3_u16, 2_u16, 1_u64), (1, 10, u16::MAX, 0)] {
        let mut rng = StdRng::seed_from_u64(0x0512_0000 + index);
        let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
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
        let native = contract::adjust(observed.circuit_context(7_u64), amount, remove)?;
        let recorded = contract::recorded::adjust(observed.circuit_context(7_u64), amount, remove)?;
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
        let prepared = contract::Contract::default()
            .recording
            .adjust_call(&observed, 7_u64, amount, remove)?
            .prepare(verifier, Fr::from(0_u64))?;
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
            "Counter effect branch {index} proved, verified, and ledger-applied under shared unbalanced smoke policy"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_ir;

    #[test]
    fn rejects_artifact_mismatch_before_proof_work() {
        validate_ir(compact_rust_effect_plan_counter_source_fixture::RETAINED_IR.as_bytes())
            .unwrap();
        let mut changed: serde_json::Value =
            serde_json::from_str(compact_rust_effect_plan_counter_source_fixture::RETAINED_IR)
                .unwrap();
        changed["stateful_circuits"][0]["name"] = "different".into();
        assert!(validate_ir(&serde_json::to_vec(&changed).unwrap()).is_err());
        assert!(
            validate_ir(compact_rust_effect_plan_wide_source_fixture::RETAINED_IR.as_bytes())
                .is_err()
        );
        assert!(validate_ir(b"{}").is_err());
    }
}
