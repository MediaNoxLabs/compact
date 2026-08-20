// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(clippy::unit_arg)]
//
// Executing gate for examples/schnorr_attest_fixture.compact.
//
// The Compact-side verifier is GENERIC (`schnorrVerify<#n>`), and the
// body lowering for a generic impure circuit is not supported, so the
// emitter does not lower it: `impure-call-target` in
// compiler/rust-passes-helpers.ss rewrites the call to
// `compact_runtime::schnorr_verify_jubjub`, and `stdlib-struct-mappings`
// routes the Compact `SchnorrSignature` type to the runtime's mirror
// struct so the rewritten call site type-checks. Both rewrites are keyed
// on NAMES, which byte-parity can only confirm textually.
//
// So run it. These tests sign a digest with a real Jubjub key off-circuit
// and push the signature through the generated circuit: a valid signature
// must be accepted (and the ledger write commit), a tampered one must be
// rejected with the runtime's message. That proves the rewrite produced a
// call that is not merely well-typed but semantically wired to the right
// key, message and signature — a swapped argument would still compile.

use compact_contract_schnorr_attest_fixture::{ledger, pure_circuits, Contract, Ledger, Witnesses};
use compact_runtime::transient_crypto::curve::{embedded, EmbeddedFr};
use compact_runtime::*;

/// Deterministic stub witnesses.
///
/// `get_schnorr_reduction` is declared by the Compact module but never
/// reached: the module body is not lowered, so the generated code calls
/// the runtime verifier instead of the in-circuit reduction. It is
/// stubbed to satisfy the trait — and its presence in the trait is
/// itself the check that a TUPLE-returning witness lowers to a Rust
/// tuple return type.
struct StubWitnesses;

impl Witnesses<()> for StubWitnesses {
    fn get_schnorr_reduction<'a>(
        &self,
        _ctx: &WitnessContext<Ledger<'a>, ()>,
        _challenge_hash: Fr,
    ) -> ((), (u8, u128)) {
        ((), (0u8, 0u128))
    }

    fn local_attestor_key<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>) -> ((), JubjubPoint) {
        ((), JubjubPoint::generator() * secret_key())
    }
}

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

fn contract() -> Contract<(), StubWitnesses> {
    Contract::new(StubWitnesses)
}

fn secret_key() -> EmbeddedFr {
    EmbeddedFr(embedded::Scalar::from(0x5eed_u64))
}

fn nonce() -> EmbeddedFr {
    EmbeddedFr(embedded::Scalar::from(0x00c0_ffee_u64))
}

/// Reduce a BLS12-381 scalar into the Jubjub scalar field, exactly as
/// `compact_runtime`'s off-circuit verifier does.
fn fr_to_embedded(fr: Fr) -> EmbeddedFr {
    let mut wide = [0u8; 64];
    wide[..32].copy_from_slice(&fr.as_le_bytes());
    EmbeddedFr(embedded::Scalar::from_bytes_wide(&wide))
}

/// `H(ann_x, ann_y, pk_x, pk_y, ...msg)` reduced into the Jubjub scalar
/// field — the challenge both the circuit and the verifier compute.
fn challenge(ann: JubjubPoint, pk: JubjubPoint, msg: &[Fr]) -> EmbeddedFr {
    let mut input = vec![
        ann.x().expect("announcement x"),
        ann.y().expect("announcement y"),
        pk.x().expect("public key x"),
        pk.y().expect("public key y"),
    ];
    input.extend_from_slice(msg);
    fr_to_embedded(transient_hash(&input))
}

/// Produce a valid Schnorr signature over `msg`: `R = k*G`,
/// `s = k + c*sk`. `response` is carried as an outer-curve `Fr` because
/// that is how Compact declares the field; the verifier reduces it back.
fn sign(msg: &[Fr]) -> SchnorrSignature {
    let sk = secret_key();
    let pk = JubjubPoint::generator() * sk;
    let k = nonce();
    let announcement = JubjubPoint::generator() * k;
    let c = challenge(announcement, pk, msg);
    let s = k.0 + c.0 * sk.0;
    let response = Fr::from_le_bytes(&s.to_bytes()).expect("jubjub scalar fits in Fr");
    SchnorrSignature {
        announcement,
        response,
    }
}

fn digest() -> [Fr; 4] {
    let mut subject = [0u8; 32];
    subject[..5].copy_from_slice(b"lot-1");
    pure_circuits::attestation_digest(subject, 7u64, Fr::from(99u64)).expect("attestation_digest")
}

/// The constructor writes the attestor key from a witness; the accessor
/// must read the same point back.
#[test]
fn initial_state_binds_the_attestor_key() {
    let result = contract().initial_state(ctor_ctx()).expect("initial_state");
    let view = ledger(&result.current_contract_state);

    assert_eq!(
        view.attestor_key().expect("attestor_key"),
        JubjubPoint::generator() * secret_key(),
    );
    assert!(view.open().expect("open"));
    assert_eq!(view.accepted_count().expect("accepted_count"), 0u64);
}

/// The whole point of the fixture: a genuine signature over the digest
/// must be accepted by the rewritten call. A rewrite that passed the
/// wrong key, the wrong message, or a defaulted signature would still
/// compile — and would fail here.
#[test]
fn valid_signature_is_accepted_and_counted() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let msg = digest();
    let after = contract
        .verify_attestation(ctx, msg, sign(&msg))
        .expect("a valid signature must verify");

    // The mutating sibling runs the same rewritten call and then commits
    // a ledger write, so the routing has to leave the context usable.
    let after = contract
        .accept_attestation(after.context, msg, sign(&msg))
        .expect("a valid signature must be accepted");
    let view = ledger(&after.context.current_query_context.state);
    assert_eq!(view.accepted_count().expect("accepted_count"), 1u64);
}

/// A signature over a DIFFERENT digest must be rejected — this is what
/// proves the message really reaches the verifier rather than being
/// dropped by the rewrite.
#[test]
fn signature_over_another_message_is_rejected() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let mut other = digest();
    other[2] = Fr::from(1234u64);
    let stale = sign(&other);

    #[allow(clippy::err_expect)]
    let err = contract
        .verify_attestation(ctx, digest(), stale)
        .err()
        .expect("a signature over another message must be rejected");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "Schnorr signature verification failed"
        ),
        "expected the Schnorr rejection message, got {err:?}"
    );
}

/// Tampering with the response scalar must also be rejected.
#[test]
fn tampered_response_is_rejected() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let msg = digest();
    let mut sig = sign(&msg);
    sig.response = Fr(sig.response.0 + Fr::from(1u64).0);

    #[allow(clippy::err_expect)]
    let err = contract
        .verify_attestation(ctx, msg, sig)
        .err()
        .expect("a tampered response must be rejected");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "Schnorr signature verification failed"
        ),
        "expected the Schnorr rejection message, got {err:?}"
    );
}

/// The exported pure circuit is deterministic and domain-separated: the
/// same inputs give the same digest, a different epoch a different one.
#[test]
fn attestation_digest_is_deterministic_and_epoch_separated() {
    let mut subject = [0u8; 32];
    subject[..5].copy_from_slice(b"lot-1");

    let a = pure_circuits::attestation_digest(subject, 7u64, Fr::from(99u64)).expect("digest a");
    let b = pure_circuits::attestation_digest(subject, 7u64, Fr::from(99u64)).expect("digest b");
    assert_eq!(a, b, "the digest must be a pure function of its inputs");

    let c = pure_circuits::attestation_digest(subject, 8u64, Fr::from(99u64)).expect("digest c");
    assert_ne!(a, c, "a different epoch must change the digest");
}
