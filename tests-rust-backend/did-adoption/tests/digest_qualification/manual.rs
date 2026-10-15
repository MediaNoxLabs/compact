// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Declared read-only Map operations followed by independent official Schnorr arithmetic.
use super::*;
use compact_rust_did_adoption_fixture::ledger_slots as slots;
pub(super) fn execute(
    observed: &ObservedContractState,
    private: u64,
    provider: &Provider,
    id: r::OpaqueString,
    digest: r::FixedVector<r::Field, 4>,
    signature: types::SchnorrSignature,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    let (frame, active) =
        slots::active.record_read(RecordingFrame::new(observed.circuit_context(private)))?;
    assertion(active, "Contract is not active")?;
    let (frame, present) =
        slots::schnorrJubjubVerificationMethods.record_member(frame, id.clone())?;
    assertion(present, "Verification method does not exist")?;
    let (frame, method) = slots::schnorrJubjubVerificationMethods.record_lookup(frame, id)?;
    let key = method.publicKey;
    // This ordered tuple is the source SchnorrHashInput's five-field FAB layout.
    let challenge = r::transient_hash((
        r::jubjub_point_x(signature.announcement),
        r::jubjub_point_y(signature.announcement),
        r::jubjub_point_x(key),
        r::jubjub_point_y(key),
        digest,
    ));
    let (frame, (q, remainder)) =
        frame.try_witness_metered(|ctx, _| provider.reduce(ctx.private_state, challenge))?;
    assertion(q.value() < 116, "Schnorr quotient out of range")?;
    let mut power = [0; 32];
    power[31] = 1;
    assertion(
        r::Field::from(q.value()) * r::Field::from_le_bytes(&power).unwrap() + remainder.as_field()
            == challenge,
        "Invalid challenge reduction",
    )?;
    let lhs = r::ec_mul_generator(signature.response)?;
    let rhs = r::ec_add(
        signature.announcement,
        r::ec_mul(key, remainder.as_field())?,
    );
    assertion(
        r::jubjub_point_x(lhs) == r::jubjub_point_x(rhs)
            && r::jubjub_point_y(lhs) == r::jubjub_point_y(rhs),
        "Invalid Jubjub Schnorr signature",
    )?;
    Ok(frame.finish(()))
}
