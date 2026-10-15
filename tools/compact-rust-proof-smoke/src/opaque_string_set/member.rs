// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! One source-bound membership proof; constructor and native insertion are supplied.
use super::*;
use compact_rust_opaque_string_set_oracle_fixture::ledger_slots as slots;
use midnight_compact_runtime::recording::RecordingFrame;

const RETAINED_IR: &str =
    include_str!("../../../compact-rust-backend/tests/opaque-string-set-schema20-ir.json");
const CIRCUIT: &str = "hasName";

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Whole source pairing precedes constructor state, key reads and proof work.
    // The external material receipt separately authenticates the generated keys.
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(0x0603_5345_544d);
    let key = OpaqueString::from("registry-key");
    let initial = contract::initial_state(ConstructorContext::new(7_u64))?;
    if slots::names
        .inspect(initial.ledger_state.get_ref())?
        .member(key.clone())
    {
        return Err("membership constructor unexpectedly contains the selected key".into());
    }
    // Supply the source's native insertion; neither it nor deployment is proved.
    let added = contract::addName(
        initial.into_circuit_context(ContractAddress::default()),
        key.clone(),
    )?;
    if added.context.private_state != 7 || !added.private_transcript_outputs.is_empty() {
        return Err("membership supplied seed changed private state or outputs".into());
    }
    let deploy = make_deploy(
        root,
        CIRCUIT,
        added.context.query.state.get_ref().clone(),
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
    let generated = contract::Contract::default();
    let native = contract::hasName(observed.circuit_context(7_u64), key.clone())?;
    let recorded = generated
        .recording()
        .hasName(observed.circuit_context(7_u64), key.clone())?;
    let call = generated
        .recording()
        .hasName_call(&observed, 7_u64, key.clone())?;
    let (frame, member) = slots::names.record_member(
        RecordingFrame::new(observed.circuit_context(7_u64)),
        key.clone(),
    )?;
    let manual = frame.finish(member);
    if !native.result
        || native.context.private_state != 7
        || !native.private_transcript_outputs.is_empty()
        || native.context.query.state != added.context.query.state
    {
        return Err("membership native result/state/private binding differs".into());
    }
    let manual_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [&recorded, call.recorded(), &manual] {
        if !candidate.execution.result
            || candidate.execution.context.private_state != 7
            || !candidate.execution.private_transcript_outputs.is_empty()
            || candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.public.verify_ops() != manual.public.verify_ops()
        {
            return Err("membership native/recorded/observed/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
            || replay.gas_cost != manual_replay.gas_cost
        {
            return Err("membership verifier replay differs from independent manual replay".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if !slots::names.inspect(&expected_state)?.member(key.clone()) {
        return Err("membership lost its supplied key".into());
    }
    let input = AlignedValue::from(key);
    let direct = check_generated_trace(root, CIRCUIT, recorded, input.clone())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/hasName.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != input
        || prepared.output != AlignedValue::from(true)
        || format!("{direct:?}") != format!("{prepared:?}")
    {
        return Err("membership declared FAB or observed/direct preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &expected_state, rng, 603)?;
    println!(
        "hasName: source-bound four-lane execution/replay, verified proof with changed-binding refusal and default-strict funded ledger-8 application; constructor and native insertion supplied, neither proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn membership_source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_membership_source_control"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn membership_absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-membership-proof-absent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
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
