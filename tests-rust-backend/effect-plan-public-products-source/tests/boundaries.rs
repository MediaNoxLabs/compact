// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_public_products_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, support::Witness, types,
};
use r::fab::{Alignment, AlignmentAtom};
use r::recording::{RecordedCircuitResult, RecordingFrame};

type Nine = (
    r::OpaqueBytes,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
);
type Eleven = (
    r::OpaqueBytes,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
);
fn nine(bytes: &[u8]) -> Nine {
    (
        bytes.to_vec().into(),
        1_u64.into(),
        2_u64.into(),
        3_u64.into(),
        4_u64.into(),
        5_u64.into(),
        6_u64.into(),
        7_u64.into(),
        8_u64.into(),
    )
}
fn eleven(bytes: &[u8]) -> Eleven {
    (
        bytes.to_vec().into(),
        1_u64.into(),
        2_u64.into(),
        3_u64.into(),
        4_u64.into(),
        5_u64.into(),
        6_u64.into(),
        7_u64.into(),
        8_u64.into(),
        9_u64.into(),
        10_u64.into(),
    )
}
fn compare(
    native: r::context::CircuitResult<u64, types::Receipt>,
    recorded: RecordedCircuitResult<u64, types::Receipt>,
    expected: u64,
) {
    assert_eq!(native.result.value, r::Field::from(expected));
    let manual = slots::calls
        .record_increment(
            RecordingFrame::new(
                compact_rust_effect_plan_public_products_source_fixture::support::context(),
            ),
            1,
        )
        .unwrap()
        .finish(types::Receipt {
            value: r::Field::from(expected),
        });
    for actual in [&recorded.execution, &manual.execution] {
        assert_eq!(actual.result, native.result);
        assert_eq!(actual.context.query.state, native.context.query.state);
        assert_eq!(actual.context.query.effects, native.context.query.effects);
        assert_eq!(actual.context.private_state, native.context.private_state);
        assert_eq!(actual.gas_cost, native.gas_cost);
        assert_eq!(
            actual.private_transcript_outputs,
            native.private_transcript_outputs
        );
    }
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
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    // Compare whole-transcript gas against an independently built whole transcript.
    let reference = manual
        .public
        .initial()
        .query(
            manual.public.verify_ops(),
            None,
            &manual.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.gas_cost, reference.gas_cost);
}
#[test]
fn standalone_tuple_vector_and_empty_interfaces_have_literal_encodings() {
    let (state, key) = setup();
    let witness = Witness::new(&[0]);
    let contract = c::Contract::from(witness.clone());
    let opaque = Alignment::singleton(AlignmentAtom::Compress);
    let field = Alignment::singleton(AlignmentAtom::Field);
    let tuple_shape =
        Alignment::concat(std::iter::once(&opaque).chain(std::iter::repeat_n(&field, 10)));
    let tuple = eleven(&[0, 9]);
    compare(
        c::tuple11(state.circuit_context(5), tuple.clone()).unwrap(),
        c::recorded::tuple11(state.circuit_context(5), tuple.clone()).unwrap(),
        1,
    );
    let prepared = contract
        .recording()
        .tuple11_call(&state, 5, tuple)
        .unwrap()
        .prepare(key.clone(), 0_u64.into())
        .unwrap();
    assert_eq!(
        atoms(&prepared.input),
        std::iter::once(vec![0, 9])
            .chain((1..=10).map(|n| vec![n]))
            .collect::<Vec<_>>()
    );
    assert_eq!(prepared.input.alignment, tuple_shape);
    assert_eq!(atoms(&prepared.output), vec![vec![1]]);
    assert_eq!(prepared.output.alignment, field);

    let vector = r::FixedVector::new([nine(&[1]), nine(&[2])]);
    compare(
        c::vector9(state.circuit_context(5), vector.clone()).unwrap(),
        c::recorded::vector9(state.circuit_context(5), vector.clone()).unwrap(),
        1,
    );
    let prepared = contract
        .recording()
        .vector9_call(&state, 5, vector)
        .unwrap()
        .prepare(key.clone(), 0_u64.into())
        .unwrap();
    let literal = [1, 2]
        .into_iter()
        .flat_map(|head| std::iter::once(vec![head]).chain((1..=8).map(|n| vec![n])))
        .collect::<Vec<_>>();
    assert_eq!(atoms(&prepared.input), literal);
    let nine_shape =
        Alignment::concat(std::iter::once(&opaque).chain(std::iter::repeat_n(&field, 8)));
    assert_eq!(
        prepared.input.alignment,
        Alignment::concat([&nine_shape, &nine_shape])
    );
    assert_eq!(atoms(&prepared.output), vec![vec![1]]);
    assert_eq!(prepared.output.alignment, field);

    let empty = r::FixedVector::<r::OpaqueBytes, 0>::new([]);
    compare(
        c::empty(state.circuit_context(5), (), empty.clone()).unwrap(),
        c::recorded::empty(state.circuit_context(5), (), empty.clone()).unwrap(),
        0,
    );
    let prepared = contract
        .recording()
        .empty_call(&state, 5, (), empty)
        .unwrap()
        .prepare(key, 0_u64.into())
        .unwrap();
    assert!(atoms(&prepared.input).is_empty());
    assert_eq!(
        prepared.input.alignment,
        Alignment::concat(std::iter::empty::<&Alignment>())
    );
    assert_eq!(atoms(&prepared.output), vec![Vec::<u8>::new()]);
    assert_eq!(prepared.output.alignment, field);
    assert!(witness.calls.borrow().is_empty());
}
#[test]
fn unused_tuple_and_vector_arguments_still_validate_every_opaque_leaf() {
    let (state, _) = setup();
    let witness = Witness::new(&[0]);
    let contract = c::Contract::from(witness.clone());
    let tuple = eleven(&[1, 0]);
    assert_eq!(
        c::tuple11(state.circuit_context(5), tuple.clone())
            .unwrap()
            .result
            .value,
        1_u64.into()
    );
    assert_eq!(
        contract
            .recording()
            .tuple11_call(&state, 5, tuple)
            .err()
            .unwrap(),
        invalid()
    );
    for index in 0..2 {
        let mut vector = r::FixedVector::new([nine(&[1]), nine(&[2])]);
        vector.0[index].0.0.push(0);
        assert_eq!(
            c::vector9(state.circuit_context(5), vector.clone())
                .unwrap()
                .result
                .value,
            1_u64.into()
        );
        assert_eq!(
            contract
                .recording()
                .vector9_call(&state, 5, vector)
                .err()
                .unwrap(),
            invalid()
        );
    }
    assert!(witness.calls.borrow().is_empty());
}
