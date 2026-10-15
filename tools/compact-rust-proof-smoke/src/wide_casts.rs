// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! A genuine wide-to-wide narrowing above u128 through a funded ledger8 proof.
use super::*;
use compact_rust_effect_plan_wide_casts_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime::{
    self as runtime,
    fab::{Alignment, AlignmentAtom},
    recording::RecordingFrame,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Full source identity before keys/state/funding; this is not key authentication.
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "narrow129";
    let mut rng = StdRng::seed_from_u64(0x0562_0000);
    let initial = c::initial_state(ConstructorContext::new(5_u64))?;
    let deploy = make_deploy(root, name, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/narrow129.verifier"),
    )?))?;
    let mut bytes = [0; 17];
    bytes[0] = 37;
    bytes[16] = 1; // 2^128 + 37, genuinely above the small carrier.
    let input = runtime::WideUint::<{ (1_u128 << 120) - 1 }, { u128::MAX }>::from_le_bytes(&bytes)?;
    let expected = types::Wide {
        value: runtime::WideUint::from_le_bytes(&bytes)?,
    };
    let native = c::narrow129(observed.circuit_context(5), input)?;
    let recorded = c::recorded::narrow129(observed.circuit_context(5), input)?;
    let call = c::recorded::Contract.narrow129_call(&observed, 5, input)?;
    let frame =
        slots::calls.record_increment(RecordingFrame::new(observed.circuit_context(5)), 1)?;
    let manual = frame.finish(expected.clone());
    for candidate in [&recorded, call.recorded(), &manual] {
        if native.result != expected
            || candidate.execution.result != expected
            || native.context.query.state != candidate.execution.context.query.state
            || native.context.query.effects != candidate.execution.context.query.effects
            || native.gas_cost != candidate.execution.gas_cost
            || native.context.private_state != 5
            || candidate.execution.context.private_state != 5
            || !native.private_transcript_outputs.is_empty()
            || !candidate.execution.private_transcript_outputs.is_empty()
            || candidate.public.verify_ops() != manual.public.verify_ops()
            || candidate.public.verify_ops().is_empty()
        {
            return Err("wide cast native/recorded/observed/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
        {
            return Err("wide cast transcript replay differs".into());
        }
    }
    if slots::calls.inspect(native.context.query.state.get_ref())? != 1 {
        return Err("wide cast Counter differs from source".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    let atoms = |value: &AlignedValue| {
        value
            .value
            .0
            .iter()
            .map(|atom| atom.0.clone())
            .collect::<Vec<_>>()
    };
    if atoms(&prepared.input) != [bytes.to_vec()]
        || atoms(&prepared.output) != [bytes.to_vec()]
        || prepared.input.alignment != Alignment::singleton(AlignmentAtom::Bytes { length: 31 })
        || prepared.output.alignment != Alignment::singleton(AlignmentAtom::Bytes { length: 17 })
    {
        return Err("wide cast literal atoms or declared alignments differ".into());
    }
    let direct = check_generated_trace(root, name, recorded, input)?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("wide cast direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 562)?;
    println!(
        "wide casts: above-u128 value, exact atoms/alignments, replay, proof binding refusal and funded default-strict ledger-8 application passed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_source_accepts_and_changed_source_refuses() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_guard_change"] = serde_json::json!(true);
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn missing_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-wide-casts-missing-ir-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!root.exists());
        let error = run(&root).unwrap_err();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.exists());
    }
}
