// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent composition of declared slots and official arithmetic/curve primitives.
use super::*;
pub(super) fn execute(
    root: Root,
    state: &ObservedContractState,
    provider: &Provider,
    digest: r::FixedVector<Field, 4>,
    signature: SchnorrSignature,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    let (frame, open) =
        slots::open.record_read(RecordingFrame::new(state.circuit_context(root.private())))?;
    assertion(open, "attestor is closed")?;
    let (frame, key) = slots::attestorKey.record_read(frame)?;
    // The source's named hash product has exactly these five fields in this order.
    let challenge = r::transient_hash((
        r::jubjub_point_x(signature.announcement),
        r::jubjub_point_y(signature.announcement),
        r::jubjub_point_x(key),
        r::jubjub_point_y(key),
        digest,
    ));
    let (frame, (quotient, remainder)) = frame.try_witness_metered(|ctx, meter| {
        let count = if provider.inspect_counter {
            Some(slots::acceptedCount.witness_read(meter)?)
        } else {
            None
        };
        provider.reduce(ctx.private_state, challenge, count)
    })?;
    assertion(quotient.value() < 116, "Schnorr quotient out of range")?;
    let mut power = [0_u8; 32];
    power[31] = 1;
    assertion(
        Field::from(quotient.value()) * Field::from_le_bytes(&power).unwrap()
            + remainder.as_field()
            == challenge,
        "Invalid challenge reduction",
    )?;
    let lhs = r::ec_mul_generator(signature.response)?;
    let rhs = r::ec_add(
        signature.announcement,
        r::ec_mul(key, remainder.as_field())?,
    );
    assertion(
        key != JubjubPoint::default() && signature.announcement != JubjubPoint::default(),
        "Schnorr verification requires a non-identity key and announcement",
    )?;
    assertion(
        r::jubjub_point_x(lhs) == r::jubjub_point_x(rhs)
            && r::jubjub_point_y(lhs) == r::jubjub_point_y(rhs),
        "Invalid Jubjub Schnorr signature",
    )?;
    let frame = if root.accepts() {
        slots::acceptedCount.record_increment(frame, 1)?
    } else {
        frame
    };
    Ok(frame.finish(()))
}
