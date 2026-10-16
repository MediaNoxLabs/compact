// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Runtime codec laws, not source-level admission of the top-level Unit type.
use super::*;
use midnight_compact_runtime as r;
use r::context::{CircuitContext, ConstructorContext, ConstructorResult, RunningCost};
use r::ledger::{ChargedState, StateValue};
use r::recording::RecordingFrame;
use r::slots::{CellSlot, MapSlot, SetSlot};
use std::fmt::Debug;

fn context(state: StateValue<DefaultDB>) -> CircuitContext<u64> {
    ConstructorResult::new(ConstructorContext::new(7), ChargedState::new(state))
        .into_circuit_context(Default::default())
}
fn lifecycle<T: CellValue + Clone + Debug + PartialEq>(key: T) {
    let initial = StateValue::Array(
        vec![
            ledger::try_constructor_cell(key.clone()).unwrap(),
            ledger::constructor_set(),
            ledger::constructor_map(),
            ledger::constructor_cell(r::Field::from(77_u64)),
        ]
        .into(),
    );
    let mut gas = RunningCost::default();
    let write = CellSlot::<T>::new(&[0])
        .write(context(initial.clone()), key.clone())
        .unwrap();
    gas += write.gas_cost;
    let read = CellSlot::<T>::new(&[0]).read(write.context).unwrap();
    gas += read.gas_cost;
    assert_eq!(read.result, key);
    let insert = SetSlot::<T>::new(&[1])
        .insert(read.context, key.clone())
        .unwrap();
    gas += insert.gas_cost;
    let present = SetSlot::<T>::new(&[1])
        .member(insert.context, key.clone())
        .unwrap();
    gas += present.gas_cost;
    assert!(present.result);
    let remove = SetSlot::<T>::new(&[1])
        .remove(present.context, key.clone())
        .unwrap();
    gas += remove.gas_cost;
    let absent = SetSlot::<T>::new(&[1])
        .member(remove.context, key.clone())
        .unwrap();
    gas += absent.gas_cost;
    assert!(!absent.result);
    let insert = MapSlot::<T, r::Field>::new(&[2])
        .insert(absent.context, key.clone(), r::Field::from(42_u64))
        .unwrap();
    gas += insert.gas_cost;
    let found = MapSlot::<T, r::Field>::new(&[2])
        .lookup(insert.context, key.clone())
        .unwrap();
    gas += found.gas_cost;
    assert_eq!(found.result, r::Field::from(42_u64));
    let remove = MapSlot::<T, r::Field>::new(&[2])
        .remove(found.context, key.clone())
        .unwrap();
    gas += remove.gas_cost;
    let native = MapSlot::<T, r::Field>::new(&[2])
        .member(remove.context, key.clone())
        .unwrap();
    gas += native.gas_cost;
    assert!(!native.result);

    let frame = CellSlot::<T>::new(&[0])
        .record_write(RecordingFrame::new(context(initial.clone())), key.clone())
        .unwrap();
    let (frame, value) = CellSlot::<T>::new(&[0]).record_read(frame).unwrap();
    assert_eq!(value, key);
    let frame = SetSlot::<T>::new(&[1])
        .record_insert(frame, key.clone())
        .unwrap();
    let (frame, present) = SetSlot::<T>::new(&[1])
        .record_member(frame, key.clone())
        .unwrap();
    assert!(present);
    let frame = SetSlot::<T>::new(&[1])
        .record_remove(frame, key.clone())
        .unwrap();
    let (frame, present) = SetSlot::<T>::new(&[1])
        .record_member(frame, key.clone())
        .unwrap();
    assert!(!present);
    let frame = MapSlot::<T, r::Field>::new(&[2])
        .record_insert(frame, key.clone(), r::Field::from(42_u64))
        .unwrap();
    let (frame, value) = MapSlot::<T, r::Field>::new(&[2])
        .record_lookup(frame, key.clone())
        .unwrap();
    assert_eq!(value, r::Field::from(42_u64));
    let frame = MapSlot::<T, r::Field>::new(&[2])
        .record_remove(frame, key.clone())
        .unwrap();
    let (frame, present) = MapSlot::<T, r::Field>::new(&[2])
        .record_member(frame, key)
        .unwrap();
    let recorded = frame.finish(present);
    assert!(!present);
    assert_eq!(
        recorded.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(recorded.execution.gas_cost, gas);
    assert_eq!(recorded.execution.context.private_state, 7);
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(recorded.execution.context.query.state.get_ref(), &initial);
    // Zero-atom types have only one encoded key; the independently populated
    // sibling slot protects unrelated storage without inventing a second key.
    assert_eq!(
        CellSlot::<r::Field>::new(&[3])
            .inspect(recorded.execution.context.query.state.get_ref())
            .unwrap(),
        r::Field::from(77_u64)
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
}
#[test]
fn zero_atom_codecs_execute_cell_set_and_map_lifecycles() {
    lifecycle(Empty {});
    lifecycle(());
    lifecycle(FixedVector::<r::Field, 0>::new([]));
}
#[derive(Clone, Debug, PartialEq, Eq, CompactCellValue)]
struct UnitMembers {
    before: (),
    value: (r::Field, ()),
    after: FixedVector<(), 2>,
}
#[test]
fn unit_members_and_tuple8_keep_atom_boundaries() {
    let value = UnitMembers {
        before: (),
        value: (r::Field::from(19_u64), ()),
        after: FixedVector::new([(), ()]),
    };
    round_trip(value.clone());
    lifecycle(value.clone());
    let encoded: Value = value.clone().into();
    assert_eq!(encoded.0.len(), 1);
    assert_eq!(UnitMembers::decode_cell_value(&encoded).unwrap(), value);
    assert!(UnitMembers::decode_cell_value(&Value::from(())).is_err());
    type Eight = (
        r::Field,
        r::Field,
        r::Field,
        r::Field,
        r::Field,
        r::Field,
        r::Field,
        r::Field,
    );
    let eight: Eight = (
        0_u64.into(),
        1_u64.into(),
        2_u64.into(),
        3_u64.into(),
        4_u64.into(),
        5_u64.into(),
        6_u64.into(),
        7_u64.into(),
    );
    round_trip(eight);
    let encoded: Value = eight.into();
    assert_eq!(encoded.0.len(), 8);
    assert_eq!(Eight::decode_cell_value(&encoded).unwrap(), eight);
    assert!(Eight::decode_cell_value(&encoded[..7]).is_err());
    let mut surplus = encoded;
    surplus.0.extend(Value::from(true).0);
    assert!(Eight::decode_cell_value(&surplus).is_err());
    let nonempty = Value::from(true);
    assert!(Empty::decode_cell_value(&nonempty).is_err());
    assert!(<()>::decode_cell_value(&nonempty).is_err());
    assert!(FixedVector::<r::Field, 0>::decode_cell_value(&nonempty).is_err());
}
