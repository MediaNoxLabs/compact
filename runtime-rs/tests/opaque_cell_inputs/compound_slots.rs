// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Compound raw input must not alias a canonical ledger key after normalization.
use super::*;
use midnight_compact_runtime as r;
use r::context::{CircuitContext, ConstructorContext, ConstructorResult};
use r::ledger::{ChargedState, StateValue};
use r::recording::RecordingFrame;
use r::slots::{CellSlot, MapSlot, SetSlot};

fn nested(last: &str) -> Nested {
    Nested {
        payloads: FixedVector::new([
            (payload("first", &[1]), OpaqueBytes::from(vec![2])),
            (payload(last, &[3]), OpaqueBytes::from(vec![4])),
        ]),
    }
}
fn context(initial: &StateValue<DefaultDB>) -> CircuitContext<u64> {
    ConstructorResult::new(
        ConstructorContext::new(7),
        ChargedState::new(initial.clone()),
    )
    .into_circuit_context(Default::default())
}
fn initial(canonical: Nested) -> StateValue<DefaultDB> {
    let state = StateValue::Array(
        vec![
            ledger::try_constructor_cell(canonical.clone()).unwrap(),
            ledger::constructor_set(),
            ledger::constructor_map(),
        ]
        .into(),
    );
    let set = SetSlot::<Nested>::new(&[1])
        .insert(context(&state), canonical.clone())
        .unwrap();
    let map = MapSlot::<Nested, r::Field>::new(&[2])
        .insert(set.context, canonical, r::Field::from(42_u64))
        .unwrap();
    // A distinct populated key survives removing the selected compound key.
    let set = SetSlot::<Nested>::new(&[1])
        .insert(map.context, nested("sibling"))
        .unwrap();
    let map = MapSlot::<Nested, r::Field>::new(&[2])
        .insert(set.context, nested("sibling"), r::Field::from(90_u64))
        .unwrap();
    map.context.query.state.get_ref().clone()
}
#[test]
fn later_opaque_member_is_rejected_across_cell_set_and_map_before_aliasing() {
    let canonical = nested("last");
    let bad = nested("last\0");
    let initial = initial(canonical.clone());
    // Demonstrate why validating the raw value matters: legacy conversion alone
    // normalizes these values to the same encoding. We do not change that API.
    assert_eq!(Value::from(bad.clone()), Value::from(canonical.clone()));
    let expected =
        CompactError::InvalidLedgerCell("opaque ledger input has trailing zero bytes".into());
    assert_eq!(bad.validate_cell_value().unwrap_err(), expected);
    macro_rules! rejects {
        ($native:expr,$recorded:expr) => {{
            assert_eq!($native.err().expect("native must reject"), expected);
            assert_eq!($recorded.err().expect("recorded must reject"), expected);
        }};
    }
    rejects!(
        CellSlot::<Nested>::new(&[0]).write(context(&initial), bad.clone()),
        CellSlot::<Nested>::new(&[0])
            .record_write(RecordingFrame::new(context(&initial)), bad.clone())
    );
    rejects!(
        SetSlot::<Nested>::new(&[1]).insert(context(&initial), bad.clone()),
        SetSlot::<Nested>::new(&[1])
            .record_insert(RecordingFrame::new(context(&initial)), bad.clone())
    );
    rejects!(
        SetSlot::<Nested>::new(&[1]).member(context(&initial), bad.clone()),
        SetSlot::<Nested>::new(&[1])
            .record_member(RecordingFrame::new(context(&initial)), bad.clone())
    );
    rejects!(
        SetSlot::<Nested>::new(&[1]).remove(context(&initial), bad.clone()),
        SetSlot::<Nested>::new(&[1])
            .record_remove(RecordingFrame::new(context(&initial)), bad.clone())
    );
    rejects!(
        MapSlot::<Nested, r::Field>::new(&[2]).insert(
            context(&initial),
            bad.clone(),
            r::Field::from(99_u64)
        ),
        MapSlot::<Nested, r::Field>::new(&[2]).record_insert(
            RecordingFrame::new(context(&initial)),
            bad.clone(),
            r::Field::from(99_u64)
        )
    );
    rejects!(
        MapSlot::<Nested, r::Field>::new(&[2]).member(context(&initial), bad.clone()),
        MapSlot::<Nested, r::Field>::new(&[2])
            .record_member(RecordingFrame::new(context(&initial)), bad.clone())
    );
    rejects!(
        MapSlot::<Nested, r::Field>::new(&[2]).lookup(context(&initial), bad.clone()),
        MapSlot::<Nested, r::Field>::new(&[2])
            .record_lookup(RecordingFrame::new(context(&initial)), bad.clone())
    );
    rejects!(
        MapSlot::<Nested, r::Field>::new(&[2]).remove(context(&initial), bad.clone()),
        MapSlot::<Nested, r::Field>::new(&[2])
            .record_remove(RecordingFrame::new(context(&initial)), bad)
    );
    assert_eq!(
        CellSlot::<Nested>::new(&[0]).inspect(&initial).unwrap(),
        canonical.clone()
    );
    assert!(
        SetSlot::<Nested>::new(&[1])
            .inspect(&initial)
            .unwrap()
            .try_member(canonical.clone())
            .unwrap()
    );
    assert_eq!(
        MapSlot::<Nested, r::Field>::new(&[2])
            .lookup(context(&initial), canonical)
            .unwrap()
            .result,
        r::Field::from(42_u64)
    );
}
#[test]
fn canonical_compound_key_removal_preserves_sibling_and_replays() {
    let selected = nested("last");
    let initial = initial(selected.clone());
    let removed = SetSlot::<Nested>::new(&[1])
        .remove(context(&initial), selected.clone())
        .unwrap();
    let native = MapSlot::<Nested, r::Field>::new(&[2])
        .remove(removed.context, selected.clone())
        .unwrap();
    let frame = SetSlot::<Nested>::new(&[1])
        .record_remove(RecordingFrame::new(context(&initial)), selected.clone())
        .unwrap();
    let recorded = MapSlot::<Nested, r::Field>::new(&[2])
        .record_remove(frame, selected.clone())
        .unwrap()
        .finish(());
    assert_eq!(
        recorded.execution.gas_cost,
        removed.gas_cost + native.gas_cost
    );
    assert_eq!(
        recorded.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(recorded.execution.context.private_state, 7);
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    let state = recorded.execution.context.query.state.get_ref();
    let set = SetSlot::<Nested>::new(&[1]).inspect(state).unwrap();
    assert!(!set.try_member(selected.clone()).unwrap());
    assert!(set.try_member(nested("sibling")).unwrap());
    let map = MapSlot::<Nested, r::Field>::new(&[2])
        .inspect(state)
        .unwrap();
    assert!(!map.try_member(selected).unwrap());
    assert_eq!(
        map.lookup(nested("sibling")).unwrap(),
        r::Field::from(90_u64)
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &native.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
}
