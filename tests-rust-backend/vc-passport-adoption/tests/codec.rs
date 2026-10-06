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

use compact_rust_vc_passport_adoption_fixture::{runtime as rt, types};
use rt::ledger::CellValue;

fn bytes(seed: u8) -> rt::FixedBytes<32> {
    rt::FixedBytes::new(std::array::from_fn(|index| seed.wrapping_add(index as u8)))
}

fn method(seed: u8) -> types::VerificationMethodRef {
    types::VerificationMethodRef {
        controllerAddress: types::ContractAddress { bytes: bytes(seed) },
        methodId: bytes(seed + 1),
    }
}

fn credential() -> types::Credential {
    types::Credential {
        version: rt::BoundedUint::new(1).unwrap(),
        schema: types::SchemaRef {
            packageId: bytes(1),
            schemaId: bytes(2),
            majorVersion: rt::BoundedUint::new(1).unwrap(),
            minorVersion: rt::BoundedUint::new(3).unwrap(),
        },
        issuerVerificationMethodRef: method(3),
        holderBinding: types::ExplicitHolderBinding {
            holderVerificationMethodRef: method(5),
        },
        issuedAt: rt::BoundedUint::new((1_u128 << 40) + 9).unwrap(),
        hasExpiration: true,
        expiresAt: rt::BoundedUint::new((1_u128 << 40) + 365).unwrap(),
        claimCommitments: types::DigitalPassportClaimCommitments {
            firstNameCommitment: bytes(11),
            lastNameCommitment: bytes(12),
            dateOfBirthCommitment: bytes(13),
            documentNumberCommitment: bytes(14),
            issuingStateCommitment: bytes(15),
        },
        claimRoot: bytes(16),
        ..Default::default()
    }
}

fn proof() -> types::Proof {
    types::Proof {
        signerVerificationMethodRef: method(3),
        createdAt: rt::BoundedUint::new((1_u128 << 40) + 10).unwrap(),
        challengeHash: bytes(21),
        publicKey: rt::ec_mul_generator(rt::Field::from(3_u64)).unwrap(),
        signature: types::Signature {
            r: rt::ec_mul_generator(rt::Field::from(7_u64)).unwrap(),
            s: rt::Field::from(13_u64),
        },
    }
}

fn framed_value(value: &rt::fab::Value) -> Vec<u8> {
    let mut framed = b"MCV1".to_vec();
    framed.extend(u32::try_from(value.0.len()).unwrap().to_be_bytes());
    for atom in &value.0 {
        framed.extend(u32::try_from(atom.0.len()).unwrap().to_be_bytes());
        framed.extend(&atom.0);
    }
    framed
}

fn assert_codec<T>(label: &str, value: T)
where
    T: Clone + PartialEq + std::fmt::Debug + Into<rt::fab::AlignedValue> + CellValue,
{
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-codec-capture.json")).unwrap();
    let upstream: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-codec-capture.json")).unwrap();
    assert_eq!(capture["vectors"], upstream["vectors"]);
    let expected = &capture["vectors"][label];
    let aligned: rt::fab::AlignedValue = value.clone().into();
    let chunks = aligned
        .value
        .0
        .iter()
        .map(|atom| hex::encode(&atom.0))
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::json!(chunks),
        expected["chunks"],
        "{label} chunks"
    );
    assert_eq!(
        serde_json::to_value(&aligned.alignment).unwrap(),
        expected["alignment"],
        "{label} alignment"
    );
    assert_eq!(
        hex::encode(framed_value(&aligned.value)),
        expected["framedHex"],
        "{label} MCV1 frame"
    );
    assert_eq!(T::decode_cell_value(&aligned.value).unwrap(), value);
}

#[test]
fn generated_fab_codecs_match_pinned_passport_descriptors_and_transport_frame() {
    assert_codec("credential", credential());
    assert_codec("proof", proof());
    let mut minimal = credential();
    minimal.issuedAt = rt::BoundedUint::new(0).unwrap();
    minimal.hasExpiration = false;
    minimal.expiresAt = rt::BoundedUint::new(0).unwrap();
    minimal.claimCommitments.documentNumberCommitment = rt::FixedBytes::new([0; 32]);
    minimal.claimRoot = rt::FixedBytes::new([0; 32]);
    assert_codec("credentialMinimal", minimal);
    let mut boundary = proof();
    boundary.createdAt = rt::BoundedUint::new(0).unwrap();
    boundary.signature.r = rt::ec_mul_generator(rt::Field::from(1_u64)).unwrap();
    boundary.signature.s = rt::Field::from(0_u64);
    assert_codec("proofBoundary", boundary);
}
