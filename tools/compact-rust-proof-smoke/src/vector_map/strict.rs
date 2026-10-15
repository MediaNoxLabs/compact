// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original vector-map source with strict funded ledger8 application.
use super::*;
use compact_rust_map_fn_oracle_fixture::{ledger_contract as contract, ledger_slots as slots};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_compact_runtime::{BoundedUint, FixedVector};

const RETAINED_IR: &str =
    include_str!("../../../../tests-rust-backend/map-fn-oracle/tests/synthetic/original-ir.json");
const CIRCUIT: &str = "ping";

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let mut rng = StdRng::seed_from_u64(0x0613_4d41_5053);
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
    let native = contract::ping(observed.circuit_context(7_u64))?;
    let recorded = contract::recorded::ping(observed.circuit_context(7_u64))?;
    let call = contract::recorded::Contract.ping_call(&observed, 7_u64)?;
    let frame = RecordingFrame::new(observed.circuit_context(7_u64));
    let expected = FixedVector::new([BoundedUint::<18446744073709551615>::new(0)?; 3]);
    let manual = slots::c.record_write(frame, expected.clone())?.finish(());
    if native.context.private_state != 7 || !native.private_transcript_outputs.is_empty() {
        return Err("Vector-map changed private data".into());
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
            return Err("Vector-map native/recorded/observed/manual disagreement".into());
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
            return Err("Vector-map replay state/effects/gas disagreement".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::c.inspect(&expected_state)? != expected {
        return Err("vector-map result differs".into());
    }
    let original = FixedVector::new([
        BoundedUint::<18446744073709551615>::new(1)?,
        BoundedUint::<18446744073709551615>::new(2)?,
        BoundedUint::<18446744073709551615>::new(3)?,
    ]);
    if slots::c.inspect(deploy.initial_state.data.get_ref())? != original {
        return Err("vector-map supplied constructor state differs".into());
    }
    let direct = check_generated_trace(root, CIRCUIT, recorded, ())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/ping.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != AlignedValue::from(())
        || prepared.output != AlignedValue::from(())
        || format!("{direct:?}") != format!("{prepared:?}")
    {
        return Err("Vector-map observed preparation binding differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &expected_state, rng, 613)?;
    println!(
        "Vector-map: original identity mapping, native/recorded/observed/manual replay, proof and changed-binding rejection, strict funded ledger8 apply; constructor state supplied, not proved"
    );
    Ok(())
}

#[cfg(test)]
#[path = "strict_tests.rs"]
mod tests;
