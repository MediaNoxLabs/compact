// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact source-bound point composition proofs; finite qualification, not grammar coverage.
use super::*;
use compact_rust_effect_plan_jubjub_source_fixture as source;
use source::{ledger_contract as contract, ledger_slots, runtime as r};

fn validate_ir(bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let actual: serde_json::Value = serde_json::from_slice(bytes)?;
    let expected: serde_json::Value = serde_json::from_str(source::RETAINED_IR)?;
    if actual != expected {
        return Err(
            "Jubjub proof artifact IR differs from the exact generated source fixture".into(),
        );
    }
    Ok(())
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Input pairing before key reads; this does not authenticate key provenance.
    validate_ir(&std::fs::read(root.join("contract/compact-rust-ir.json"))?)?;
    for (index, branch) in [(0, None), (1, Some(false)), (2, Some(true))] {
        run_case(root, index, branch)?;
    }
    Ok(())
}

fn run_case(root: &Path, index: u64, branch: Option<bool>) -> Result<(), Box<dyn Error>> {
    let circuit = if branch.is_some() {
        "read_branch"
    } else {
        "compose"
    };
    let mut rng = StdRng::seed_from_u64(0x0517_0000 + index);
    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let deploy = make_deploy(
        root,
        circuit,
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
    let point = r::ec_mul_generator(Field::from(7_u64))?;
    let scalar = Field::from(3_u64);
    let (native, recorded, input, expected, first, second) = if let Some(selected) = branch {
        let expected = if selected {
            r::ec_add(point, r::ec_mul_generator(scalar)?)
        } else {
            r::ec_mul(point, scalar)?
        };
        (
            contract::read_branch(observed.circuit_context(7_u64), selected, point, scalar)?,
            contract::recorded::read_branch(
                observed.circuit_context(7_u64),
                selected,
                point,
                scalar,
            )?,
            AlignedValue::concat(&[
                AlignedValue::from(selected),
                AlignedValue::from(point),
                AlignedValue::from(scalar),
            ]),
            expected,
            point,
            expected,
        )
    } else {
        let stored = r::ec_mul(r::ec_add(r::ec_mul_generator(scalar)?, point), scalar)?;
        (
            contract::compose(observed.circuit_context(7_u64), point, scalar)?,
            contract::recorded::compose(observed.circuit_context(7_u64), point, scalar)?,
            AlignedValue::concat(&[AlignedValue::from(point), AlignedValue::from(scalar)]),
            r::ec_add(stored, r::ec_mul_generator(Field::from(1_u64))?),
            stored,
            r::JubjubPoint::identity(),
        )
    };
    if native.result != expected
        || recorded.execution.result != native.result
        || native.context.query.state != recorded.execution.context.query.state
        || native.context.query.effects != recorded.execution.context.query.effects
        || native.gas_cost != recorded.execution.gas_cost
        || native.context.private_state != 7
        || recorded.execution.context.private_state != 7
        || !native.private_transcript_outputs.is_empty()
        || !recorded.execution.private_transcript_outputs.is_empty()
    {
        return Err("Jubjub source native/recorded execution mismatch".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if ledger_slots::first.inspect(&expected_state)? != first
        || ledger_slots::second.inspect(&expected_state)? != second
    {
        return Err("Jubjub source result/storage semantics changed".into());
    }
    let manual = check_generated_trace(root, circuit, recorded, input)?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{circuit}.verifier")),
    )?))?;
    let contract = contract::Contract::default();
    let prepared = if let Some(selected) = branch {
        contract
            .recording
            .read_branch_call(&observed, 7_u64, selected, point, scalar)?
            .prepare(verifier, Fr::from(0_u64))?
    } else {
        contract
            .recording
            .compose_call(&observed, 7_u64, point, scalar)?
            .prepare(verifier, Fr::from(0_u64))?
    };
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("Jubjub observed call differs from direct recording".into());
    }
    check_transaction(root, circuit, deploy, prepared, &mut rng, |state| {
        if state.data.get_ref() != &expected_state {
            return Err("Jubjub ledger application changed expected state".into());
        }
        Ok(())
    })?;
    println!(
        "Jubjub {circuit} case {index} proved, verified and ledger-applied under shared unbalanced smoke policy"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_mismatched_source_before_reading_keys() {
        validate_ir(source::RETAINED_IR.as_bytes()).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(source::RETAINED_IR).unwrap();
        changed["stateful_circuits"][0]["name"] = "different".into();
        assert!(validate_ir(&serde_json::to_vec(&changed).unwrap()).is_err());
        assert!(
            validate_ir(include_bytes!(
                "../../compact-rust-backend/tests/jubjub-scalar-cell-schema20-ir.json"
            ))
            .is_err()
        );
        assert!(validate_ir(b"{}").is_err());
    }
}
