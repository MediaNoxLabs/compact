// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound composite Set proof using default strict funded application.
use super::*;

const RETAINED_IR: &str =
    include_str!("../../../compact-rust-backend/tests/adt-set-vector-schema20-ir.json");
const CIRCUIT: &str = "test";

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(0x0607_5345_5456);
    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let deploy = make_deploy(
        root,
        CIRCUIT,
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
    let native = contract::test(observed.circuit_context(7_u64))?;
    let recorded = contract::recorded::test(observed.circuit_context(7_u64))?;
    let generated = contract::Contract::default();
    let call = generated.recording().test_call(&observed, 7_u64)?;
    // The source inserts two distinct vectors, removes one, then resets.
    // Its complete assertion sequence is proved, not merely the final state.
    let expected_state = native.context.query.state.get_ref().clone();
    let final_set = set_view_at_path::<FixedVector<Field, 2>, _>(&expected_state, &[0])?;
    if !final_set.is_empty() || final_set.size()?.value() != 0 {
        return Err("native vector Set sequence did not reset to empty".into());
    }
    if native.context.private_state != 7 || !native.private_transcript_outputs.is_empty() {
        return Err("vector Set sequence changed private data".into());
    }
    let direct_replay = recorded.public.initial().query(
        recorded.public.verify_ops(),
        None,
        &native.context.cost_model,
    )?;
    for actual in [&recorded, call.recorded()] {
        if actual.execution.context.query.state != native.context.query.state
            || actual.execution.context.query.effects != native.context.query.effects
            || actual.execution.gas_cost != native.gas_cost
            || actual.execution.context.private_state != 7
            || !actual.execution.private_transcript_outputs.is_empty()
            || actual.public.verify_ops() != recorded.public.verify_ops()
        {
            return Err("vector Set native/recorded/observed disagreement".into());
        }
        let replay = actual.public.initial().query(
            actual.public.verify_ops(),
            None,
            &native.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
            || replay.gas_cost != direct_replay.gas_cost
        {
            return Err("vector Set replay state/effects/gas disagreement".into());
        }
    }
    let direct = check_generated_trace(root, CIRCUIT, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/test.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from(())
        || prepared.output != AlignedValue::from(())
        || format!("{direct:?}") != format!("{prepared:?}")
    {
        return Err("vector Set observed preparation binding differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &expected_state, rng, 607)?;
    println!(
        "composite Set: full-source native/recorded/observed/replay agreement, proof and changed-binding rejection, strict funded ledger8 apply; constructor supplied, not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_complete_ir_is_refused() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_codec_control"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }

    #[test]
    fn absent_source_is_refused_before_state_or_keys() {
        let root = std::env::temp_dir().join(format!(
            "compact-set-vector-absent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!root.exists());
        assert_eq!(
            run(&root)
                .unwrap_err()
                .downcast_ref::<io::Error>()
                .unwrap()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.exists());
    }
}
