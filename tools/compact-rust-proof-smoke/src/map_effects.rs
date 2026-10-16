// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! One selected source-bound Map insert/lookup call with actual funded strict application.
use super::*;
use compact_rust_effect_plan_maps_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime::{self as runtime, recording::RecordingFrame};

#[derive(Clone)]
struct UnusedWitness;
impl c::TryWitnesses<u64> for UnusedWitness {
    fn pick_key(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), runtime::CompactError> {
        Err(runtime::CompactError::AssertionFailed(
            "put must not call pick_key".into(),
        ))
    }
    fn pick_value(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), runtime::CompactError> {
        Err(runtime::CompactError::AssertionFailed(
            "put must not call pick_value".into(),
        ))
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Exact whole-source identity precedes keys, state construction and funding.
    // This is input pairing, not an assertion that IR authenticates proving keys.
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "put";
    let mut rng = StdRng::seed_from_u64(0x0579_0000);
    let seed: runtime::OpaqueString = "seed".into();
    let key: runtime::OpaqueString = "λ\0key".into();
    let value = runtime::Field::from(11_u64);
    let initial = c::initial_state(ConstructorContext::new(5_u64), seed.clone())?;
    // Initial Map seed is actual generated constructor data, not fabricated scalar state.
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
        root.join("keys/put.verifier"),
    )?))?;
    let native = c::put(observed.circuit_context(5), key.clone(), value)?;
    let recorded = c::recorded::put(observed.circuit_context(5), key.clone(), value)?;
    let generated = c::Contract::from(UnusedWitness);
    let call = generated
        .recording()
        .put_call(&observed, 5, key.clone(), value)?;
    let frame = slots::table.record_insert(
        RecordingFrame::new(observed.circuit_context(5)),
        key.clone(),
        value,
    )?;
    let frame = slots::calls.record_increment(frame, 1)?;
    let (frame, found) = slots::table.record_lookup(frame, key.clone())?;
    let manual = frame.finish(types::Found { value: found });
    if found != value || native.result != (types::Found { value }) {
        return Err("Map put returned a value different from literal11".into());
    }
    for candidate in [&recorded, call.recorded(), &manual] {
        if candidate.execution.result != native.result
            || candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.execution.context.private_state != 5
            || native.context.private_state != 5
            || !candidate.execution.private_transcript_outputs.is_empty()
            || !native.private_transcript_outputs.is_empty()
            || candidate.public.verify_ops() != manual.public.verify_ops()
            || candidate.public.verify_ops().is_empty()
        {
            return Err("Map native/recorded/observed/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
        {
            return Err("Map transcript replay differs".into());
        }
    }
    if slots::calls.inspect(native.context.query.state.get_ref())? != 1 {
        return Err("Map put Counter increment differs".into());
    }
    // Reading the other constructor entry checks that insertion did not replace the Map.
    let seed_read = slots::table
        .inspect(native.context.query.state.get_ref())?
        .lookup(seed)?;
    if seed_read != runtime::Field::from(7_u64) {
        return Err("Map put lost the original constructor seed".into());
    }
    let expected_state = native.context.query.state.get_ref().clone();
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from((key.clone(), value))
        || prepared.output != AlignedValue::from((value,))
    {
        return Err("Map put prepared public values or alignments differ".into());
    }
    let raw_atoms = prepared
        .input
        .value
        .0
        .iter()
        .map(|a| a.0.clone())
        .collect::<Vec<_>>();
    if raw_atoms != vec!["λ\0key".as_bytes().to_vec(), vec![11]] {
        return Err("Map put did not preserve canonical UTF8/internal-NUL key atoms".into());
    }
    let direct = check_generated_trace(root, name, recorded, (key, value))?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("Map direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 579)?;
    println!(
        "Map put: generated seeded state, exact UTF8/input/output bindings, insert->Counter->lookup, native/recorded/manual replay, verified proof+changed-binding refusal and default-strict Dust-funded call application passed; deployment itself was not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_source_change_refuses_before_proof_work() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_map_guard"] = serde_json::json!(true);
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn missing_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-map-proof-missing-ir-{}-{}",
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
