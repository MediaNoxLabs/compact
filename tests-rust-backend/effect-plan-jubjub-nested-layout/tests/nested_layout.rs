// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Explicit synthetic IR qualification; this layout has no source/proof claim.
use compact_rust_effect_plan_jubjub_nested_layout_fixture::{
    ledger_contract as contract, ledger_slots, runtime as r,
};
use r::context::{CircuitContext, ConstructorContext, ConstructorResult};
use r::ledger::{ChargedState, DefaultDB, StateValue, constructor_cell};
use r::recording::RecordingFrame;
use r::{Field, JubjubPoint};

fn context(nested: bool) -> CircuitContext<u64> {
    let cells = || {
        (0..15)
            .map(|_| constructor_cell::<_, DefaultDB>(JubjubPoint::identity()))
            .collect::<Vec<_>>()
    };
    let fields = if nested {
        vec![
            StateValue::Array(
                vec![constructor_cell::<_, DefaultDB>(JubjubPoint::identity())].into(),
            ),
            StateValue::Array(cells().into()),
        ]
    } else {
        let mut flat = cells();
        flat.push(constructor_cell::<_, DefaultDB>(JubjubPoint::identity()));
        flat
    };
    ConstructorResult::new(
        ConstructorContext::new(7),
        ChargedState::new(StateValue::Array(fields.into())),
    )
    .into_circuit_context(Default::default())
}

#[test]
fn declared_nested_paths_preserve_query_order_and_neighboring_point_cells() {
    let point = r::ec_mul_generator(Field::from(7_u64)).unwrap();
    let scalar = Field::from(3_u64);
    for selected in [false, true] {
        let native = contract::read_branch(context(true), selected, point, scalar).unwrap();
        let recorded =
            contract::recorded::read_branch(context(true), selected, point, scalar).unwrap();
        let frame = ledger_slots::first
            .record_write(RecordingFrame::new(context(true)), point)
            .unwrap();
        let (frame, supplied) = ledger_slots::first.record_read(frame).unwrap();
        let expected = if selected {
            r::ec_add(supplied, r::ec_mul_generator(scalar).unwrap())
        } else {
            r::ec_mul(supplied, scalar).unwrap()
        };
        let frame = ledger_slots::second.record_write(frame, expected).unwrap();
        let (frame, result) = ledger_slots::second.record_read(frame).unwrap();
        let manual = frame.finish(result);
        assert_eq!(native.result, expected);
        assert_eq!(recorded.execution.result, expected);
        assert_eq!(
            recorded.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(
            manual.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(
            recorded.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(
            manual.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(recorded.execution.gas_cost, native.gas_cost);
        assert_eq!(manual.execution.gas_cost, native.gas_cost);
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
        let control = manual
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
        assert_eq!(replay.gas_cost, control.gas_cost);
        assert_eq!(
            ledger_slots::padding_2
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            JubjubPoint::identity()
        );
        assert_eq!(
            ledger_slots::padding_15
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            JubjubPoint::identity()
        );
    }
}

#[test]
fn a_flat_runtime_state_cannot_satisfy_the_declared_nested_layout() {
    let point = r::ec_mul_generator(Field::from(7_u64)).unwrap();
    let scalar = Field::from(3_u64);
    let native = contract::read_branch(context(false), true, point, scalar)
        .err()
        .expect("native layout refusal");
    let recorded = contract::recorded::read_branch(context(false), true, point, scalar)
        .err()
        .expect("recorded layout refusal");
    assert_eq!(recorded, native);
}
