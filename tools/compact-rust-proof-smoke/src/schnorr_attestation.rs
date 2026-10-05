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

//! Prove both source Schnorr attestation entry points with recorded ledger-8 VM.

use super::*;
use compact_rust_schnorr_attest_oracle_fixture::ledger_contract as schnorr;
use compact_rust_schnorr_attest_oracle_fixture::types::SchnorrSignature;
use midnight_compact_runtime as runtime;

type Uint248 = runtime::WideUint<
    1329227995784915872903807060280344575_u128,
    340282366920938463463374607431768211455_u128,
>;

#[derive(Clone, Copy)]
struct Witness;
impl schnorr::Witnesses<u64> for Witness {
    fn getSchnorrReduction(
        &self,
        context: WitnessContext<'_, u64, schnorr::LedgerView<'_>>,
        challenge_hash: Field,
    ) -> (u64, (runtime::BoundedUint<127>, Uint248)) {
        let bytes = challenge_hash.as_le_bytes();
        (
            *context.private_state + 1,
            (
                runtime::BoundedUint::<127>::new(bytes[31] as u128).unwrap(),
                Uint248::from_le_bytes(&bytes[..31]).unwrap(),
            ),
        )
    }
    fn localAttestorKey(
        &self,
        context: WitnessContext<'_, u64, schnorr::LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        (
            *context.private_state + 1,
            runtime::ec_mul_generator(Field::from(1_u64)).unwrap(),
        )
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/schnorr-attest-oracle.json"
    ))?;
    let digest = compact_rust_schnorr_attest_oracle_fixture::pure_circuits::attestationDigest(
        runtime::FixedBytes::new([0_u8; 32]),
        runtime::BoundedUint::<{ u64::MAX as u128 }>::new(1)?,
        Field::from(2_u64),
    )?;
    let generator = runtime::ec_mul_generator(Field::from(1_u64))?;
    let response = hex::decode(
        reference["signatureResponseHex"]
            .as_str()
            .ok_or("response")?,
    )?;
    let signature = SchnorrSignature {
        announcement: generator,
        response: Field::from_le_bytes(&response).ok_or("invalid signature response")?,
    };
    for (index, name) in ["verifyAttestation", "acceptAttestation"]
        .iter()
        .enumerate()
    {
        let mut rng = StdRng::seed_from_u64(0x0163_5000 + index as u64);
        let initial = schnorr::initial_state(ConstructorContext::new(7_u64), &Witness)?;
        let deploy = make_deploy(root, name, initial.ledger_state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let context = observed.circuit_context(initial.private_state);
        let native = if index == 0 {
            schnorr::verifyAttestation(context, &Witness, digest.clone(), signature.clone())?
        } else {
            schnorr::acceptAttestation(context, &Witness, digest.clone(), signature.clone())?
        };
        let recorded = if index == 0 {
            schnorr::recorded::verifyAttestation(
                observed.circuit_context(initial.private_state),
                &Witness,
                digest.clone(),
                signature.clone(),
            )?
        } else {
            schnorr::recorded::acceptAttestation(
                observed.circuit_context(initial.private_state),
                &Witness,
                digest.clone(),
                signature.clone(),
            )?
        };
        if native.context.query.state.get_ref() != recorded.execution.context.query.state.get_ref()
            || native.context.query.effects != recorded.execution.context.query.effects
            || native.gas_cost != recorded.execution.gas_cost
            || native.context.private_state != recorded.execution.context.private_state
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
        {
            return Err(format!("{name}: native and recorded differ").into());
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let input = AlignedValue::concat(&[
            AlignedValue::from(digest.clone()),
            AlignedValue::from(signature.clone()),
        ]);
        let manual = check_generated_trace(root, name, recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        let typed = if index == 0 {
            schnorr::Contract::from(Witness)
                .recording()
                .verifyAttestation_call(
                    &observed,
                    initial.private_state,
                    digest.clone(),
                    signature.clone(),
                )?
        } else {
            schnorr::Contract::from(Witness)
                .recording()
                .acceptAttestation_call(
                    &observed,
                    initial.private_state,
                    digest.clone(),
                    signature.clone(),
                )?
        }
        .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{name}: typed and manual calls differ").into());
        }
        check_transaction(root, name, deploy, typed, &mut rng, |applied| {
            if applied.data.get_ref() != &expected_state {
                return Err(format!("{name}: ledger apply state differs").into());
            }
            Ok(())
        })?;
        println!("{name}: pinned proof verified and ledger-8 applied");
    }
    Ok(())
}
