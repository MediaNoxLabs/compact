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

use super::{codec, lifecycle_witness::Witness};
use compact_rust_did_adoption_fixture::{ledger_contract as c, runtime as r, types};
use serde_json::Value;

pub fn invoke_recorded(
    context: r::context::CircuitContext<u64>,
    witness: &Witness,
    row: &Value,
) -> Result<r::recording::RecordedCircuitResult<u64, ()>, r::CompactError> {
    let signature = types::SchnorrSignature {
        announcement: codec::point("2"),
        response: codec::field_hex(row["responseHex"].as_str().unwrap()),
    };
    let version = codec::version(row["version"].as_str().unwrap());
    match row["name"].as_str().unwrap() {
        "setService" => c::recorded::setService(
            context,
            witness,
            codec::service(&row["args"]["service"]),
            codec::map(&row["args"]["mutation"]),
            signature,
            version,
        ),
        "removeService" => c::recorded::removeService(
            context,
            witness,
            codec::string(&row["args"]["id"]),
            signature,
            version,
        ),
        "deactivate" => c::recorded::deactivate(context, witness, signature, version),
        other => panic!("not a Service recording case: {other}"),
    }
}
