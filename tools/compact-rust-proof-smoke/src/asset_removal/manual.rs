// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Independent source-order removal trace, using official ledger slots.
use super::*;
use runtime::context::CircuitContext;
use runtime::recording::{RecordedCircuitResult, RecordingFrame};

pub(super) fn remove(
    context: CircuitContext<u64>,
    key: runtime::OpaqueString,
    witness: &TimestampWitness,
) -> Result<RecordedCircuitResult<u64, ()>, Box<dyn Error>> {
    let (frame, open) = slots::open.record_read(RecordingFrame::new(context))?;
    if !open {
        return Err("registry is closed".into());
    }
    let (frame, frozen) = slots::frozen.record_read(frame)?;
    if frozen {
        return Err("registry is frozen".into());
    }
    let (frame, exists) = slots::records.record_member(frame, key.clone())?;
    if !exists {
        return Err("record does not exist".into());
    }
    let (frame, watched) = slots::watchList.record_member(frame, key.clone())?;
    if watched {
        return Err("record is still watched".into());
    }
    let frame = slots::records.record_remove(frame, key.clone())?;
    let frame = slots::retiredKeys.record_insert(frame, key)?;
    let frame = slots::writeCount.record_increment(frame, 1)?;
    let frame = slots::revision.record_increment(frame, 1)?;
    let (frame, timestamp) = frame.try_witness_metered(|context, meter| {
        witness.0.borrow_mut().push(context.private_state);
        slots::recordCount.witness_read(meter)?;
        slots::records.witness_view(meter)?;
        slots::watchList.witness_view(meter)?;
        Ok((
            context.private_state + 1,
            BoundedUint::<{ u64::MAX as u128 }>::new(1_700_000_000)?,
        ))
    })?;
    Ok(slots::updatedAt.record_write(frame, timestamp)?.finish(()))
}
