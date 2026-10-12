// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_slots as slots, runtime as r, types};
use r::context::CircuitContext;
use r::recording::{RecordedCircuitResult, RecordingFrame};
pub type Wide = r::WideUint<1, { u128::MAX }>;
pub fn proof_value() -> types::Stored {
    let mut bytes = [0_u8; 17];
    bytes[0] = 9;
    bytes[16] = 1;
    value(Wide::from_le_bytes(&bytes).unwrap())
}
pub fn value(wide: Wide) -> types::Stored {
    types::Stored {
        stamp: r::BoundedUint::new(100).unwrap(),
        fields: r::FixedVector::new([3_u64.into(), 5_u64.into()]),
        pair: (7_u64.into(), true),
        wide,
    }
}
pub fn manual_read(
    context: CircuitContext<u64>,
    key: r::OpaqueString,
    as_of: r::BoundedUint<{ u64::MAX as u128 }>,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    let (frame, present) =
        slots::records.record_member(RecordingFrame::new(context), key.clone())?;
    if !present {
        return Err(r::CompactError::AssertionFailed(
            "record does not exist".into(),
        ));
    }
    let (frame, stored) = slots::records.record_lookup(frame, key)?;
    if stored.stamp.value() > as_of.value() {
        return Err(r::CompactError::AssertionFailed(
            "record is in the future".into(),
        ));
    }
    Ok(frame.finish(()))
}
