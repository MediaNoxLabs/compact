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
//! `schnorrVerify` exactly: Poseidon over
//! `[ann_x, ann_y, pk_x, pk_y, ...msg]` then reduce modulo the Jubjub
//! scalar order → check `g^s == announcement + pk^c`.

use midnight_transient_crypto::curve::{EmbeddedFr, Fr};
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

/// Hash `(ann_x, ann_y, pk_x, pk_y, ...msg)` with the Poseidon-based
/// transient hash and reduce modulo the Jubjub scalar field order.
fn compute_challenge(ann_x: Fr, ann_y: Fr, pk_x: Fr, pk_y: Fr, msg: &[Fr]) -> EmbeddedFr {
    let mut hash_input = Vec::with_capacity(4 + msg.len());
    hash_input.push(ann_x);
    hash_input.push(ann_y);
    hash_input.push(pk_x);
    hash_input.push(pk_y);
    hash_input.extend_from_slice(msg);
    let hash = transient_hash(&hash_input);
    fr_to_embedded_fr(hash)
}

/// Reduce a BLS12-381 scalar `Fr` modulo the Jubjub scalar field order,
/// i.e. `x mod r_jubjub`. Single implementation lives in
/// [`crate::std_lib::jubjub_scalar_from_field`], which is also what
/// Compact's `as JubjubScalar` cast lowers to.
///
/// # Warning: this is NOT the vendored circuit's challenge reduction
///
/// The doc comment this replaced claimed the function "mirrors what the
/// matching circuit does via the `getSchnorrReduction` witness". It does
/// not. The vendored `examples/did-05/jubjub-schnorr/src/schnorr.compact`
/// derives its challenge by **248-bit truncation** — `cFull = q·2^248 +
/// cTruncated` with `c = cTruncated`, i.e. `cFull mod 2^248` — whereas
/// this is `cFull mod r_jubjub`. Since `2^248 < r_jubjub` the two differ
/// whenever `cFull >= 2^248`, which is essentially always, so
/// [`schnorr_verify_jubjub`] and that circuit disagree. No test currently
/// covers the gap (`did-05`'s tests are constructor-scaffold/readback
/// only). The 0.33 standard library's `jubjubSchnorrVerify` uses the
/// mod-`r` form, so this function matches the STDLIB semantics and a
/// coordinated migration off the vendored module is the eventual fix.
/// Tracked on MediaNoxLabs/compact#17.
fn fr_to_embedded_fr(fr: Fr) -> EmbeddedFr {
    crate::std_lib::jubjub_scalar_from_field(fr)
}

/// Off-circuit Schnorr verifier. Returns `true` iff the signature is
/// valid for `(pk, msg)`. Identity public-key / announcement are
/// rejected up front, matching the circuit's identity guards.
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
    // wider than the embedded scalar order. Reduce before the
    // group-arithmetic check, matching what the in-circuit
    // `getSchnorrReduction` witness would expose as (q, r).
    let response_embed = fr_to_embedded_fr(sig.response);
    let lhs = JubjubPoint::generator() * response_embed;
    let rhs = sig.announcement + pk * challenge;
    lhs == rhs
}

/// A Schnorr signature over the JubJub curve — the Rust mirror of the
/// 0.33 standard library's `JubjubSchnorrSignature` struct
/// (`announcement: JubjubPoint`, `response: Field`). Field layout and
/// order match the Compact struct exactly so codegen-constructed
/// values line up; the codegen's stdlib-struct mapping routes the
/// Compact type here instead of emitting its own declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JubjubSchnorrSignature {
    /// The announcement point, `R = k * G`.
    pub announcement: JubjubPoint,
    /// The response scalar, encoded as an outer-curve `Fr` (Compact
    /// `Field`). Reduced modulo the JubJub scalar order before use,
    /// matching the stdlib circuit's `response as JubjubScalar` cast.
    pub response: Fr,
}

/// Pure-circuit-shaped verifier used by the compact codegen to replace
/// calls to the 0.33 standard library's `jubjubSchnorrVerify<#N>`
/// circuit. Mirrors the stdlib body exactly:
///
/// - challenge `c = transientHash(annX, annY, pkX, pkY, msg...)`
///   reduced into the JubJub scalar field (the stdlib's
///   `cNative as JubjubScalar` cast — plain mod-r reduction);
/// - `response as JubjubScalar` — same reduction;
/// - valid iff `s·G == R + c·pk`.
///
/// Identity points contribute zero coordinates to the hash (the
/// `jubjubPointX`/`jubjubPointY` semantics), with no up-front identity
/// rejection — exactly like the stdlib circuit.
pub fn jubjub_schnorr_verify<const N: usize>(
    msg: [Fr; N],
    signature: JubjubSchnorrSignature,
    pk: JubjubPoint,
) -> bool {
    let ann_x = signature.announcement.x().unwrap_or_else(|| Fr::from(0u64));
    let ann_y = signature.announcement.y().unwrap_or_else(|| Fr::from(0u64));
    let pk_x = pk.x().unwrap_or_else(|| Fr::from(0u64));
    let pk_y = pk.y().unwrap_or_else(|| Fr::from(0u64));

    let c = compute_challenge(ann_x, ann_y, pk_x, pk_y, &msg);

    let lhs = JubjubPoint::generator() * fr_to_embedded_fr(signature.response);
    let rhs = signature.announcement + pk * c;
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
