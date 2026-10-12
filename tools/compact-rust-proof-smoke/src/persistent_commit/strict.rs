// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original declared-Field commitment with source binding and strict funded application.
use super::*;
use compact_rust_call_arg_declared_type_fixture::ledger_slots as slots;
use midnight_compact_runtime::recording::RecordingFrame;

const RETAINED_IR: &str =
    include_str!("../../../compact-rust-backend/tests/call-arg-declared-type-schema20-ir.json");
const CIRCUIT: &str = "commitSmall";

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(0x0611_434f_4d4d);
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
    let native = contract::commitSmall(observed.circuit_context(7_u64))?;
    let recorded = contract::recorded::commitSmall(observed.circuit_context(7_u64))?;
    let call = contract::recorded::Contract.commitSmall_call(&observed, 7_u64)?;
    let frame = RecordingFrame::new(observed.circuit_context(7_u64));
    let (frame, opening) = slots::opening.record_read(frame)?;
    // The input is explicitly Field, preserving the source formal type rather than
    // accidentally hashing an inferred machine integer's representation.
    let expected = midnight_compact_runtime::persistent_commit(Field::from(5_u64), opening);
    let manual = slots::commitCell.record_write(frame, expected)?.finish(());
    if native.context.private_state != 7 || !native.private_transcript_outputs.is_empty() {
        return Err("Field commitment changed private data".into());
    }
    let manual_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &native.context.cost_model,
    )?;
    for actual in [&recorded, call.recorded(), &manual] {
        if actual.execution.context.query.state != native.context.query.state
            || actual.execution.context.query.effects != native.context.query.effects
            || actual.execution.gas_cost != native.gas_cost
            || actual.execution.context.private_state != 7
            || !actual.execution.private_transcript_outputs.is_empty()
            || actual.public.verify_ops() != manual.public.verify_ops()
        {
            return Err("Field commitment native/recorded/observed/manual disagreement".into());
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
            return Err("Field commitment replay state/effects/gas disagreement".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::commitCell.inspect(&expected_state)? != expected
        || slots::opening.inspect(&expected_state)? != opening
    {
        return Err("Field commitment or constructor opening differs".into());
    }
    let direct = check_generated_trace(root, CIRCUIT, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/commitSmall.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from(())
        || prepared.output != AlignedValue::from(())
        || format!("{direct:?}") != format!("{prepared:?}")
    {
        return Err("Field commitment observed preparation binding differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &expected_state, rng, 611)?;
    println!(
        "Field commitment: exact declared input, native/recorded/observed/manual replay, proof and changed-binding rejection, strict funded ledger8 apply; constructor opening supplied, not proved"
    );
    Ok(())
}

#[cfg(test)]
#[path = "strict_tests.rs"]
mod tests;
