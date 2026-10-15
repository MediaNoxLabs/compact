// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Newly composable ordinary returns; separate from old-family inclusion.
use compact_rust_effect_plan_terminal_composition_source_fixture::{
    ledger_contract as c, ledger_slots, runtime as r,
};
use r::Field;
use r::context::{CircuitContext, ConstructorContext};
use r::recording::RecordingFrame;

fn context(rounds: usize) -> CircuitContext<()> {
    let mut ctx = c::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(Default::default());
    for i in 0..rounds {
        ctx = c::update(ctx, Field::from(i as u64)).unwrap().context;
    }
    ctx
}
#[test]
fn two_cells_and_pure_helper_match_native_and_independent_recording() {
    for rounds in [0, 2] {
        for seed in [0_u64, 1, 19] {
            let seed = Field::from(seed);
            let native = c::update(context(rounds), seed).unwrap();
            let recorded = c::recorded::update(context(rounds), seed).unwrap();
            let (frame, before) = ledger_slots::first
                .record_read(RecordingFrame::new(context(rounds)))
                .unwrap();
            // Official primitive oracle; the two-field tuple is the source vector
            // aligned representation. Do not invoke the generated pure helper.
            let value = r::transient_hash((before, seed));
            let frame = ledger_slots::first.record_write(frame, value).unwrap();
            let (frame, prior) = ledger_slots::second.record_read(frame).unwrap();
            let after = prior + value;
            let frame = ledger_slots::second.record_write(frame, after).unwrap();
            let manual = frame.finish(after);
            assert_eq!(native.result, after);
            for execution in [&recorded.execution, &manual.execution] {
                assert_eq!(execution.result, native.result);
                assert_eq!(execution.context.query.state, native.context.query.state);
                assert_eq!(
                    execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(execution.gas_cost, native.gas_cost);
                assert!(execution.private_transcript_outputs.is_empty());
            }
            assert_eq!(
                ledger_slots::first
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                value
            );
            assert_eq!(
                ledger_slots::second
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                after
            );
            assert_eq!(
                serde_json::to_value(recorded.public.verify_ops()).unwrap(),
                serde_json::to_value(manual.public.verify_ops()).unwrap()
            );
            let replay = recorded
                .public
                .initial()
                .query(
                    recorded.public.verify_ops(),
                    None,
                    &recorded.execution.context.cost_model,
                )
                .unwrap();
            let expected = manual
                .public
                .initial()
                .query(
                    manual.public.verify_ops(),
                    None,
                    &manual.execution.context.cost_model,
                )
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, expected.gas_cost);
        }
    }
}
#[test]
fn selected_empty_branch_needs_no_synthetic_read_or_write() {
    for selected in [false, true] {
        let seed = Field::from(9_u64);
        let native = c::optional_write(context(0), selected, seed).unwrap();
        let recorded = c::recorded::optional_write(context(0), selected, seed).unwrap();
        assert_eq!(native.result, seed);
        assert_eq!(recorded.execution.result, seed);
        assert_eq!(
            recorded.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(
            recorded.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(recorded.execution.gas_cost, native.gas_cost);
        assert_eq!(recorded.public.verify_ops().is_empty(), !selected);
        assert_eq!(
            ledger_slots::first
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            if selected { seed } else { Field::from(0_u64) }
        );
    }
}
#[cfg(feature = "ledger-transaction")]
#[test]
fn empty_branch_is_recordable_but_not_a_prepared_call() {
    use midnight_serialize::{Deserializable, Serializable};
    use r::transaction::{CallSpec, PrepareCallError, VerifierKey, prepare_call};
    let seed = Field::from(9_u64);
    let recorded = c::recorded::optional_write(context(0), false, seed).unwrap();
    assert!(recorded.public.verify_ops().is_empty());
    // Empty-program refusal precedes key inspection; this carrier is not a proof key.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let carrier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let spec = CallSpec::new("optional_write", carrier, (false, seed), Field::from(0_u64));
    assert!(matches!(
        prepare_call(recorded, spec),
        Err(PrepareCallError::EmptyTranscript)
    ));
}
