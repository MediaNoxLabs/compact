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

use did_contract::{runtime as r, types as t};
use serde_json::Value;
pub fn string(v: &Value) -> r::OpaqueString {
    r::OpaqueString(v.as_str().unwrap().into())
}
pub fn field_hex(v: &str) -> r::Field {
    r::Field::from_le_bytes(&hex::decode(v).unwrap()).unwrap()
}
pub fn point(secret: &str) -> r::JubjubPoint {
    r::ec_mul_generator(r::Field::from(secret.parse::<u64>().unwrap())).unwrap()
}
pub fn version(v: &str) -> r::BoundedUint<18446744073709551615> {
    r::BoundedUint::new(v.parse().unwrap()).unwrap()
}
pub fn set(v: &Value) -> t::SetMutation {
    match v.as_u64().unwrap() {
        0 => t::SetMutation::Undefined,
        1 => t::SetMutation::Insert,
        2 => t::SetMutation::Remove,
        _ => panic!("invalid test enum"),
    }
}
pub fn map(v: &Value) -> t::MapMutation {
    match v.as_u64().unwrap() {
        0 => t::MapMutation::Undefined,
        1 => t::MapMutation::Insert,
        2 => t::MapMutation::Update,
        _ => panic!("invalid test enum"),
    }
}
pub fn relation(v: &Value) -> t::VerificationMethodRelation {
    match v.as_u64().unwrap() {
        0 => t::VerificationMethodRelation::Undefined,
        1 => t::VerificationMethodRelation::Authentication,
        2 => t::VerificationMethodRelation::AssertionMethod,
        3 => t::VerificationMethodRelation::KeyAgreement,
        4 => t::VerificationMethodRelation::CapabilityInvocation,
        5 => t::VerificationMethodRelation::CapabilityDelegation,
        _ => panic!("invalid test enum"),
    }
}
pub fn method(v: &Value) -> t::VerificationMethod {
    let k = &v["publicKeyJwk"];
    t::VerificationMethod {
        id: string(&v["id"]),
        typ: match v["typ"].as_u64().unwrap() {
            0 => t::VerificationMethodType::Undefined,
            1 => t::VerificationMethodType::JsonWebKey,
            _ => panic!("invalid test enum"),
        },
        publicKeyJwk: t::PublicKeyJwk {
            kty: match k["kty"].as_u64().unwrap() {
                0 => t::KeyType::EC,
                1 => t::KeyType::RSA,
                2 => t::KeyType::oct,
                3 => t::KeyType::OKP,
                _ => panic!("invalid test enum"),
            },
            crv: match k["crv"].as_u64().unwrap() {
                0 => t::CurveType::Ed25519,
                1 => t::CurveType::X25519,
                2 => t::CurveType::Jubjub,
                3 => t::CurveType::P256,
                4 => t::CurveType::Secp256k1,
                5 => t::CurveType::BLS12381G1,
                6 => t::CurveType::BLS12381G2,
                _ => panic!("invalid test enum"),
            },
            x: string(&k["x"]),
            y: string(&k["y"]),
        },
    }
}
pub fn schnorr(v: &Value) -> t::SchnorrJubjubVerificationMethod {
    t::SchnorrJubjubVerificationMethod {
        id: string(&v["id"]),
        publicKey: point(v["key"].as_str().unwrap()),
    }
}
pub fn service(v: &Value) -> t::Service {
    t::Service {
        id: string(&v["id"]),
        typ: string(&v["typ"]),
        serviceEndpoint: string(&v["serviceEndpoint"]),
    }
}
