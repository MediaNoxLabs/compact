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

//! Calls all original pure exports directly, including values rejected only by
//! stateful guards. These sampled hashes do not prove collision resistance.
use compact_rust_did_adoption_fixture::{pure_circuits as p, runtime as r, types as t};
use serde_json::{Value, json};
#[path = "../support/codec.rs"]
mod codec;
use codec::*;
fn invoke(row: &Value) -> r::FixedVector<r::Field, 4> {
    let a = &row["args"];
    let address = t::ContractAddress {
        bytes: r::FixedBytes::new([row["addressByte"].as_u64().unwrap() as u8; 32]),
    };
    let version = version(row["version"].as_str().unwrap());
    match row["name"].as_str().unwrap() {
        "controllerAuthorizationDigest" => p::controllerAuthorizationDigest(
            address,
            version,
            r::Field::from(a["operationHash"].as_str().unwrap().parse::<u64>().unwrap()),
            r::Field::from(a["argsHash"].as_str().unwrap().parse::<u64>().unwrap()),
        ),
        "rotateControllerKeyAuthorizationDigest" => p::rotateControllerKeyAuthorizationDigest(
            address,
            version,
            point(a["key"].as_str().unwrap()),
        ),
        "recoverControllerKeyAuthorizationDigest" => p::recoverControllerKeyAuthorizationDigest(
            address,
            version,
            point(a["key"].as_str().unwrap()),
        ),
        "setAlsoKnownAsAuthorizationDigest" => p::setAlsoKnownAsAuthorizationDigest(
            address,
            version,
            string(&a["value"]),
            set(&a["mutation"]),
        ),
        "setVerificationMethodAuthorizationDigest" => p::setVerificationMethodAuthorizationDigest(
            address,
            version,
            method(&a["method"]),
            map(&a["mutation"]),
        ),
        "removeVerificationMethodAuthorizationDigest" => {
            p::removeVerificationMethodAuthorizationDigest(address, version, string(&a["id"]))
        }
        "setSchnorrJubjubVerificationMethodAuthorizationDigest" => {
            p::setSchnorrJubjubVerificationMethodAuthorizationDigest(
                address,
                version,
                schnorr(&a["method"]),
                map(&a["mutation"]),
            )
        }
        "removeSchnorrJubjubVerificationMethodAuthorizationDigest" => {
            p::removeSchnorrJubjubVerificationMethodAuthorizationDigest(
                address,
                version,
                string(&a["id"]),
            )
        }
        "setVerificationMethodRelationAuthorizationDigest" => {
            p::setVerificationMethodRelationAuthorizationDigest(
                address,
                version,
                relation(&a["relation"]),
                string(&a["id"]),
                set(&a["mutation"]),
            )
        }
        "setServiceAuthorizationDigest" => p::setServiceAuthorizationDigest(
            address,
            version,
            service(&a["service"]),
            map(&a["mutation"]),
        ),
        "removeServiceAuthorizationDigest" => {
            p::removeServiceAuthorizationDigest(address, version, string(&a["id"]))
        }
        "deactivateAuthorizationDigest" => p::deactivateAuthorizationDigest(address, version),
        other => panic!("unreviewed export {other}"),
    }
    .unwrap()
}
fn cases() -> Vec<Value> {
    let capture: Value = serde_json::from_str(include_str!("../oracle/pure.json")).unwrap();
    capture["cases"].as_array().unwrap().clone()
}
fn check(name: &str, expected_count: usize) {
    let rows: Vec<_> = cases().into_iter().filter(|r| r["name"] == name).collect();
    assert_eq!(rows.len(), expected_count);
    let base = invoke(&rows[0]);
    for row in rows {
        let value = invoke(&row);
        let captured: Vec<_> = row["resultHex"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| field_hex(v.as_str().unwrap()))
            .collect();
        assert_eq!(
            value.0.as_slice(),
            captured.as_slice(),
            "{name}/{}",
            row["id"]
        );
        assert_eq!(
            json!(
                value
                    .0
                    .as_slice()
                    .iter()
                    .map(|f| hex::encode(f.as_le_bytes()))
                    .collect::<Vec<_>>()
            ),
            row["resultHex"]
        );
        if row["id"] != "base" {
            assert_ne!(
                value, base,
                "sampled argument sensitivity: {name}/{}",
                row["id"]
            );
        }
    }
}
macro_rules! api {
    ($test:ident,$name:literal,$n:literal) => {
        #[test]
        fn $test() {
            check($name, $n);
        }
    };
}
api!(controller_domain, "controllerAuthorizationDigest", 6);
api!(
    rotate_controller,
    "rotateControllerKeyAuthorizationDigest",
    5
);
api!(
    recover_controller,
    "recoverControllerKeyAuthorizationDigest",
    5
);
api!(set_alias, "setAlsoKnownAsAuthorizationDigest", 8);
api!(set_method, "setVerificationMethodAuthorizationDigest", 22);
api!(
    remove_method,
    "removeVerificationMethodAuthorizationDigest",
    6
);
api!(
    set_schnorr_method,
    "setSchnorrJubjubVerificationMethodAuthorizationDigest",
    9
);
api!(
    remove_schnorr_method,
    "removeSchnorrJubjubVerificationMethodAuthorizationDigest",
    6
);
api!(
    set_relation,
    "setVerificationMethodRelationAuthorizationDigest",
    13
);
api!(set_service, "setServiceAuthorizationDigest", 12);
api!(remove_service, "removeServiceAuthorizationDigest", 6);
api!(deactivate, "deactivateAuthorizationDigest", 4);
#[test]
fn distinct_operation_domains_for_reviewed_base_calls() {
    let mut operation_fields = std::collections::BTreeSet::new();
    for row in cases()
        .into_iter()
        .filter(|r| r["id"] == "base" && r["name"] != "controllerAuthorizationDigest")
    {
        let digest = invoke(&row);
        assert!(
            operation_fields.insert(hex::encode(digest.0.as_slice()[2].as_le_bytes())),
            "sampled operation domains must differ"
        );
    }
    assert_eq!(operation_fields.len(), 11);
}
