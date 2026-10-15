// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Removing a nested Map consumes only its key, never a scalar value codec.
use midnight_compact_runtime::context::{CircuitContext, ConstructorContext, ConstructorResult};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, LedgerHashMap, StateValue, constructor_cell,
};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_compact_runtime::slots::{MapNode, MapSlot};
use midnight_compact_runtime::{CompactError, Field, OpaqueString};

const NESTED: MapSlot<OpaqueString, MapNode<bool, Field>> = MapSlot::new(&[0]);

fn key(value: &str) -> OpaqueString {
    OpaqueString::from(value)
}

fn child(number: u64) -> StateValue<DefaultDB> {
    StateValue::Map(
        LedgerHashMap::new()
            .insert(
                AlignedValue::from(false),
                constructor_cell(Field::from(number)),
            )
            .insert(
                AlignedValue::from(true),
                constructor_cell(Field::from(number + 1)),
            ),
    )
}

fn state(selected: bool) -> StateValue<DefaultDB> {
    let mut entries = LedgerHashMap::new().insert(AlignedValue::from(key("sibling")), child(90));
    if selected {
        entries = entries.insert(AlignedValue::from(key("selected")), child(10));
    }
    StateValue::Array(
        vec![
            StateValue::Map(entries),
            constructor_cell(Field::from(77_u64)),
        ]
        .into(),
    )
}

fn context(value: StateValue<DefaultDB>) -> CircuitContext<u64> {
    ConstructorResult::new(ConstructorContext::new(7_u64), ChargedState::new(value))
        .into_circuit_context(ContractAddress::default())
}

#[test]
fn nested_map_removal_preserves_other_children_and_replays_exactly() {
    for present in [true, false] {
        let initial = state(present);
        let native = NESTED
            .remove(context(initial.clone()), key("selected"))
            .unwrap();
        let recorded = NESTED
            .record_remove(
                RecordingFrame::new(context(initial.clone())),
                key("selected"),
            )
            .unwrap()
            .finish(());
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        // The expected official ledger state is constructed independently: both
        // entries of the selected child vanish; the entire other child and the
        // unrelated sibling Cell remain byte-for-byte equal. Absent removal is
        // the existing ledger no-op policy, rather than a lookup error.
        let expected = state(false);
        assert_eq!(
            native.context.query.state.get_ref(),
            &expected,
            "present={present}"
        );
        assert_eq!(
            recorded.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(
            recorded.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(replay.context.effects, native.context.query.effects);
        assert_eq!(recorded.execution.gas_cost, native.gas_cost);
        // A single removal is already a ResultModeVerify program in native and
        // recorded execution. Replaying that same single query has the same cost.
        assert_eq!(replay.gas_cost, native.gas_cost);
        assert_eq!(native.context.private_state, 7);
        assert_eq!(recorded.execution.context.private_state, 7);
        assert!(native.private_transcript_outputs.is_empty());
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        if !present {
            assert_eq!(native.context.query.state.get_ref(), &initial);
        }
    }
}

#[test]
fn nested_map_removal_keeps_key_validation_and_invalid_slot_refusals() {
    let invalid_key = key("selected\0");
    let native = NESTED
        .remove(context(state(true)), invalid_key.clone())
        .err()
        .unwrap();
    let recorded = NESTED
        .record_remove(RecordingFrame::new(context(state(true))), invalid_key)
        .err()
        .unwrap();
    // Both paths validate the key before constructing a ledger query.
    assert!(
        matches!(&native, CompactError::InvalidLedgerCell(detail) if detail.contains("trailing zero"))
    );
    assert_eq!(recorded, native);
    const WRONG_SLOT: MapSlot<OpaqueString, MapNode<bool, Field>> = MapSlot::new(&[1]);
    let native = WRONG_SLOT
        .remove(context(state(true)), key("selected"))
        .err()
        .unwrap();
    let recorded = WRONG_SLOT
        .record_remove(RecordingFrame::new(context(state(true))), key("selected"))
        .err()
        .unwrap();
    assert_eq!(native, recorded);
    assert!(matches!(native, CompactError::LedgerQueryRejected(_)));
}
