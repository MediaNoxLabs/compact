// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_owned_cells_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots, runtime as r,
    support::{context, envelope, reference},
};
use r::context::{CircuitResult, ConstructorContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field};
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
fn owned_cells_branch_read_projection_write_and_reuse_match_manual_program() {
    for populated in [false, true] {
        for selected in [false, true] {
            let native = c::run(context(populated, false), selected).unwrap();
            let recorded = c::recorded::run(context(populated, false), selected).unwrap();
            let manual = reference(context(populated, false), selected).unwrap();
            compare(&native, &recorded, &manual);
            assert!(native.result.matched);
            assert_eq!(
                native.result.marker,
                Field::from(if selected {
                    if populated { 41_u64 } else { 0 }
                } else {
                    9
                })
            );
            let stored = ledger_slots::envelope
                .inspect(native.context.query.state.get_ref())
                .unwrap();
            assert_eq!(stored.entry.is_some, !selected || populated);
            assert_eq!(
                stored.entry.value,
                if selected {
                    envelope(populated).entry.value
                } else {
                    Default::default()
                }
            );
            assert_eq!(
                stored.bytes,
                if selected {
                    envelope(populated).bytes
                } else {
                    Default::default()
                }
            );
            assert_eq!(
                ledger_slots::text
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                stored.entry.value
            );
            assert_eq!(
                ledger_slots::blob
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                stored.bytes
            );
            assert_eq!(
                ledger_slots::padding0
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                Field::from(if populated { 100_u64 } else { 0 })
            );
            assert_eq!(
                ledger_slots::padding11
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                Field::from(if populated { 111_u64 } else { 0 })
            );
            assert_eq!(native.context.private_state, 7);
            assert!(native.private_transcript_outputs.is_empty());
        }
    }
}
#[test]
fn real_source_constructor_and_nested_slot_layout_match_seeded_controls() {
    let native = c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default());
    assert_eq!(native.query.state, context(false, false).query.state);
    let ir: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
    let fields = ir["ledger_fields"].as_array().unwrap();
    assert_eq!(fields.len(), 16);
    assert_eq!(fields[0]["id"], "text");
    assert_eq!(fields[0]["path"], serde_json::json!([0, 0]));
    assert_eq!(fields[1]["path"], serde_json::json!([1, 0]));
    for populated in [false, true] {
        let native = c::inspectText(context(populated, false)).unwrap();
        let recorded = c::recorded::inspectText(context(populated, false)).unwrap();
        let (frame, _value) = ledger_slots::text
            .record_read(RecordingFrame::new(context(populated, false)))
            .unwrap();
        let manual = frame.finish(
            compact_rust_effect_plan_owned_cells_source_fixture::types::Receipt {
                matched: true,
                marker: Field::from(0_u64),
            },
        );
        compare(&native, &recorded, &manual);
        assert!(native.result.matched);
        assert_eq!(native.result.marker, Field::from(0_u64));
    }
}
#[test]
fn malformed_owned_string_decode_is_same_runtime_failure() {
    let native = c::inspectText(context(false, true)).err().unwrap();
    let recorded = c::recorded::inspectText(context(false, true))
        .err()
        .unwrap();
    let manual = ledger_slots::text
        .record_read(RecordingFrame::new(context(false, true)))
        .err()
        .unwrap();
    assert!(
        matches!(&native,CompactError::InvalidLedgerCell(message) if message.contains("utf-8"))
    );
    assert_eq!(recorded, native);
    assert_eq!(manual, native);
}

// Existing runtime-policy debt, tracked in CoPS-011. This characterizes the
// pre-execution boundary; it does not claim preservation of arbitrary payloads.
#[test]
fn existing_constructor_normalizes_noncanonical_opaque_atoms_before_execution() {
    use r::ledger::{DefaultDB, constructor_cell, read_cell};
    for (input, expected) in [
        (vec![0], vec![]),
        (vec![0, 1, 255, 0], vec![0, 1, 255]),
        (vec![1, 0, 2], vec![1, 0, 2]),
    ] {
        let state = constructor_cell::<_, DefaultDB>(r::OpaqueBytes::from(input));
        let actual = read_cell::<r::OpaqueBytes, DefaultDB>(&state).unwrap();
        assert_eq!(actual, r::OpaqueBytes::from(expected));
    }
    for (input, expected) in [("a\0", "a"), ("\0", ""), ("a\0b", "a\0b")] {
        let state = constructor_cell::<_, DefaultDB>(r::OpaqueString::from(input));
        let actual = read_cell::<r::OpaqueString, DefaultDB>(&state).unwrap();
        assert_eq!(actual, r::OpaqueString::from(expected));
    }
}
