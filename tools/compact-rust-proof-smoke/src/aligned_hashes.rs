// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! One source-bound ledger8 hash-to-curve/persistent-hash proof with strict funding.
use super::*;
use compact_rust_effect_plan_aligned_hashes_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::recording::RecordingFrame;
use std::{cell::RefCell, rc::Rc};

type Event = (&'static str, u64, u128);
#[derive(Clone, Default)]
struct Witness(Rc<RefCell<Vec<Event>>>);
impl Witness {
    fn take(&self, name: &'static str, private: u64, count: u128) -> (u64, Field) {
        self.0.borrow_mut().push((name, private, count));
        (
            private + 1,
            Field::from(if name == "curveInput" { 1_u64 } else { 2_u64 }),
        )
    }
}
#[allow(non_snake_case)]
impl c::TryWitnesses<u64> for Witness {
    fn curveInput(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Field), r::CompactError> {
        Ok(self.take(
            "curveInput",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
        ))
    }
    fn hashInput(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Field), r::CompactError> {
        Ok(self.take("hashInput", *ctx.private_state, ctx.ledger.calls()?.value()))
    }
    fn expectedPoint(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        Err(r::CompactError::AssertionFailed(
            "hashes must not invoke expectedPoint".into(),
        ))
    }
    fn expectedDigest(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::FixedBytes<32>), r::CompactError> {
        Err(r::CompactError::AssertionFailed(
            "hashes must not invoke expectedDigest".into(),
        ))
    }
}

fn known_output() -> types::Hashes {
    // Frozen official TS runtime answers for curveInput=1 and hashInput=2.
    // No call to the Rust hashing implementation under test builds this oracle.
    types::Hashes {
        x: Field::from_le_bytes(&[
            157, 52, 201, 185, 204, 155, 101, 1, 10, 105, 33, 117, 104, 143, 190, 125, 12, 131,
            178, 249, 240, 240, 108, 254, 120, 107, 228, 167, 96, 185, 219, 18,
        ])
        .unwrap(),
        digest: r::FixedBytes([
            87, 120, 249, 133, 219, 117, 76, 102, 40, 105, 31, 86, 250, 218, 229, 12, 101, 253,
            219, 232, 235, 46, 147, 3, 150, 51, 254, 250, 5, 212, 94, 49,
        ]),
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "hashes";
    let mut rng = StdRng::seed_from_u64(0x0582_0000);
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
    let a = Witness::default();
    let b = Witness::default();
    let w = Witness::default();
    let m = Witness::default();
    let native = c::hashes(observed.circuit_context(5), &a, true)?;
    let recorded = c::recorded::hashes(observed.circuit_context(5), &b, true)?;
    let contract = c::Contract::from(w.clone());
    let call = contract.recording().hashes_call(&observed, 5, true)?;
    let frame =
        slots::calls.record_increment(RecordingFrame::new(observed.circuit_context(5)), 1)?;
    let (frame, curve) = frame.try_witness_metered(|ctx, meter| {
        Ok(m.take(
            "curveInput",
            ctx.private_state,
            slots::calls.witness_read(meter)? as u128,
        ))
    })?;
    let (frame, hash) = frame.try_witness_metered(|ctx, meter| {
        Ok(m.take(
            "hashInput",
            ctx.private_state,
            slots::calls.witness_read(meter)? as u128,
        ))
    })?;
    if curve != Field::from(1_u64) || hash != Field::from(2_u64) {
        return Err("aligned hash witness input mismatch".into());
    }
    let expected = known_output();
    let manual = frame.finish(expected.clone());
    for witness in [&a, &b, &w, &m] {
        if *witness.0.borrow() != vec![("curveInput", 5, 1), ("hashInput", 6, 1)] {
            return Err("aligned hash callback order/read/private state mismatch".into());
        }
    }
    let outputs = vec![AlignedValue::from(curve), AlignedValue::from(hash)];
    let manual_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [&recorded, call.recorded(), &manual] {
        if candidate.execution.result != expected
            || native.result != expected
            || candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.execution.context.private_state != 7
            || native.context.private_state != 7
            || candidate.execution.private_transcript_outputs != outputs
            || native.private_transcript_outputs != outputs
            || candidate.public.verify_ops() != manual.public.verify_ops()
            || candidate.public.verify_ops().is_empty()
        {
            return Err("aligned hash native/recorded/observed/manual differs".into());
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
            return Err("aligned hash verifier replay differs".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::calls.inspect(&expected_state)? != 1 {
        return Err("aligned hash Counter differs".into());
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/hashes.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from(true) || prepared.output != AlignedValue::from(expected)
    {
        return Err("aligned hash prepared original input/product output differs".into());
    }
    let direct = check_generated_trace(root, name, recorded, true)?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("aligned hash direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 582)?;
    println!(
        "hashes(selected=true, curve=1, hash=2): frozen TS outputs, ordered witnesses, Counter, replay, exact FAB, default-v2 verified proof+changed-binding refusal and default-strict funded ledger8 application; constructor state supplied, deployment not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mismatched_whole_source_refuses_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        value["independent_hash_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&value).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn known_answers_are_bound_to_fixture_oracle() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests-rust-backend/effect-plan-aligned-hashes-source/oracle/typescript.json"
        ))
        .unwrap();
        let rows = oracle["knownAnswers"].as_array().unwrap();
        let one = rows.iter().find(|r| r["input"] == "1").unwrap();
        let two = rows.iter().find(|r| r["input"] == "2").unwrap();
        // Decimal parsing through repeated field arithmetic is independent of
        // the fixed little-endian byte spelling in the selected proof adapter.
        let x = one["point"]["x"]
            .as_str()
            .unwrap()
            .bytes()
            .fold(Field::from(0_u64), |value, byte| {
                value * Field::from(10_u64) + Field::from((byte - b'0') as u64)
            });
        assert_eq!(known_output().x, x);
        assert_eq!(serde_json::json!(known_output().digest.0), two["digest"]);
    }
}
