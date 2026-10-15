// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! The unchanged opaque_cell_inputs source's Unit-return setBytes call.
use super::*;
use compact_rust_opaque_cell_inputs_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::recording::RecordingFrame;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(593);
    let original_envelope = types::Envelope {
        payload: types::Payload {
            text: "nested".into(),
            bytes: vec![2].into(),
        },
        marker: r::Field::from(23_u64),
    };
    let initial = c::initial_state(
        ConstructorContext::new(7_u64),
        "initial".into(),
        vec![1].into(),
        original_envelope.clone(),
    )?;
    let deploy = make_deploy(
        root,
        "setBytes",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let observed = pure_field_arguments::observed_state(&deploy);
    let value: r::OpaqueBytes = vec![1, 0, 2].into();
    let native = c::setBytes(observed.circuit_context(7), value.clone())?;
    let recorded = c::recorded::setBytes(observed.circuit_context(7), value.clone())?;
    let manual = slots::blob
        .record_write(
            RecordingFrame::new(observed.circuit_context(7)),
            value.clone(),
        )?
        .finish(());
    let call = c::recorded::Contract.setBytes_call(&observed, 7, value.clone())?;
    if native.context.private_state != 7 || !native.private_transcript_outputs.is_empty() {
        return Err("setBytes native private state/output differs".into());
    }
    let expected_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [&recorded, call.recorded(), &manual] {
        if candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.execution.context.private_state != 7
            || !candidate.execution.private_transcript_outputs.is_empty()
            || candidate.public.verify_ops() != manual.public.verify_ops()
        {
            return Err("setBytes native/recorded/observed/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
            || replay.gas_cost != expected_replay.gas_cost
        {
            return Err("setBytes verifier replay differs".into());
        }
    }
    let expected = native.context.query.state.get_ref().clone();
    if slots::blob.inspect(&expected)? != value
        || slots::text.inspect(&expected)? != r::OpaqueString::from("initial")
        || slots::envelope.inspect(&expected)? != original_envelope
    {
        return Err("setBytes changed unexpected Cell values".into());
    }
    let verifier = pure_field_arguments::verifier(root, "setBytes")?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from(value.clone())
        || prepared.output != AlignedValue::from(())
    {
        return Err("setBytes declared input/Unit output differs".into());
    }
    let direct = check_generated_trace(root, "setBytes", recorded, value)?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("setBytes direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, "setBytes", &prepared, &verifier)?;
    opaque_public::apply_funded(root, "setBytes", prepared, deploy, &expected, rng, 593)?;
    println!(
        "Unit setBytes: exact source/declared input, native/recorded/observed/manual/replay, verified proof and changed-binding refusal, default-strict funded application; constructor state supplied, deployment not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_opaque_cell_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-opaque-cell-absent-{}-{}",
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
