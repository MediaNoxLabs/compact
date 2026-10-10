// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! One full-source owned-callback proof, with canonical public inputs and strict funding.
use super::*;
use compact_rust_effect_plan_witness_inputs_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, support,
};
use midnight_compact_runtime as runtime;

const TEXT: &str = "é文🦀\0tail";
const BYTES: &[u8] = &[0, 1, 0, 255];

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Exact full input before keys/state/funding; not key provenance authentication.
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "run";
    let mut rng = StdRng::seed_from_u64(0x0557_0000);
    let initial = c::initial_state(ConstructorContext::new(7_u64))?;
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
        root.join("keys/run.verifier"),
    )?))?;
    let native_witness = support::Witness::default();
    let recorded_witness = support::Witness::default();
    let observed_witness = support::Witness::default();
    let manual_witness = support::Witness::default();
    let native = c::run(
        observed.circuit_context(7),
        &native_witness,
        TEXT.into(),
        BYTES.to_vec().into(),
        true,
    )?;
    let recorded = c::recorded::run(
        observed.circuit_context(7),
        &recorded_witness,
        TEXT.into(),
        BYTES.to_vec().into(),
        true,
    )?;
    let generated = c::Contract::from(observed_witness.clone());
    let call =
        generated
            .recording()
            .run_call(&observed, 7, TEXT.into(), BYTES.to_vec().into(), true)?;
    let manual = support::manual(
        observed.circuit_context(7),
        TEXT,
        BYTES,
        true,
        &manual_witness,
    )?;
    let expected_events = [7, 8].map(|private| support::Event {
        private,
        counter: 6,
        arguments: support::arguments(TEXT, BYTES),
    });
    for witness in [
        &native_witness,
        &recorded_witness,
        &observed_witness,
        &manual_witness,
    ] {
        // The support callback mutates all its owned copies after capture. Both
        // invocations must still receive these exact original raw arguments.
        if witness.events.borrow().as_slice() != expected_events {
            return Err("owned witness arguments/order/private transition differ".into());
        }
    }
    if !native.result.value || slots::calls.inspect(native.context.query.state.get_ref())? != 6 {
        return Err("owned witness result or counter differs from source".into());
    }
    for candidate in [&recorded, call.recorded(), &manual] {
        if native.result != candidate.execution.result
            || native.context.query.state != candidate.execution.context.query.state
            || native.context.query.effects != candidate.execution.context.query.effects
            || native.gas_cost != candidate.execution.gas_cost
            || native.context.private_state != 9
            || candidate.execution.context.private_state != 9
            || native.private_transcript_outputs != candidate.execution.private_transcript_outputs
            || candidate.execution.private_transcript_outputs != vec![AlignedValue::from(true); 2]
            || candidate.public.verify_ops() != manual.public.verify_ops()
            || candidate.public.verify_ops().is_empty()
        {
            return Err("owned witness native/recorded/manual execution differs".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    let prepared = call.prepare(verifier.clone(), Fr::from(0u64))?;
    require_atoms(&prepared.input, &[TEXT.as_bytes(), BYTES, &[1]])?;
    require_atoms(&prepared.output, &[&[1]])?;
    let input = AlignedValue::concat(&[
        AlignedValue::from(runtime::OpaqueString::from(TEXT)),
        AlignedValue::from(runtime::OpaqueBytes::from(BYTES.to_vec())),
        AlignedValue::from(true),
    ]);
    let direct = check_generated_trace(root, name, recorded, input)?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("owned witness manual/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 557)?;
    println!(
        "owned witness inputs: exact raw callback arguments and reused ownership, two private Boolean outputs, proof binding rejection and funded default-strict application passed"
    );
    Ok(())
}

fn require_atoms(value: &AlignedValue, expected: &[&[u8]]) -> Result<(), Box<dyn Error>> {
    let actual = value
        .value
        .0
        .iter()
        .map(|atom| atom.0.as_slice())
        .collect::<Vec<_>>();
    if actual != expected {
        return Err("owned witness public atoms differ from literal canonical bytes".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_input_change_is_rejected() {
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["contractName"] = serde_json::json!("independent-owned-input-change");
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR,)
                .unwrap_err()
                .to_string()
                .contains("IR differs")
        );
    }

    #[test]
    fn missing_input_refuses_before_keys_and_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-owned-witness-missing-ir-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
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
