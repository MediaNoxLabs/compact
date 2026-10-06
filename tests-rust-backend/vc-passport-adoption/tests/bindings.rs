// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_vc_passport_adoption_fixture::{pure_circuits as pure, runtime as rt, types};

fn b32(seed: u8) -> rt::FixedBytes<32> {
    rt::FixedBytes::new(std::array::from_fn(|index| seed.wrapping_add(index as u8)))
}
fn padded(text: &str) -> rt::FixedBytes<32> {
    let mut bytes = [0; 32];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    rt::FixedBytes::new(bytes)
}
fn method() -> types::VerificationMethodRef {
    types::VerificationMethodRef {
        controllerAddress: types::ContractAddress { bytes: b32(1) },
        methodId: b32(2),
    }
}
fn binding() -> types::ExplicitHolderBinding {
    types::ExplicitHolderBinding {
        holderVerificationMethodRef: method(),
    }
}
fn proof() -> types::Proof {
    types::Proof {
        signerVerificationMethodRef: method(),
        createdAt: rt::BoundedUint::<18446744073709551615>::new(100).unwrap(),
        challengeHash: b32(55),
        ..Default::default()
    }
}
fn registry() -> types::StatusRegistryRef {
    types::StatusRegistryRef {
        registryId: b32(20),
        authorityVerificationMethodRef: method(),
    }
}
fn status() -> types::RegistryBoundStatusBinding {
    types::RegistryBoundStatusBinding {
        registryRef: registry(),
        statusHandleCommitment: b32(21),
    }
}
fn schema() -> types::SchemaRef {
    types::SchemaRef {
        packageId: padded("midnight:vc:digital-passport"),
        schemaId: padded("digital-passport:v1"),
        majorVersion: rt::BoundedUint::<65535>::new(1).unwrap(),
        minorVersion: rt::BoundedUint::<65535>::new(0).unwrap(),
    }
}
fn rows() -> Vec<serde_json::Value> {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-bindings-capture.json")).unwrap();
    capture["rows"].as_array().unwrap().clone()
}
fn assert_unit(name: &str, actual: Result<(), rt::CompactError>, rows: &[serde_json::Value]) {
    let expected = rows.iter().find(|row| row["name"] == name).unwrap();
    if expected["outcome"] == "ok" {
        assert_eq!(expected["value"], serde_json::json!([]), "{name}");
        assert!(actual.is_ok(), "{name}: {actual:?}");
    } else {
        assert_eq!(
            actual.unwrap_err().to_string(),
            expected["message"].as_str().unwrap(),
            "{name}"
        );
    }
}
fn assert_root(
    name: &str,
    actual: Result<rt::FixedBytes<32>, rt::CompactError>,
    rows: &[serde_json::Value],
) {
    let expected = rows.iter().find(|row| row["name"] == name).unwrap();
    if expected["outcome"] == "ok" {
        assert_eq!(
            hex::encode(actual.unwrap().into_array()),
            expected["value"].as_str().unwrap(),
            "{name}"
        );
    } else {
        assert_eq!(
            actual.unwrap_err().to_string(),
            expected["message"].as_str().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn holder_and_status_bindings_match_both_typescript_oracles() {
    let rows = rows();
    assert_eq!(rows.len(), 23);
    let mut empty_method = method();
    empty_method.methodId = rt::FixedBytes::new([0; 32]);
    let mut other_controller = method();
    other_controller.controllerAddress.bytes = b32(3);
    let mut other_method = method();
    other_method.methodId = b32(4);
    let mut other_proof_controller = proof();
    other_proof_controller.signerVerificationMethodRef = other_controller.clone();
    let mut other_proof_method = proof();
    other_proof_method.signerVerificationMethodRef = other_method.clone();
    let empty = rt::FixedBytes::new([0; 32]);

    assert_unit(
        "holder_valid",
        pure::assertValidExplicitHolderBinding(binding()),
        &rows,
    );
    assert_unit(
        "holder_missing_method",
        pure::assertValidExplicitHolderBinding(types::ExplicitHolderBinding {
            holderVerificationMethodRef: empty_method.clone(),
        }),
        &rows,
    );
    assert_unit(
        "holder_match",
        pure::assertMatchingExplicitHolderBindings(binding(), binding()),
        &rows,
    );
    assert_unit(
        "holder_other_controller",
        pure::assertMatchingExplicitHolderBindings(
            binding(),
            types::ExplicitHolderBinding {
                holderVerificationMethodRef: other_controller,
            },
        ),
        &rows,
    );
    assert_unit(
        "holder_other_method",
        pure::assertMatchingExplicitHolderBindings(
            binding(),
            types::ExplicitHolderBinding {
                holderVerificationMethodRef: other_method,
            },
        ),
        &rows,
    );
    assert_unit(
        "holder_proof_match",
        pure::assertProofMatchesExplicitHolderBinding(binding(), proof()),
        &rows,
    );
    assert_unit(
        "holder_proof_other_controller",
        pure::assertProofMatchesExplicitHolderBinding(binding(), other_proof_controller),
        &rows,
    );
    assert_unit(
        "holder_proof_other_method",
        pure::assertProofMatchesExplicitHolderBinding(binding(), other_proof_method),
        &rows,
    );

    assert_unit(
        "registry_valid",
        pure::assertValidStatusRegistryRef(registry()),
        &rows,
    );
    assert_unit(
        "registry_missing_id",
        pure::assertValidStatusRegistryRef(types::StatusRegistryRef {
            registryId: empty,
            ..registry()
        }),
        &rows,
    );
    assert_unit(
        "registry_missing_authority",
        pure::assertValidStatusRegistryRef(types::StatusRegistryRef {
            authorityVerificationMethodRef: empty_method,
            ..registry()
        }),
        &rows,
    );
    assert_unit(
        "status_valid",
        pure::assertValidRegistryBoundStatusBinding(status()),
        &rows,
    );
    assert_unit(
        "status_missing_handle",
        pure::assertValidRegistryBoundStatusBinding(types::RegistryBoundStatusBinding {
            statusHandleCommitment: empty,
            ..status()
        }),
        &rows,
    );
    assert_unit(
        "status_missing_registry",
        pure::assertValidRegistryBoundStatusBinding(types::RegistryBoundStatusBinding {
            registryRef: types::StatusRegistryRef {
                registryId: empty,
                ..registry()
            },
            ..status()
        }),
        &rows,
    );
    assert_root(
        "status_root_valid",
        pure::registryBoundStatusBindingRoot(status()),
        &rows,
    );
    assert_root(
        "status_root_invalid",
        pure::registryBoundStatusBindingRoot(types::RegistryBoundStatusBinding {
            statusHandleCommitment: empty,
            ..status()
        }),
        &rows,
    );
    assert_root(
        "issuer_scope_valid",
        pure::issuerScopeCommitment(schema()),
        &rows,
    );
    assert_root(
        "issuer_scope_invalid",
        pure::issuerScopeCommitment(types::SchemaRef {
            schemaId: empty,
            ..schema()
        }),
        &rows,
    );
}

#[test]
fn identity_proof_is_rejected_by_each_context() {
    let rows = rows();
    let proof = proof();
    let root = b32(33);
    assert!(proof.publicKey == rt::JubjubPoint::default());
    let bad_signature = pure::verifySignature(
        proof.publicKey,
        proof.signature.clone(),
        rt::Field::from(5_u64),
    );
    let expected = rows
        .iter()
        .find(|row| row["name"] == "verify_identity_key")
        .unwrap();
    assert_eq!(
        bad_signature.unwrap_err().to_string(),
        expected["message"].as_str().unwrap()
    );
    assert_unit(
        "issuance_proof_identity_key",
        pure::assertValidIssuanceContextProof(root, proof.clone()),
        &rows,
    );
    assert_unit(
        "presentation_proof_identity_key",
        pure::assertValidPresentationContextProof(root, proof.clone()),
        &rows,
    );
    assert_unit(
        "signer_proof_identity_key",
        pure::assertValidSignerAuthorizationContextProof(root, proof.clone()),
        &rows,
    );
    assert_unit(
        "verifier_proof_identity_key",
        pure::assertValidVerifierRequestContextProof(root, proof),
        &rows,
    );
}
