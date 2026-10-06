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
    rt::FixedBytes::new(std::array::from_fn(|i| seed.wrapping_add(i as u8)))
}
fn b64(seed: u8) -> rt::FixedBytes<64> {
    rt::FixedBytes::new(std::array::from_fn(|i| seed.wrapping_add(i as u8)))
}
fn pad(s: &str) -> rt::FixedBytes<32> {
    let mut a = [0; 32];
    a[..s.len()].copy_from_slice(s.as_bytes());
    rt::FixedBytes::new(a)
}
fn hex(a: rt::FixedBytes<32>) -> String {
    a.into_array()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn field_hex(value: rt::Field) -> String {
    let mut result = hex::encode(
        value
            .as_le_bytes()
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>(),
    );
    while result.starts_with('0') && result.len() > 1 {
        result.remove(0);
    }
    result
}
fn schema() -> types::SchemaRef {
    types::SchemaRef {
        packageId: pad("midnight:vc:digital-passport"),
        schemaId: pad("digital-passport:v1"),
        majorVersion: rt::BoundedUint::<65535>::new(1).unwrap(),
        minorVersion: rt::BoundedUint::<65535>::new(0).unwrap(),
    }
}
fn method() -> types::VerificationMethodRef {
    types::VerificationMethodRef {
        controllerAddress: types::ContractAddress { bytes: b32(1) },
        methodId: b32(2),
    }
}
fn commitments() -> types::DigitalPassportClaimCommitments {
    types::DigitalPassportClaimCommitments {
        firstNameCommitment: b32(10),
        lastNameCommitment: b32(11),
        dateOfBirthCommitment: b32(12),
        documentNumberCommitment: b32(13),
        issuingStateCommitment: b32(14),
    }
}
fn credential() -> types::Credential {
    let com = commitments();
    types::Credential {
        version: rt::BoundedUint::<65535>::new(1).unwrap(),
        schema: schema(),
        issuerVerificationMethodRef: method(),
        holderBinding: types::ExplicitHolderBinding {
            holderVerificationMethodRef: method(),
        },
        issuedAt: rt::BoundedUint::<18446744073709551615>::new(100).unwrap(),
        claimCommitments: com.clone(),
        claimRoot: pure::digitalPassportClaimRoot(com).unwrap(),
        ..Default::default()
    }
}
fn presentation() -> types::Presentation {
    types::Presentation {
        version: rt::BoundedUint::<65535>::new(1).unwrap(),
        schema: schema(),
        credentialClaimRoot: pure::digitalPassportClaimRoot(commitments()).unwrap(),
        issuerVerificationMethodRef: method(),
        holderBinding: types::ExplicitHolderBinding {
            holderVerificationMethodRef: method(),
        },
        disclosed: types::DigitalPassportDisclosures {
            firstNameValuePadded: b64(20),
            firstNameOpening: b32(21),
            lastNameValuePadded: b64(22),
            lastNameOpening: b32(23),
            proveAgeOverThreshold: true,
            ageThresholdYears: rt::BoundedUint::<255>::new(18).unwrap(),
            documentNumberValue: b32(24),
            documentNumberOpening: b32(25),
            issuingStateValue: b32(26),
            issuingStateOpening: b32(27),
            ..Default::default()
        },
    }
}
#[test]
fn body_roots() {
    let c = credential();
    assert_eq!(
        hex(pure::digitalPassportCredentialBodyRoot(c.clone()).unwrap()),
        "45b85bc2af786f741cd94cbef575ddb3b1e90902ad9393a335b1c148fbeb8ecf"
    );
    let mut c2 = c;
    c2.issuedAt = rt::BoundedUint::<18446744073709551615>::new(101).unwrap();
    assert_eq!(
        hex(pure::digitalPassportCredentialBodyRoot(c2).unwrap()),
        "d5fc1fbc9157667052501c926561f9b082ddc7c3b0b61534274aaa8c539cd398"
    );
    let p = presentation();
    assert_eq!(
        hex(pure::digitalPassportPresentationBodyRoot(p.clone()).unwrap()),
        "9b9f66f1b19bf6eb27061a48ff2b493ed1d5f9d98c144dfefbc6f67a71fedc9b"
    );
    let mut p2 = p;
    p2.disclosed.revealFirstName = true;
    assert_eq!(
        hex(pure::digitalPassportPresentationBodyRoot(p2).unwrap()),
        "b75983a5c45c93f361517f721008df1680161962987a30a2749c8a99656dd607"
    );
}
#[test]
fn proof_roots() {
    let proof = types::Proof {
        signerVerificationMethodRef: method(),
        createdAt: rt::BoundedUint::<18446744073709551615>::new(100).unwrap(),
        challengeHash: b32(55),
        ..Default::default()
    };
    let root = b32(33);
    assert_eq!(
        hex(pure::issuanceProofPayloadRoot(root, proof.clone()).unwrap()),
        "e9ce6b985e5b953ee7ec9031b19748ecd37a0788820109ff0221fea5a75247d8"
    );
    assert_eq!(
        hex(pure::presentationProofPayloadRoot(root, proof.clone()).unwrap()),
        "c6432862cd909966d0db171cf5b7c47812dc5f88ca3a3fc1d1966011cb626aee"
    );
    assert_eq!(
        field_hex(pure::issuanceProofChallenge(root, proof.clone()).unwrap()),
        "3ea120ef3df3e03347bb5a447e67a86e50f95cd92f15eaac18a34c5600ccea"
    );
    assert_eq!(
        field_hex(pure::presentationProofChallenge(root, proof.clone()).unwrap()),
        "b90eb14a50852bc6147817fcc906ccf696a06329eccedc3884eb4af8414cd9"
    );
    assert_eq!(
        field_hex(pure::signerAuthorizationProofChallenge(root, proof.clone()).unwrap()),
        "f21bc778bc5055278641bbc120ff377eb08f475c7aea8cada6a46e690330aa"
    );
    assert_eq!(
        field_hex(pure::verifierRequestProofChallenge(root, proof).unwrap()),
        "3f6f3b50dde185742d8065ea30301e1df3e14465525915741fc1a7c6bd5922"
    );
}
