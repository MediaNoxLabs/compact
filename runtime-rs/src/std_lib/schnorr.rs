// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Module-1 (Schnorr) — Schnorr-on-Jubjub signature verification
//! exposed in a shape the compact codegen can call directly.
//!
//! The pinned `midnight-transient-crypto 2.1.0` does not yet expose the
//! `schnorr` module that midnight-ledger ships locally (the impl was
//! added post-2.1.0). To keep compact-runtime self-contained we vendor
//! ~50 LOC of the verifier here, then expose a circuit-shaped wrapper
//! (`schnorr_verify_jubjub`) that takes a `CircuitContext`, threads it
//! through a no-op `query_for_verify`, and surfaces the verification
//! result as a `CompactError::AssertionFailed` on rejection.
//!
//! When upstream `midnight-transient-crypto` exposes `schnorr` in a
//! future release the vendored bits can be deleted in favour of
//! `pub use midnight_transient_crypto::schnorr::*` and the wrapper
//! unchanged.
//!
//! Algorithm matches `jubjub-schnorr/src/schnorr.compact`'s
//! `schnorrVerify` and midnight-did's `computeJubjubDigestChallenge`
//! exactly: Poseidon over `[ann_x, ann_y, pk_x, pk_y, ...msg]`, reduce
//! the challenge modulo 2^248 (keep the low 31 little-endian bytes),
//! then check `g^s == announcement + pk^c`.

use midnight_transient_crypto::curve::{embedded, EmbeddedFr, Fr};
use midnight_transient_crypto::hash::transient_hash;

use crate::{
    query_for_verify, CircuitContext, CircuitResults, CompactError, DefaultDB, JubjubPoint,
    OpProgramVerify,
};

/// A Schnorr signature over the embedded curve. Layout matches the
/// Compact-side `Schnorr.SchnorrSignature` struct exactly
/// (`announcement: JubjubPoint`, `response: Field`) so the codegen's
/// generated user-struct lines up by name + field types and the
/// `schnorr_verify_jubjub` wrapper accepts both. The `response` field
/// is stored as the outer scalar `Fr` (matching Compact's `Field`); the
/// off-circuit verifier reduces it to `EmbeddedFr` modulo the Jubjub
/// scalar order before the group-arithmetic check (`fr_to_embedded_fr`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchnorrSignature {
    /// The announcement point, `R = k * G`.
    pub announcement: JubjubPoint,
    /// The response scalar, encoded as an outer-curve `Fr`. The
    /// off-circuit verifier reduces this modulo the Jubjub scalar
    /// field order before use.
    pub response: Fr,
}

impl crate::Aligned for SchnorrSignature {
    fn alignment() -> crate::Alignment {
        crate::Alignment::concat([
            &<crate::JubjubPoint as crate::Aligned>::alignment(),
            &<crate::Fr as crate::Aligned>::alignment(),
        ])
    }
}

impl From<SchnorrSignature> for crate::Value {
    fn from(sig: SchnorrSignature) -> crate::Value {
        crate::Value::concat(
            [
                crate::Value::from(sig.announcement),
                crate::Value::from(sig.response),
            ]
            .iter(),
        )
    }
}

/// Hash `(ann_x, ann_y, pk_x, pk_y, ...msg)` with the Poseidon-based
/// transient hash and reduce modulo 2^248, exactly as the Compact circuit
/// and midnight-did's `computeJubjubDigestChallenge` do. Since 2^248 is
/// smaller than the Jubjub scalar order, the low 31 little-endian bytes
/// inject canonically into `EmbeddedFr`.
fn compute_challenge(ann_x: Fr, ann_y: Fr, pk_x: Fr, pk_y: Fr, msg: &[Fr]) -> EmbeddedFr {
    let mut hash_input = Vec::with_capacity(4 + msg.len());
    hash_input.push(ann_x);
    hash_input.push(ann_y);
    hash_input.push(pk_x);
    hash_input.push(pk_y);
    hash_input.extend_from_slice(msg);
    let mut hash_bytes = transient_hash(&hash_input).as_le_bytes();
    hash_bytes.resize(32, 0);
    hash_bytes[31] = 0;
    EmbeddedFr::from_le_bytes(&hash_bytes).expect("2^248-truncated challenge is below Jubjub r")
}

/// Reduce an outer BLS12-381 scalar `Fr` modulo the Jubjub scalar order.
/// Schnorr responses are scalar-field values carried in Compact's wider
/// `Field` type, so response conversion uses mod-r reduction. This is
/// intentionally different from the challenge's mod-2^248 truncation.
fn fr_to_embedded_fr(fr: Fr) -> EmbeddedFr {
    let mut wide = [0u8; 64];
    wide[..32].copy_from_slice(&fr.as_le_bytes());
    EmbeddedFr(embedded::Scalar::from_bytes_wide(&wide))
}

/// Off-circuit Schnorr verifier. Returns `true` iff the signature is
/// valid for `(pk, msg)`. Identity public-key / announcement are
/// rejected up front.
///
/// The circuit-side verifier in `examples/schnorr_attest_fixture.compact`
/// now carries the same guard, but note the two are NOT exercised
/// together: the emitter rewrites a call to the generic `schnorrVerify`
/// into this function, so on the Rust path the circuit's own body — guard
/// included — never runs. The circuit guard governs proving and the
/// TypeScript path. See MediaNoxLabs/compact#26.
pub fn verify(pk: JubjubPoint, msg: &[Fr], sig: &SchnorrSignature) -> bool {
    if pk.is_identity() || sig.announcement.is_identity() {
        return false;
    }
    let pk_x = match pk.x() {
        Some(x) => x,
        None => return false,
    };
    let pk_y = match pk.y() {
        Some(y) => y,
        None => return false,
    };
    let ann_x = match sig.announcement.x() {
        Some(x) => x,
        None => return false,
    };
    let ann_y = match sig.announcement.y() {
        Some(y) => y,
        None => return false,
    };

    let challenge = compute_challenge(ann_x, ann_y, pk_x, pk_y, msg);
    // Compact's `SchnorrSignature.response` is declared `Field` (Fr) —
    // wider than the embedded scalar order. Reduce the response modulo r
    // before the group-arithmetic check. `getSchnorrReduction` concerns the
    // challenge's separate mod-2^248 reduction and is not used here.
    let response_embed = fr_to_embedded_fr(sig.response);
    let lhs = JubjubPoint::generator() * response_embed;
    let rhs = sig.announcement + pk * challenge;
    lhs == rhs
}

/// Circuit-shaped wrapper used by the compact codegen to replace
/// `self.schnorr_verify(ctx, msg, sig, pk)?` calls inside the
/// generated `schnorr_verify_digest` circuit body. Verifies the
/// signature, returns `Err(CompactError::AssertionFailed)` on
/// rejection, and otherwise threads `ctx` through a no-op
/// `query_for_verify` to produce a `CircuitResults<PS, ()>` shaped the
/// same way an inlined Compact assert body would.
pub fn schnorr_verify_jubjub<PS, const N: usize>(
    ctx: CircuitContext<PS>,
    msg: [Fr; N],
    sig: SchnorrSignature,
    pk: JubjubPoint,
) -> Result<CircuitResults<PS, ()>, CompactError>
where
    PS: Clone,
{
    if !verify(pk, &msg, &sig) {
        return Err(CompactError::AssertionFailed(
            "Schnorr signature verification failed".into(),
        ));
    }
    let ops = OpProgramVerify::<DefaultDB>::new().build();
    let results = query_for_verify(
        &ctx.current_query_context,
        &ops,
        ctx.gas_limit,
        &ctx.cost_model,
    )?;
    Ok(CircuitResults {
        result: (),
        context: CircuitContext {
            current_query_context: results.context,
            ..ctx
        },
        gas_cost: results.gas_cost,
    })
}
