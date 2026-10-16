// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent official-slot programs and explicit source-layout initial states.
use crate::{ledger_slots as slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext, ConstructorResult};
use r::fab::{Aligned, AlignedValue, Value};
use r::ledger::{ChargedState, DefaultDB, StateValue, constructor_cell, constructor_set};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field};
#[derive(Clone, Copy)]
pub enum Fault {
    None,
    ScalarContainer,
    StoredText,
}
pub fn key(populated: bool) -> types::OwnedKey {
    if populated {
        types::OwnedKey {
            text: "key\0ok".into(),
            bytes: vec![0, 1, 0, 255].into(),
            marker: Field::from(11_u64),
        }
    } else {
        Default::default()
    }
}
pub fn context(populated: bool, fault: Fault) -> CircuitContext<u64> {
    let scalar = if matches!(fault, Fault::ScalarContainer) {
        constructor_cell::<_, DefaultDB>(false)
    } else {
        constructor_set::<DefaultDB>()
    };
    let stored = if matches!(fault, Fault::StoredText) {
        let mut value: Value = key(populated).into();
        value.0[0].0 = vec![0xff];
        StateValue::from(AlignedValue::new(value, types::OwnedKey::alignment()).unwrap())
    } else {
        constructor_cell::<_, DefaultDB>(key(populated))
    };
    let mut rest = vec![
        constructor_set::<DefaultDB>(),
        constructor_set::<DefaultDB>(),
        stored,
        constructor_cell::<_, DefaultDB>(Field::from(0_u64)),
    ];
    rest.extend((0..11).map(|i| {
        constructor_cell::<_, DefaultDB>(Field::from(if populated { 100_u64 + i } else { 0 }))
    }));
    ConstructorResult::new(
        ConstructorContext::new(7),
        ChargedState::new(StateValue::Array(
            vec![
                StateValue::Array(vec![scalar].into()),
                StateValue::Array(rest.into()),
            ]
            .into(),
        )),
    )
    .into_circuit_context(Default::default())
}
pub fn reference(
    ctx: CircuitContext<u64>,
    seed: Field,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, types::Receipt>, CompactError> {
    let (frame, scalar_before) = slots::scalars.record_member(RecordingFrame::new(ctx), seed)?;
    let frame = slots::scalars.record_insert(frame, seed)?;
    let (frame, scalar_after) = slots::scalars.record_member(frame, seed)?;
    let (frame, key) = if selected {
        slots::storedKey.record_read(frame)?
    } else {
        (frame, types::OwnedKey::default())
    };
    let (frame, owned_before) = slots::keys.record_member(frame, key.clone())?;
    let (frame, name_before) = slots::names.record_member(frame, key.text.clone())?;
    let frame = if selected {
        let frame = slots::keys.record_insert(frame, key.clone())?;
        slots::keys.record_insert(frame, key.clone())?
    } else {
        frame
    };
    let frame = slots::names.record_insert(frame, key.text.clone())?;
    let (frame, owned_after) = slots::keys.record_member(frame, key.clone())?;
    let (frame, name_after) = slots::names.record_member(frame, key.text)?;
    let frame = slots::outcome.record_write(frame, seed)?;
    let (frame, marker) = slots::outcome.record_read(frame)?;
    Ok(frame.finish(types::Receipt {
        scalarBefore: scalar_before,
        scalarAfter: scalar_after,
        ownedBefore: owned_before,
        ownedAfter: owned_after,
        nameBefore: name_before,
        nameAfter: name_after,
        marker,
    }))
}
fn check(value: bool, message: &str) -> Result<(), CompactError> {
    if value {
        Ok(())
    } else {
        Err(CompactError::AssertionFailed(message.into()))
    }
}
pub fn lifecycle_reference(
    ctx: CircuitContext<u64>,
    seed: Field,
) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    let frame = slots::scalars.record_reset(RecordingFrame::new(ctx))?;
    let (frame, empty) = slots::scalars.record_is_empty(frame)?;
    check(empty, "reset must be empty")?;
    let frame = slots::scalars.record_insert(frame, seed)?;
    let frame = slots::scalars.record_insert(frame, seed)?;
    let (frame, size) = slots::scalars.record_size(frame)?;
    check(size == 1, "duplicate changes size")?;
    let frame = slots::scalars.record_remove(frame, seed)?;
    let frame = slots::scalars.record_remove(frame, seed)?;
    let (frame, empty) = slots::scalars.record_is_empty(frame)?;
    check(empty, "removal must be empty")?;
    let frame = slots::scalars.record_insert(frame, seed)?;
    let frame = slots::scalars.record_reset(frame)?;
    let (frame, size) = slots::scalars.record_size(frame)?;
    check(size == 0, "final reset changes size")?;
    Ok(frame.finish(()))
}
