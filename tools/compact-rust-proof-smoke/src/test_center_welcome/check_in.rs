// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Selected original check-in proof, with supplied constructor state.
use super::*;
use compact_rust_test_center_welcome_fixture::ledger_slots as slots;
use midnight_compact_runtime::recording::RecordingFrame;

#[derive(Clone, Default)]
struct Witness(Rc<RefCell<Vec<(u64, String)>>>);
impl contract::Witnesses<u64> for Witness {
    fn local_sk(
        &self,
        _: WitnessContext<'_, u64, contract::LedgerView<'_>>,
    ) -> (
        u64,
        compact_rust_test_center_welcome_fixture::types::MaybeCompact1,
    ) {
        panic!("selected check-in must not request the organizer secret")
    }
    fn set_local_id(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
        participant: OpaqueString,
    ) -> (u64, ()) {
        self.0
            .borrow_mut()
            .push((*context.private_state, participant.0));
        (*context.private_state + 1, ())
    }
}

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    const CIRCUIT: &str = "check_in";
    let mut rng = StdRng::seed_from_u64(0x0605_4348_4543_4b49);
    let participant = OpaqueString::from("alice");
    let participants = FixedVector::new(std::array::from_fn(|index| Maybe {
        is_some: index == 7,
        value: OpaqueString::from(if index == 7 { "alice" } else { "" }),
    }));
    // Original constructor and its organizer witness are supplied, not proved.
    let initial = contract::initial_state(
        ConstructorContext::new(7_u64),
        &CheckInWitness,
        participants,
    )?;
    if !slots::eligible_participants
        .inspect(initial.ledger_state.get_ref())?
        .member(participant.clone())
        || slots::checked_in_participants
            .inspect(initial.ledger_state.get_ref())?
            .member(participant.clone())
    {
        return Err("check-in constructor eligibility/attendance differs".into());
    }
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
    let native_witness = Witness::default();
    let recorded_witness = Witness::default();
    let observed_witness = Witness::default();
    let generated = contract::Contract::from(observed_witness.clone());
    let native = contract::check_in(
        observed.circuit_context(7_u64),
        &native_witness,
        participant.clone(),
    )?;
    let recorded = contract::recorded::check_in(
        observed.circuit_context(7_u64),
        &recorded_witness,
        participant.clone(),
    )?;
    let call = generated
        .recording()
        .check_in_call(&observed, 7_u64, participant.clone())?;
    let (frame, eligible) = slots::eligible_participants.record_member(
        RecordingFrame::new(observed.circuit_context(7_u64)),
        participant.clone(),
    )?;
    if !eligible {
        return Err(CompactError::AssertionFailed("Not eligible participant".into()).into());
    }
    let frame = slots::checked_in_participants.record_insert(frame, participant.clone())?;
    let mut manual_events = Vec::new();
    let (frame, ()) = frame.try_witness_metered(|context, _| {
        manual_events.push((context.private_state, participant.0.clone()));
        Ok((context.private_state + 1, ()))
    })?;
    let manual = frame.finish(());
    let expected_events = vec![(7, "alice".to_owned())];
    if manual_events != expected_events
        || [&native_witness, &recorded_witness, &observed_witness]
            .iter()
            .any(|witness| *witness.0.borrow() != expected_events)
    {
        return Err("check-in witness arguments/order/count differ".into());
    }
    let expected_private = vec![AlignedValue::from(())];
    if native.context.private_state != 8 || native.private_transcript_outputs != expected_private {
        return Err("check-in native private state/FAB differs".into());
    }
    let manual_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [&recorded, call.recorded(), &manual] {
        if candidate.execution.context.private_state != 8
            || candidate.execution.private_transcript_outputs != expected_private
            || candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.public.verify_ops() != manual.public.verify_ops()
        {
            return Err("check-in native/recorded/observed/manual effects differ".into());
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
            return Err("check-in replay differs from independent manual replay".into());
        }
    }
    // Results are Unit in all four lanes by their generated/static Rust types.
    // Full state equality to the manual single-slot insertion above also preserves
    // organizer keys and every unrelated ledger value.
    let expected_state = native.context.query.state.get_ref().clone();
    if !slots::eligible_participants
        .inspect(&expected_state)?
        .member(participant.clone())
        || !slots::checked_in_participants
            .inspect(&expected_state)?
            .member(participant.clone())
    {
        return Err("check-in lost eligibility or attendance".into());
    }
    let input = AlignedValue::from(participant);
    let direct = check_generated_trace(root, CIRCUIT, recorded, input.clone())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/check_in.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != input
        || prepared.output != AlignedValue::from(())
        || format!("{direct:?}") != format!("{prepared:?}")
    {
        return Err("check-in declared FAB/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &expected_state, rng, 605)?;
    println!(
        "original Welcome check_in: source-bound four-lane execution/replay, verified proof and changed-binding refusal, default-strict funded ledger-8 application; constructor state supplied, deployment not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_in_source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_check_in_source_control"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn check_in_absent_source_refuses_before_state_or_keys() {
        let root = std::env::temp_dir().join(format!(
            "compact-check-in-proof-absent-{}-{}",
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
