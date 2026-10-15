// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! A selected broad Unit call, with ordered private callbacks and strict funding.
use super::*;
use compact_rust_effect_plan_unit_composition_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::recording::RecordingFrame;
use std::{cell::RefCell, rc::Rc};

type WitnessEvent = (&'static str, u64, u128);
#[derive(Clone, Default)]
struct Witness(Rc<RefCell<Vec<WitnessEvent>>>);
impl Witness {
    fn take(&self, name: &'static str, private: u64, count: u128) -> (u64, bool) {
        self.0.borrow_mut().push((name, private, count));
        (private + 1, true)
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn first(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok(self.take("first", *ctx.private_state, ctx.ledger.calls()?.value()))
    }
    fn second(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok(self.take("second", *ctx.private_state, ctx.ledger.calls()?.value()))
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "run";
    let mut rng = StdRng::seed_from_u64(0x0580_0000);
    let key: r::OpaqueString = "λ\0key".into();
    let initial = c::initial_state(ConstructorContext::new(5_u64), "seed".into())?;
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
    let native_witness = Witness::default();
    let recorded_witness = Witness::default();
    let observed_witness = Witness::default();
    let manual_witness = Witness::default();
    let native = c::run(
        observed.circuit_context(5),
        &native_witness,
        key.clone(),
        true,
        true,
    )?;
    let recorded = c::recorded::run(
        observed.circuit_context(5),
        &recorded_witness,
        key.clone(),
        true,
        true,
    )?;
    let contract = c::Contract::from(observed_witness.clone());
    let call = contract
        .recording()
        .run_call(&observed, 5, key.clone(), true, true)?;
    // Independent selected-true execution. Guard/assertions succeed; callbacks
    // read exactly the same Counter moments as the generated source.
    let frame = RecordingFrame::new(observed.circuit_context(5));
    let (frame, first) = frame.try_witness_metered(|ctx, meter| {
        Ok(manual_witness.take(
            "first",
            ctx.private_state,
            slots::calls.witness_read(meter)? as u128,
        ))
    })?;
    assert!(first);
    let frame = slots::calls.record_increment(frame, 1)?;
    let frame =
        slots::entries.record_insert(frame, key.clone(), types::Entry { label: key.clone() })?;
    let (frame, second) = frame.try_witness_metered(|ctx, meter| {
        Ok(manual_witness.take(
            "second",
            ctx.private_state,
            slots::calls.witness_read(meter)? as u128,
        ))
    })?;
    assert!(second);
    let frame = slots::calls.record_increment(frame, 1)?;
    let manual = slots::done.record_write(frame, true)?.finish(());
    let events = vec![("first", 5, 0), ("second", 6, 1)];
    for witness in [
        &native_witness,
        &recorded_witness,
        &observed_witness,
        &manual_witness,
    ] {
        if *witness.0.borrow() != events {
            return Err("Unit callback order/private-state/read moments differ".into());
        }
    }
    let outputs = vec![AlignedValue::from(true), AlignedValue::from(true)];
    let manual_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [&recorded, call.recorded(), &manual] {
        if candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.execution.context.private_state != 7
            || native.context.private_state != 7
            || candidate.execution.private_transcript_outputs != outputs
            || native.private_transcript_outputs != outputs
            || candidate.public.verify_ops() != manual.public.verify_ops()
            || candidate.public.verify_ops().is_empty()
        {
            return Err("Unit native/recorded/observed/manual execution differs".into());
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
            return Err("Unit verifier replay differs".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::calls.inspect(&expected_state)? != 2
        || !slots::done.inspect(&expected_state)?
        || slots::entries
            .inspect(&expected_state)?
            .lookup(key.clone())?
            != (types::Entry { label: key.clone() })
        || slots::entries
            .inspect(&expected_state)?
            .lookup("seed".into())?
            != (types::Entry {
                label: "seed".into(),
            })
    {
        return Err("Unit final Counter/Cell/Map state differs".into());
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/run.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from((key.clone(), true, true))
        || prepared.output != AlignedValue::from(())
    {
        return Err("Unit original argument order or output binding differs".into());
    }
    let direct = check_generated_trace(root, name, recorded, (key, true, true))?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("Unit direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 580)?;
    println!(
        "Unit run(selected=true, enabled=true): ordered witnesses, repeated helper, Map/Counter/Cell, exact declared bindings, verified proof and changed-binding refusal, default-strict Dust-funded call application passed; generated constructor state supplied, deployment itself not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_source_refuses_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut altered: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        altered["independent_unit_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&altered).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-unit-proof-absent-{}-{}",
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
