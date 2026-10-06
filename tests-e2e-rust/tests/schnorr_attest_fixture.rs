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

/// Uses the public key from midnight-did's fixed TypeScript parity vector.
struct OfficialTsWitnesses;

impl Witnesses<()> for OfficialTsWitnesses {
    fn get_schnorr_reduction<'a>(
        &self,
        _ctx: &WitnessContext<Ledger<'a>, ()>,
        _challenge_hash: Fr,
    ) -> ((), (u8, u128)) {
        panic!("dead get_schnorr_reduction witness was invoked")
    }

    fn local_attestor_key<'a>(&self, _ctx: &WitnessContext<Ledger<'a>, ()>) -> ((), JubjubPoint) {
        ((), official_ts_public_key())
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

/// The legacy, incompatible challenge reduction: full `Fr` modulo the
/// Jubjub scalar order. Responses still use this conversion because they are
/// scalar values carried in Compact's wider `Field` type.
fn fr_to_embedded_mod_r(fr: Fr) -> EmbeddedFr {
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

/// The challenge both the official TypeScript implementation, Compact
/// circuit, and native verifier compute: the low 248 bits of the hash.
fn challenge(ann: JubjubPoint, pk: JubjubPoint, msg: &[Fr]) -> EmbeddedFr {
    let mut bytes = challenge_hash(ann, pk, msg).as_le_bytes();
    bytes.resize(32, 0);
    bytes[31] = 0;
    EmbeddedFr::from_le_bytes(&bytes).expect("2^248-truncated challenge fits in Jubjub Fr")
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

fn fr_from_be_hex(value: &str) -> Fr {
    let mut bytes = hex::decode(value).expect("valid fixed hex");
    bytes.reverse();
    Fr::from_le_bytes(&bytes).expect("fixed value is a canonical outer Fr")
}

fn embedded_from_be_hex(value: &str) -> EmbeddedFr {
    let mut bytes = hex::decode(value).expect("valid fixed hex");
    bytes.reverse();
    EmbeddedFr::from_le_bytes(&bytes).expect("fixed value is a canonical Jubjub scalar")
}

fn embedded_from_fr(value: Fr) -> EmbeddedFr {
    fr_to_embedded_mod_r(value)
}

fn embedded_to_be_hex(value: EmbeddedFr) -> String {
    let mut bytes = value.as_le_bytes();
    bytes.resize(32, 0);
    bytes.reverse();
    hex::encode(bytes)
}

fn point_from_be_hex(x: &str, y: &str) -> JubjubPoint {
    JubjubPoint::new(fr_from_be_hex(x), fr_from_be_hex(y)).expect("fixed point is on Jubjub")
}

fn official_ts_public_key() -> JubjubPoint {
    point_from_be_hex(
        "10cc9670cf170b19094f29fc3035cce2aeb054b31fa82c580aed0cc13d211cf4",
        "1f4c181670dbd0619140fce7977354f46d6ca2176c30cad5759a44432899addd",
    )
}

fn official_ts_digest() -> [Fr; 4] {
    [
        Fr::from(0x2bdb_0067_176f_d1bfu64),
        Fr::from(0xb017_2636_b6c9_1955u64),
        Fr::from(0xe28e_ed13_04bc_16d9u64),
        Fr::from(0xcbb1_5010_30aa_4576u64),
    ]
}

fn official_ts_signature() -> SchnorrSignature {
    SchnorrSignature {
        announcement: point_from_be_hex(
            "02b4bfc039ddca33a2bc807a2df358682a81a6dd0db45eaf9567f00d00021146",
            "0abff840b93c8fbc864111ba6009a31d227a9e04d44adcc6a44c0b103bb459da",
        ),
        response: fr_from_be_hex(
            "0603b2f0bc6eb850600cc297da66b157c88c53a731cfda0887153d531eabcd9c",
        ),
    }
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
/// The fixed key, nonce, and message also pin a transient hash with
/// significant bytes above bit 248. This is the executable regression for the
/// dead `Uint<248>` witness surface: the native verifier must retain the low
/// 248 bits exactly, without routing them through generated `u128` code.
#[test]
fn valid_wide_challenge_signature_is_accepted_and_counted() {
    let contract = contract();
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, init.current_private_state);
    let initial_query_context = ctx.current_query_context.clone();

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
    assert_ne!(
        challenge_bytes[31], 0,
        "fixture hash must exercise bits discarded by mod 2^248"
    );

    let after = contract
        .verify_attestation(ctx, msg, signature)
        .expect("a valid low-248-bit-challenge signature must verify");

    // The generated local verifier shares the exported wrapper's proof data,
    // just as the TypeScript emitter passes one `partialProofData` object
    // through `_verifyAttestation_0` -> `_schnorrVerifyDigest_0`. Only the
    // exported/root metadata survives, and the dead reduction witness adds no
    // private output.
    let root = after
        .context
        .call_proof_data_trace
        .single_contract_call()
        .expect("nested verifier must fold into one exported root");
    assert_eq!(root.circuit_id, "verify_attestation");
    assert_eq!(root.contract_address, &initial_query_context.address);
    assert!(root.private_transcript_outputs.is_empty());
    assert_eq!(
        root.input,
        &aligned_value_from_parts(&[proof_aligned_array(&msg), proof_aligned_value(&signature),])
    );
    assert_eq!(root.output, &aligned_value_from_parts(&[]));
    assert_eq!(
        root.initial_query_context.address,
        initial_query_context.address
    );
    assert_eq!(
        root.initial_query_context.state,
        initial_query_context.state
    );
    assert_eq!(
        root.final_query_context.address,
        after.context.current_query_context.address
    );
    assert_eq!(
        root.final_query_context.state,
        after.context.current_query_context.state
    );
    assert_eq!(root.public_transcript.len(), 6);
    assert!(matches!(
        root.public_transcript,
        [
            Op::Dup { .. },
            Op::Idx { .. },
            Op::Popeq { .. },
            Op::Dup { .. },
            Op::Idx { .. },
            Op::Popeq { .. },
        ]
    ));

    // The mutating sibling runs the same rewritten call and then commits
    // a ledger write, so the routing has to leave the context usable.
    let after = contract
        .accept_attestation(after.context, msg, sign(&msg))
        .expect("a valid signature must be accepted");
    let view = ledger(&after.context.current_query_context.state);
    assert_eq!(view.accepted_count().expect("accepted_count"), 1u64);
    assert_eq!(after.context.call_proof_data_trace.len(), 2);
    assert!(matches!(
        after.context.call_proof_data_trace.single_contract_call(),
        Err(CompactError::ProofData(_))
    ));
}

/// Official TypeScript golden vector from
/// `@midnight-ntwrk/midnight-did-jubjub-schnorr`: seed `01..20`, payload
/// `midnight-identity jubjub-schnorr golden vector`. The digest and 96-byte
/// signature are fixed reference outputs, not generated by this test.
#[test]
fn official_ts_mod_2_248_signature_is_accepted_and_legacy_mod_r_is_rejected() {
    let contract: Contract<(), OfficialTsWitnesses> = Contract::new(OfficialTsWitnesses);
    let init = contract.initial_state(ctor_ctx()).expect("initial_state");
    let state = init.current_contract_state;
    let msg = official_ts_digest();
    let signature = official_ts_signature();
    let public_key = official_ts_public_key();

    let full_challenge = challenge_hash(signature.announcement, public_key, &msg);
    let full_bytes = full_challenge.as_le_bytes();
    assert_ne!(
        full_bytes[31], 0,
        "golden challenge must exercise bits above bit 248"
    );
    let ts_challenge = challenge(signature.announcement, public_key, &msg);
    assert_eq!(
        embedded_to_be_hex(ts_challenge),
        "00ea4164e7c7915d9443d109b1b4ada096b5328543b42d97c1ce2994871d2dc8"
    );

    contract
        .verify_attestation(CircuitContext::new(state.clone(), ()), msg, signature)
        .expect("official TypeScript signature must verify in generated Rust");

    let legacy_challenge = fr_to_embedded_mod_r(full_challenge);
    assert_ne!(legacy_challenge, ts_challenge);
    let secret =
        embedded_from_be_hex("003cf13236b83e4b35ea71e39406dfdd00e489a68dae970a1e77b592a40ffa35");
    let ts_response = embedded_from_fr(signature.response);
    let legacy_response = ts_response + (legacy_challenge - ts_challenge) * secret;
    let legacy_signature = SchnorrSignature {
        announcement: signature.announcement,
        response: Fr::from_le_bytes(&legacy_response.0.to_bytes())
            .expect("Jubjub response fits in outer Fr"),
    };
    assert_eq!(
        JubjubPoint::generator() * legacy_response,
        legacy_signature.announcement + public_key * legacy_challenge,
        "negative control must be valid under the legacy mod-r challenge"
    );

    #[allow(clippy::err_expect)]
    let err = contract
        .verify_attestation(CircuitContext::new(state, ()), msg, legacy_signature)
        .err()
        .expect("legacy mod-r response must be rejected by mod-2^248 verification");
    assert!(matches!(err, CompactError::AssertionFailed(_)));
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
