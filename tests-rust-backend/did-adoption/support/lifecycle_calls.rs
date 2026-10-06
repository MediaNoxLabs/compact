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

use super::{codec::*, lifecycle_witness::Witness};
use compact_rust_did_adoption_fixture::{ledger_contract as c, runtime as r, types as t};
use serde_json::Value;
// Explicit typed generated calls: this table is test dispatch, not an evaluator.
pub fn invoke(
    context: r::context::CircuitContext<u64>,
    w: &Witness,
    row: &Value,
) -> Result<r::context::CircuitResult<u64, ()>, r::CompactError> {
    let a = &row["args"];
    let sig = t::SchnorrSignature {
        announcement: point("2"),
        response: field_hex(row["responseHex"].as_str().unwrap()),
    };
    let version = version(row["version"].as_str().unwrap());
    match row["name"].as_str().unwrap() {
        "rotateControllerKey" => {
            c::rotateControllerKey(context, w, point(a["key"].as_str().unwrap()), sig, version)
        }
        "recoverControllerKey" => {
            c::recoverControllerKey(context, w, point(a["key"].as_str().unwrap()), sig, version)
        }
        "setAlsoKnownAs" => c::setAlsoKnownAs(
            context,
            w,
            string(&a["value"]),
            set(&a["mutation"]),
            sig,
            version,
        ),
        "setVerificationMethod" => c::setVerificationMethod(
            context,
            w,
            method(&a["method"]),
            map(&a["mutation"]),
            sig,
            version,
        ),
        "removeVerificationMethod" => {
            c::removeVerificationMethod(context, w, string(&a["id"]), sig, version)
        }
        "setSchnorrJubjubVerificationMethod" => c::setSchnorrJubjubVerificationMethod(
            context,
            w,
            schnorr(&a["method"]),
            map(&a["mutation"]),
            sig,
            version,
        ),
        "removeSchnorrJubjubVerificationMethod" => {
            c::removeSchnorrJubjubVerificationMethod(context, w, string(&a["id"]), sig, version)
        }
        "setVerificationMethodRelation" => c::setVerificationMethodRelation(
            context,
            w,
            relation(&a["relation"]),
            string(&a["id"]),
            set(&a["mutation"]),
            sig,
            version,
        ),
        "setService" => c::setService(
            context,
            w,
            service(&a["service"]),
            map(&a["mutation"]),
            sig,
            version,
        ),
        "removeService" => c::removeService(context, w, string(&a["id"]), sig, version),
        "deactivate" => c::deactivate(context, w, sig, version),
        "verifySchnorrJubjubDigestSignature" => c::verifySchnorrJubjubDigestSignature(
            context,
            w,
            string(&a["id"]),
            r::FixedVector::new([1u64, 2, 3, 4].map(r::Field::from)),
            sig,
        ),
        other => panic!("unreviewed lifecycle export: {other}"),
    }
}
