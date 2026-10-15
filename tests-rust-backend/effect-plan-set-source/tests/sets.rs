// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_set_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, runtime as r,
    support::{Fault, context, key, lifecycle_reference, reference},
    types,
};
use r::context::{CircuitResult, ConstructorContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{BoundedUint, CompactError, Field};
fn compare<T: PartialEq + std::fmt::Debug>(
    native: &CircuitResult<u64, T>,
    recorded: &RecordedCircuitResult<u64, T>,
    manual: &RecordedCircuitResult<u64, T>,
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
        assert_eq!(
            execution.context.private_state,
            native.context.private_state
        );
        assert_eq!(
            execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
        assert_eq!(execution.gas_cost, native.gas_cost);
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
fn scalar_owned_and_string_sets_preserve_duplicate_distinct_and_branch_semantics() {
    for populated in [false, true] {
        let mut native_ctx = context(populated, Fault::None);
        let mut recorded_ctx = context(populated, Fault::None);
        let mut manual_ctx = context(populated, Fault::None);
        let mut expected_scalars = Vec::new();
        let mut expected_keys = Vec::new();
        let mut expected_names = Vec::new();
        for (index, (seed, selected)) in [(7_u64, true), (7, true), (9, false), (9, true)]
            .into_iter()
            .enumerate()
        {
            let stored = if index == 3 {
                types::OwnedKey {
                    text: "other".into(),
                    bytes: vec![2, 0, 3].into(),
                    marker: Field::from(22_u64),
                }
            } else {
                key(populated)
            };
            if index == 3 {
                native_ctx = slots::storedKey
                    .write(native_ctx, stored.clone())
                    .unwrap()
                    .context;
                recorded_ctx = slots::storedKey
                    .write(recorded_ctx, stored.clone())
                    .unwrap()
                    .context;
                manual_ctx = slots::storedKey
                    .write(manual_ctx, stored.clone())
                    .unwrap()
                    .context;
            }
            let chosen = if selected {
                stored
            } else {
                types::OwnedKey::default()
            };
            let scalar_before = expected_scalars.contains(&seed);
            let owned_before = expected_keys.contains(&chosen);
            let name_before = expected_names.contains(&chosen.text);
            if !scalar_before {
                expected_scalars.push(seed);
            }
            if selected && !owned_before {
                expected_keys.push(chosen.clone());
            }
            if !name_before {
                expected_names.push(chosen.text.clone());
            }
            let seed = Field::from(seed);
            let native = c::run(native_ctx, seed, selected).unwrap();
            let recorded = c::recorded::run(recorded_ctx, seed, selected).unwrap();
            let manual = reference(manual_ctx, seed, selected).unwrap();
            compare(&native, &recorded, &manual);
            assert_eq!(
                native.result,
                types::Receipt {
                    scalarBefore: scalar_before,
                    scalarAfter: true,
                    ownedBefore: owned_before,
                    ownedAfter: owned_before || selected,
                    nameBefore: name_before,
                    nameAfter: true,
                    marker: seed
                }
            );
            let state = native.context.query.state.get_ref();
            assert_eq!(
                slots::scalars
                    .inspect(state)
                    .unwrap()
                    .size()
                    .unwrap()
                    .value(),
                expected_scalars.len() as u128
            );
            assert_eq!(
                slots::keys.inspect(state).unwrap().size().unwrap().value(),
                expected_keys.len() as u128
            );
            assert_eq!(
                slots::names.inspect(state).unwrap().size().unwrap().value(),
                expected_names.len() as u128
            );
            for key in &expected_keys {
                assert!(
                    slots::keys
                        .inspect(state)
                        .unwrap()
                        .try_member(key.clone())
                        .unwrap()
                );
            }
            assert_eq!(
                slots::padding0.inspect(state).unwrap(),
                Field::from(if populated { 100_u64 } else { 0 })
            );
            assert_eq!(
                slots::padding10.inspect(state).unwrap(),
                Field::from(if populated { 110_u64 } else { 0 })
            );
            assert_eq!(native.context.private_state, 7);
            assert!(native.private_transcript_outputs.is_empty());
            native_ctx = native.context;
            recorded_ctx = recorded.execution.context;
            manual_ctx = manual.execution.context;
        }
    }
}
#[test]
fn complete_set_lifecycle_and_direct_returns_match_official_queries() {
    for populated in [false, true] {
        let start = || {
            let ctx = context(populated, Fault::None);
            if populated {
                slots::scalars
                    .insert(ctx, Field::from(99_u64))
                    .unwrap()
                    .context
            } else {
                ctx
            }
        };
        let native = c::scalarSize(start()).unwrap();
        let recorded = c::recorded::scalarSize(start()).unwrap();
        let (frame, size) = slots::scalars
            .record_size(RecordingFrame::new(start()))
            .unwrap();
        let manual = frame.finish(BoundedUint::<18446744073709551615>::new(size as u128).unwrap());
        compare(&native, &recorded, &manual);
        assert_eq!(native.result.value(), u128::from(populated));
        let native = c::scalarEmpty(start()).unwrap();
        let recorded = c::recorded::scalarEmpty(start()).unwrap();
        let (frame, empty) = slots::scalars
            .record_is_empty(RecordingFrame::new(start()))
            .unwrap();
        compare(&native, &recorded, &frame.finish(empty));
        assert_eq!(native.result, !populated);
        let seed = Field::from(7_u64);
        let native = c::lifecycle(start(), seed).unwrap();
        let recorded = c::recorded::lifecycle(start(), seed).unwrap();
        let manual = lifecycle_reference(start(), seed).unwrap();
        compare(&native, &recorded, &manual);
        assert_eq!(
            slots::scalars
                .inspect(native.context.query.state.get_ref())
                .unwrap()
                .size()
                .unwrap()
                .value(),
            0
        );
    }
}
#[test]
fn invalid_containers_and_selected_malformed_owned_cell_fail_at_the_same_boundary() {
    let seed = Field::from(3_u64);
    for selected in [false, true] {
        let native = c::run(context(false, Fault::ScalarContainer), seed, selected)
            .err()
            .unwrap();
        let recorded = c::recorded::run(context(false, Fault::ScalarContainer), seed, selected)
            .err()
            .unwrap();
        let manual = reference(context(false, Fault::ScalarContainer), seed, selected)
            .err()
            .unwrap();
        assert_eq!(native, recorded);
        assert_eq!(native, manual);
    }
    let native = c::run(context(false, Fault::StoredText), seed, true)
        .err()
        .unwrap();
    let recorded = c::recorded::run(context(false, Fault::StoredText), seed, true)
        .err()
        .unwrap();
    let manual = reference(context(false, Fault::StoredText), seed, true)
        .err()
        .unwrap();
    assert!(
        matches!(&native,CompactError::InvalidLedgerCell(message) if message.contains("utf-8"))
    );
    assert_eq!(native, recorded);
    assert_eq!(native, manual);
    let native = c::run(context(false, Fault::StoredText), seed, false).unwrap();
    let recorded = c::recorded::run(context(false, Fault::StoredText), seed, false).unwrap();
    let manual = reference(context(false, Fault::StoredText), seed, false).unwrap();
    compare(&native, &recorded, &manual);
}
#[test]
fn source_constructor_and_sixteen_declarations_match_real_nested_paths() {
    let generated = c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default());
    assert_eq!(
        generated.query.state,
        context(false, Fault::None).query.state
    );
    let ir: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
    let fields = ir["ledger_fields"].as_array().unwrap();
    assert_eq!(fields.len(), 16);
    assert_eq!(fields[0]["id"], "scalars");
    assert_eq!(fields[0]["path"], serde_json::json!([0, 0]));
    assert_eq!(fields[1]["path"], serde_json::json!([1, 0]));
    assert_eq!(fields[15]["path"], serde_json::json!([1, 14]));
}
