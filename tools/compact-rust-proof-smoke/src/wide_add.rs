// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! A genuine carry above u128 through checked recording and a funded ledger8 proof.
use super::*;
use compact_rust_effect_plan_wide_add_source_fixture::{
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
    let name = "add128";
    let mut rng = StdRng::seed_from_u64(0x0564_0000);
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
        root.join("keys/add128.verifier"),
    )?))?;
    // Independently specified carry: (2^128 - 1) + 1 = 2^128.
    let left = BoundedUint::<{ u128::MAX }>::new(u128::MAX)?;
    let right = BoundedUint::<{ u128::MAX }>::new(1)?;
    let mut bytes = [0; 17];
    bytes[16] = 1;
    let expected = types::Wide {
        value: runtime::WideUint::from_le_bytes(&bytes)?,
    };
    let native = c::add128(observed.circuit_context(5), left, right)?;
    let recorded = c::recorded::add128(observed.circuit_context(5), left, right)?;
    let call = c::recorded::Contract.add128_call(&observed, 5, left, right)?;
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
            return Err("wide addition native/recorded/observed/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
        {
            return Err("wide addition transcript replay differs".into());
        }
    }
    if slots::calls.inspect(native.context.query.state.get_ref())? != 1 {
        return Err("wide addition Counter differs from source".into());
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
    let small = Alignment::singleton(AlignmentAtom::Bytes { length: 16 });
    if atoms(&prepared.input) != [vec![255; 16], vec![1]]
        || atoms(&prepared.output) != [bytes.to_vec()]
        || prepared.input.alignment != Alignment::concat([&small, &small])
        || prepared.output.alignment != Alignment::singleton(AlignmentAtom::Bytes { length: 17 })
    {
        return Err("wide addition literal atoms or declared alignments differ".into());
    }
    let direct = check_generated_trace(root, name, recorded, (left, right))?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("wide addition direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 564)?;
    println!(
        "wide addition: actual 128-bit carry, exact atoms/alignments, replay, proof binding refusal and funded default-strict ledger-8 application passed"
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
    fn missing_or_changed_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-wide-add-missing-ir-{}-{}",
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
        std::fs::create_dir_all(root.join("contract")).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_guard_change"] = serde_json::json!(true);
        let path = root.join("contract/compact-rust-ir.json");
        let bytes = serde_json::to_vec(&changed).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let error = run(&root).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Proof artifact IR differs from the generated source fixture"
        );
        assert!(!root.join("keys").exists());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_dir_all(root).unwrap();
    }
}
