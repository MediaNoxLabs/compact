// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound ordinary snapshot proof; constructor state is supplied.
use super::*;
use compact_rust_literal_bytes_field_oracle_fixture::ledger_slots as slots;
use midnight_compact_runtime::{CompactError, recording::RecordingFrame};
const RETAINED_IR: &str =
    include_str!("../../../compact-rust-backend/tests/literal-bytes-field-schema20-ir.json");
const CIRCUIT: &str = "snapshot";

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(0x0606_534e_4150);
    let expected = Fr::from(16961_u64);
    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    let initial_state = initial.ledger_state.get_ref().clone();
    let deploy = make_deploy(root, CIRCUIT, initial_state.clone(), &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let native = contract::snapshot(observed.circuit_context(7_u64))?;
    let recorded = contract::recorded::snapshot(observed.circuit_context(7_u64))?;
    let generated = contract::Contract::default();
    let call = generated.recording().snapshot_call(&observed, 7_u64)?;
    let (frame, first) =
        slots::value.record_read(RecordingFrame::new(observed.circuit_context(7_u64)))?;
    if first != expected {
        return Err(CompactError::AssertionFailed("constructor cast".into()).into());
    }
    let (frame, result) = slots::value.record_read(frame)?;
    let manual = frame.finish(result);
    if native.result != expected || native.context.query.state.get_ref() != &initial_state {
        return Err("snapshot native value or unchanged constructor state differs".into());
    }
    let manual_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &native.context.cost_model,
    )?;
    for actual in [&recorded, call.recorded(), &manual] {
        if actual.execution.result != expected
            || actual.execution.context.private_state != 7
            || !actual.execution.private_transcript_outputs.is_empty()
            || actual.execution.context.query.state != native.context.query.state
            || actual.execution.context.query.effects != native.context.query.effects
            || actual.execution.gas_cost != native.gas_cost
            || actual.public.verify_ops() != manual.public.verify_ops()
        {
            return Err("ordinary snapshot execution/recording/observation/manual mismatch".into());
        }
        let replay = actual.public.initial().query(
            actual.public.verify_ops(),
            None,
            &native.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
            || replay.gas_cost != manual_replay.gas_cost
        {
            return Err("ordinary snapshot replay mismatch".into());
        }
    }
    if native.context.private_state != 7 || !native.private_transcript_outputs.is_empty() {
        return Err("snapshot native private state differs".into());
    }
    let direct = check_generated_trace(root, CIRCUIT, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/snapshot.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from(())
        || prepared.output != AlignedValue::from(expected)
        || format!("{direct:?}") != format!("{prepared:?}")
    {
        return Err("snapshot preparation binding differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &initial_state, rng, 606)?;
    println!(
        "ordinary snapshot: source-bound native/recorded/observed/manual agreement, proof verified and changed binding rejected, strict funded ledger8 application; constructor state supplied, not proved"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_complete_ir_refuses_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_snapshot_control"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn absent_source_refuses_before_constructor_or_keys() {
        let root = std::env::temp_dir().join(format!(
            "compact-snapshot-absent-{}-{}",
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
