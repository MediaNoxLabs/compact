// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent declared-slot composition, including helper assertions after the witness.
use super::*;
use compact_rust_did_digest_read_reducer_fixture::ledger_slots as slots;
pub(super) fn execute(
    observed: &ObservedContractState,
    private: u64,
    provider: &Provider,
    id: r::OpaqueString,
    expected: r::JubjubPoint,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    let (frame, active) =
        slots::active.record_read(RecordingFrame::new(observed.circuit_context(private)))?;
    assertion(active, "inactive")?;
    let (frame, present) = slots::methods.record_member(frame, id.clone())?;
    assertion(present, "missing")?;
    let (frame, method) = slots::methods.record_lookup(frame, id)?;
    let (frame, admit) = frame.try_witness_metered(|ctx, _| provider.answer(ctx.private_state))?;
    assertion(admit, "witness denied")?;
    assertion(
        r::jubjub_point_x(expected) == r::jubjub_point_x(method.verificationPoint),
        "wrong point",
    )?;
    assertion(
        r::jubjub_point_y(expected) == r::jubjub_point_y(method.verificationPoint),
        "wrong point",
    )?;
    Ok(frame.finish(()))
}
