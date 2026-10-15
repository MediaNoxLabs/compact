// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use compact_rust_schnorr_attest_oracle_fixture::{ledger_contract as c, ledger_slots as slots};
use midnight_compact_runtime as r;
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::transaction::{ObservedContractState, VerifierKey};
mod manual;
#[path = "../../../support/witness_only_recording.rs"]
#[allow(dead_code)]
mod observation;
mod provider;
mod scenarios;
use provider::{Behavior, Event, Provider};

#[derive(Clone, Copy)]
enum Root {
    Verify,
    Accept,
}
impl Root {
    fn accepts(self) -> bool {
        matches!(self, Self::Accept)
    }
    fn private(self) -> u64 {
        if self.accepts() { 9 } else { 8 }
    }
    fn trace(self) -> &'static str {
        if self.accepts() {
            "acceptTrace"
        } else {
            "verifyTrace"
        }
    }
    fn transcript(self) -> &'static str {
        if self.accepts() {
            "acceptTranscript"
        } else {
            "verifyTranscript"
        }
    }
    fn native(
        self,
        state: &ObservedContractState,
        provider: &Provider,
        digest: r::FixedVector<Field, 4>,
        signature: SchnorrSignature,
    ) -> Result<r::context::CircuitResult<u64, ()>, r::CompactError> {
        let context = state.circuit_context(self.private());
        if self.accepts() {
            c::acceptAttestation(context, provider, digest, signature)
        } else {
            c::verifyAttestation(context, provider, digest, signature)
        }
    }
    fn observed<'a>(
        self,
        state: &'a ObservedContractState,
        provider: &Provider,
        digest: r::FixedVector<Field, 4>,
        signature: SchnorrSignature,
    ) -> Result<r::transaction::RecordedCall<'a, u64, ()>, r::CompactError> {
        let contract = c::Contract::from(provider.clone());
        if self.accepts() {
            contract
                .recording()
                .acceptAttestation_call(state, self.private(), digest, signature)
        } else {
            contract
                .recording()
                .verifyAttestation_call(state, self.private(), digest, signature)
        }
    }
    fn recorded(
        self,
        state: &ObservedContractState,
        provider: &Provider,
        digest: r::FixedVector<Field, 4>,
        signature: SchnorrSignature,
    ) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
        let context = state.circuit_context(self.private());
        if self.accepts() {
            c::recorded::acceptAttestation(context, provider, digest, signature)
        } else {
            c::recorded::verifyAttestation(context, provider, digest, signature)
        }
    }
}
fn inputs() -> (
    serde_json::Value,
    r::FixedVector<Field, 4>,
    SchnorrSignature,
) {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../runtime-rs/tests/fixtures/schnorr-attest-oracle.json"
    ))
    .unwrap();
    let digest = attestationDigest(
        FixedBytes::new([0; 32]),
        BoundedUint::new(1).unwrap(),
        Field::from(2_u64),
    )
    .unwrap();
    let signature = SchnorrSignature {
        announcement: r::ec_mul_generator(Field::from(1_u64)).unwrap(),
        response: Field::from_le_bytes(
            &hex::decode(oracle["signatureResponseHex"].as_str().unwrap()).unwrap(),
        )
        .unwrap(),
    };
    (oracle, digest, signature)
}
fn setup(closed: bool, identity_key: bool) -> (ObservedContractState, VerifierKey) {
    let initial = initial_state(ConstructorContext::new(7_u64), &OracleWitness).unwrap();
    let mut context = initial.into_circuit_context(Default::default());
    if closed {
        context = slots::open.write(context, false).unwrap().context;
    }
    if identity_key {
        context = slots::attestorKey
            .write(context, JubjubPoint::default())
            .unwrap()
            .context;
    }
    observation::observation(
        context.query.state.get_ref().clone(),
        &["verifyAttestation", "acceptAttestation"],
    )
}
fn assertion(condition: bool, message: &str) -> Result<(), r::CompactError> {
    if condition {
        Ok(())
    } else {
        Err(r::CompactError::AssertionFailed(message.into()))
    }
}
