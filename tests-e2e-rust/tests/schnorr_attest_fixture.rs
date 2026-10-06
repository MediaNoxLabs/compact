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
// `midnight_compact_runtime::schnorr_verify_jubjub`, and `stdlib-struct-mappings`
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
use midnight_compact_runtime::transient_crypto::curve::{embedded, EmbeddedFr};
use midnight_compact_runtime::*;

/// Deterministic witnesses for the generated Rust execution path.
///
/// `get_schnorr_reduction` is declared by the Compact module but never
/// reached: the generic module body is not lowered, so the generated code
/// calls the runtime verifier instead of the in-circuit reduction. The
/// generated trait currently spells the Compact `[Uint<7>, Uint<248>]`
/// result as `(u8, u128)`, which cannot carry the second component losslessly.
/// That API mismatch is a dead surface, not a truncation strategy: this
/// implementation panics so every executing test proves the witness stayed
/// unreachable.
struct StubWitnesses;

impl Witnesses<()> for StubWitnesses {
    fn get_schnorr_reduction<'a>(
        &self,
        _ctx: &WitnessContext<Ledger<'a>, ()>,
        _challenge_hash: Fr,
    ) -> ((), (u8, u128)) {
        panic!("dead get_schnorr_reduction witness was invoked")
    }

    fn local_attestor_key<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>) -> ((), JubjubPoint) {
        ((), JubjubPoint::generator() * secret_key())
    }
}

/// Witnesses whose attestor key is the IDENTITY point.
///
/// This is not a contrived value: `JubjubPoint::default()` IS the identity,
/// and a ledger cell holds its type's default until written, so this stands
/// in for a contract whose key cell was never meaningfully set.
struct IdentityKeyWitnesses;

impl Witnesses<()> for IdentityKeyWitnesses {
    fn get_schnorr_reduction<'a>(
        &self,
        _ctx: &WitnessContext<Ledger<'a>, ()>,
        _challenge_hash: Fr,
    ) -> ((), (u8, u128)) {
        panic!("dead get_schnorr_reduction witness was invoked")
    }

    fn local_attestor_key<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>) -> ((), JubjubPoint) {
        ((), JubjubPoint::default())
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
/// `midnight_compact_runtime`'s off-circuit verifier does.
fn fr_to_embedded(fr: Fr) -> EmbeddedFr {
    let mut wide = [0u8; 64];
    wide[..32].copy_from_slice(&fr.as_le_bytes());
    EmbeddedFr(embedded::Scalar::from_bytes_wide(&wide))
}

/// The full-width `H(ann_x, ann_y, pk_x, pk_y, ...msg)` value, before
/// reduction into the Jubjub scalar field.
fn challenge_hash(ann: JubjubPoint, pk: JubjubPoint, msg: &[Fr]) -> Fr {
    let mut input = vec![
        ann.x().expect("announcement x"),
        ann.y().expect("announcement y"),
        pk.x().expect("public key x"),
        pk.y().expect("public key y"),
    ];
    input.extend_from_slice(msg);
    transient_hash(&input)
}

/// The challenge both the circuit and the verifier compute.
fn challenge(ann: JubjubPoint, pk: JubjubPoint, msg: &[Fr]) -> EmbeddedFr {
    fr_to_embedded(challenge_hash(ann, pk, msg))
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
///
/// The fixed key, nonce, and message also pin a challenge with significant
/// bytes above bit 128. This is the executable regression for the dead
/// `Uint<248>` witness surface: the native verifier must consume the full
/// transient hash rather than a generated `u128` or any truncation of it.
#[test]
fn valid_wide_challenge_signature_is_accepted_and_counted() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    let msg = digest();
    let signature = sign(&msg);
    let public_key = JubjubPoint::generator() * secret_key();
    let challenge_bytes = challenge_hash(signature.announcement, public_key, &msg).as_le_bytes();
    assert_eq!(
        hex::encode(&challenge_bytes),
        "84d8175a6134f71644c7a1239c707ef5566f52986f9ad7d55a08114362e9186e",
        "deterministic transient-hash challenge changed"
    );
    assert!(
        challenge_bytes[16..].iter().any(|byte| *byte != 0),
        "fixture challenge must not fit in 128 bits"
    );

    let after = contract
        .verify_attestation(ctx, msg, signature)
        .expect("a valid full-width-challenge signature must verify");

    // PR #372 must retain the exact root input and must not invent a private
    // output for the dead reduction witness. The trace also contains the
    // generated local schnorr_verify_digest wrapper before the root call.
    let calls = after.context.call_proof_data_trace.as_slice();
    assert_eq!(
        calls
            .iter()
            .map(|call| call.circuit_id.as_str())
            .collect::<Vec<_>>(),
        ["schnorr_verify_digest", "verify_attestation"]
    );
    assert!(calls
        .iter()
        .all(|call| call.proof_data.private_transcript_outputs().is_empty()));
    let root_call = calls.last().expect("root proof data");
    assert_eq!(
        root_call.proof_data.input,
        aligned_value_from_parts(&[proof_aligned_array(&msg), proof_aligned_value(&signature),])
    );
    assert_eq!(root_call.proof_data.output, aligned_value_from_parts(&[]));
    assert!(
        matches!(
            after.context.call_proof_data_trace.single_contract_call(),
            Err(CompactError::ProofData(_))
        ),
        "nested local-call proof data must fail closed until it is folded into the root call"
    );

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

/// An identity public key must be rejected, and the forgery it would
/// otherwise enable must fail.
///
/// With `pk = O`, `ecMul(pk, c)` is `O` for every `c`, so verification
/// collapses to `response*G == announcement`: the challenge, and with it the
/// message, drops out. An attacker needs no secret — pick any `s`, set
/// `response = s` and `announcement = s*G`, and the pair verifies for EVERY
/// message under that key. That is universal forgery, and it fails OPEN,
/// which is the dangerous direction.
///
/// The forged signature below is constructed exactly that way, so this test
/// fails if the identity guard is ever removed rather than merely asserting
/// that some invalid signature is rejected.
///
/// SCOPE, worth being precise about: this exercises the guard in the RUST
/// verifier (`midnight_compact_runtime::schnorr_verify_jubjub`), because the
/// emitter rewrites the generic `schnorrVerify` call into it — so the
/// circuit's own guard never runs on this path. The circuit guard added to
/// `examples/schnorr_attest_fixture.compact` governs proving and the
/// TypeScript path, and is NOT covered here. That gap is the divergence
/// tracked in MediaNoxLabs/compact#26.
#[test]
fn identity_public_key_is_rejected() {
    let contract = Contract::new(IdentityKeyWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);

    // The forgery: response = s, announcement = s*G, for an arbitrary s.
    let s = EmbeddedFr(embedded::Scalar::from(0xf0e_u64));
    let forged = SchnorrSignature {
        announcement: JubjubPoint::generator() * s,
        response: Fr::from_le_bytes(&s.0.to_bytes()).expect("jubjub scalar fits in Fr"),
    };

    #[allow(clippy::err_expect)]
    let err = contract
        .verify_attestation(ctx, digest(), forged)
        .err()
        .expect("an identity public key must be rejected, not universally forgeable");
    assert!(
        matches!(
            err,
            CompactError::AssertionFailed(ref m) if m == "Schnorr signature verification failed"
        ),
        "expected the Schnorr rejection message, got {err:?}"
    );
}
