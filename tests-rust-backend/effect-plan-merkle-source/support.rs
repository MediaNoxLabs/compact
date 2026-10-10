// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{BoundedUint, CompactError, FixedBytes};
pub type Leaf = BoundedUint<255>;
pub type Index = BoundedUint<{ u64::MAX as u128 }>;
pub type Context = CircuitContext<u64>;
pub fn context() -> Context {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
pub fn root(context: &Context) -> types::MerkleTreeDigest {
    types::MerkleTreeDigest {
        field: slots::plain
            .inspect(context.query.state.get_ref())
            .unwrap()
            .root()
            .unwrap()
            .0,
    }
}
fn observe_frame(
    frame: RecordingFrame<u64>,
    root: types::MerkleTreeDigest,
) -> Result<RecordedCircuitResult<u64, types::Snapshot>, CompactError> {
    let (frame, plain_full) = slots::plain.record_is_full(frame)?;
    let (frame, historic_full) = slots::historic.record_is_full(frame)?;
    let (frame, plain_known) = slots::plain.record_check_root(frame, root.clone())?;
    let (frame, historic_known) = slots::historic.record_check_root(frame, root)?;
    Ok(frame.finish(types::Snapshot {
        plain_full,
        historic_full,
        plain_known,
        historic_known,
    }))
}
pub fn observe(
    ctx: Context,
    root: types::MerkleTreeDigest,
) -> Result<RecordedCircuitResult<u64, types::Snapshot>, CompactError> {
    observe_frame(RecordingFrame::new(ctx), root)
}
pub fn run(
    ctx: Context,
    value: Leaf,
    root: types::MerkleTreeDigest,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, types::Snapshot>, CompactError> {
    let frame = slots::plain.record_insert(RecordingFrame::new(ctx), value)?;
    let frame = if selected {
        slots::historic.record_insert(frame, value)?
    } else {
        frame
    };
    observe_frame(frame, root)
}
pub fn place(
    ctx: Context,
    value: Leaf,
    index: Index,
) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    let frame = slots::plain.record_insert_index(RecordingFrame::new(ctx), value, index)?;
    let frame = slots::historic.record_insert_index(frame, value, index)?;
    Ok(frame.finish(()))
}
pub fn hashes(
    ctx: Context,
    hash: FixedBytes<32>,
    index: Index,
) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    let frame = slots::plain.record_insert_hash(RecordingFrame::new(ctx), hash)?;
    let frame = slots::historic.record_insert_hash(frame, hash)?;
    let frame = slots::plain.record_insert_hash_index(frame, hash, index)?;
    let frame = slots::historic.record_insert_hash_index(frame, hash, index)?;
    Ok(frame.finish(()))
}
pub fn defaults(
    ctx: Context,
    index: Index,
) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    let frame = slots::plain.record_insert_index_default(RecordingFrame::new(ctx), index)?;
    let frame = slots::historic.record_insert_index_default(frame, index)?;
    Ok(frame.finish(()))
}
pub fn forget(ctx: Context) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    Ok(slots::historic
        .record_reset_history(RecordingFrame::new(ctx))?
        .finish(()))
}
pub fn reset(ctx: Context) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    let frame = slots::plain.record_reset_to_default(RecordingFrame::new(ctx))?;
    Ok(slots::historic.record_reset_to_default(frame)?.finish(()))
}
