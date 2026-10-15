// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::common::*;
#[test]
fn nonidentity_product_maps_preserve_captures_and_outer_shadowed_value() {
    let (state, key) = setup();
    let values = vector([1, 3, 5]);
    let native = c::products(
        state.circuit_context(7),
        values.clone(),
        field(10),
        field(99),
    )
    .unwrap();
    let recorded = c::recorded::products(
        state.circuit_context(7),
        values.clone(),
        field(10),
        field(99),
    )
    .unwrap();
    let contract = c::Contract::from(Offset::default());
    let call = contract
        .recording()
        .products_call(&state, 7, values, field(10), field(99))
        .unwrap();
    let expected = crate::types::ProductBatch {
        values: r::FixedVector::new([1, 3, 5].map(|n| crate::types::MappedItem {
            sum: field(n + 10),
            pair: (field(n), field(10)),
        })),
        outer: field(99),
    };
    for actual in [&recorded, call.recorded()] {
        pure_parity(&state, &native, actual, expected.clone());
    }
    assert_eq!(
        r::fab::AlignedValue::from(native.result),
        r::fab::AlignedValue::from(expected)
    );
    crate::support::refuses_empty(call, key);
}
#[test]
fn nested_map_formals_shadow_locally_without_losing_outer_captures() {
    let (state, key) = setup();
    let native = c::nested(state.circuit_context(7), field(10), field(99)).unwrap();
    let recorded = c::recorded::nested(state.circuit_context(7), field(10), field(99)).unwrap();
    let contract = c::Contract::from(Offset::default());
    let call = contract
        .recording()
        .nested_call(&state, 7, field(10), field(99))
        .unwrap();
    let expected = crate::types::NestedBatch {
        matrix: r::FixedVector::new([vector([14, 15]), vector([15, 16])]),
        outer: field(99),
    };
    for actual in [&recorded, call.recorded()] {
        pure_parity(&state, &native, actual, expected.clone());
    }
    crate::support::refuses_empty(call, key);
}
#[test]
fn empty_map_has_exact_typed_output_and_does_not_run_its_fallible_body() {
    let (state, key) = setup();
    let native = c::empty(state.circuit_context(7)).unwrap();
    let recorded = c::recorded::empty(state.circuit_context(7)).unwrap();
    let contract = c::Contract::from(Offset::default());
    let call = contract.recording().empty_call(&state, 7).unwrap();
    for actual in [&recorded, call.recorded()] {
        pure_parity(&state, &native, actual, vector([]));
    }
    assert_eq!(
        r::fab::AlignedValue::from(native.result),
        r::fab::AlignedValue::from(vector([]))
    );
    crate::support::refuses_empty(call, key);
}
