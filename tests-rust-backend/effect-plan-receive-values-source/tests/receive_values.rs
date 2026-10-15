// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_receive_values_source_fixture::{
    ledger_contract as c, ledger_slots, runtime as r, types::Receipt,
};
use r::context::{CircuitContext, CircuitResult, ConstructorContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{Field, JubjubPoint};
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
fn reference(
    ctx: CircuitContext<u64>,
    direct: bool,
    selected: bool,
    equal_markers: bool,
) -> RecordedCircuitResult<u64, Receipt> {
    let frame = ledger_slots::stored
        .record_write(RecordingFrame::new(ctx), direct)
        .unwrap();
    let frame = if selected {
        ledger_slots::stored
            .record_write(frame, direct && equal_markers)
            .unwrap()
    } else {
        frame
    };
    let (frame, aggregate) = ledger_slots::stored.record_read(frame).unwrap();
    frame.finish(Receipt { direct, aggregate })
}
fn compare(
    native: &CircuitResult<u64, Receipt>,
    recorded: &RecordedCircuitResult<u64, Receipt>,
    manual: &RecordedCircuitResult<u64, Receipt>,
) {
    for execution in [&recorded.execution, &manual.execution] {
        assert_eq!(execution.result, native.result);
        assert_eq!(execution.context.query.state, native.context.query.state);
        assert_eq!(
            execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(
            execution.context.query.call_context.com_indices,
            native.context.query.call_context.com_indices
        );
        assert_eq!(execution.gas_cost, native.gas_cost);
        assert_eq!(execution.context.private_state, 7);
        assert!(execution.private_transcript_outputs.is_empty());
    }
    assert_eq!(native.context.private_state, 7);
    assert!(native.private_transcript_outputs.is_empty());
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
#[test]
fn official_point_equality_aggregate_markers_and_branch_order_match_known_results() {
    let generator = || r::ec_mul_generator(Field::from(1_u64)).unwrap();
    let cases = [
        (JubjubPoint::identity(), JubjubPoint::identity(), true),
        (generator(), generator(), true),
        (JubjubPoint::identity(), generator(), false),
        (
            generator(),
            r::ec_mul_generator(Field::from(2_u64)).unwrap(),
            false,
        ),
    ];
    for (left, right, equal_points) in cases {
        for equal_markers in [false, true] {
            for selected in [false, true] {
                let left_marker = Field::from(3_u64);
                let right_marker = Field::from(if equal_markers { 3_u64 } else { 4 });
                let native =
                    c::points(context(), left, right, left_marker, right_marker, selected).unwrap();
                let recorded = c::recorded::points(
                    context(),
                    left,
                    right,
                    left_marker,
                    right_marker,
                    selected,
                )
                .unwrap();
                let manual = reference(context(), equal_points, selected, equal_markers);
                assert_eq!(native.result.direct, equal_points);
                assert_eq!(
                    native.result.aggregate,
                    equal_points && (!selected || equal_markers)
                );
                compare(&native, &recorded, &manual);
            }
        }
    }
}
#[test]
fn empty_named_struct_default_construction_and_reuse_equal_after_either_prior_state() {
    for prior in [false, true] {
        let start = || {
            ledger_slots::stored
                .write(context(), prior)
                .unwrap()
                .context
        };
        let native = c::emptyLocals(start()).unwrap();
        let recorded = c::recorded::emptyLocals(start()).unwrap();
        let manual = reference(start(), true, false, true);
        assert_eq!(
            native.result,
            Receipt {
                direct: true,
                aggregate: true
            }
        );
        compare(&native, &recorded, &manual);
    }
}
