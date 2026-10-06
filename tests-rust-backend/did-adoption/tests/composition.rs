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

use compact_rust_did_adoption_fixture as fixture;

use fixture::ledger_contract as c;
use midnight_compact_runtime as runtime;
use runtime::context::CircuitContext;
use runtime::{BoundedUint, CompactError, Field};
use serde_json::{Value, json};
use std::cell::RefCell;
#[path = "../support/witness.rs"]
mod support;
#[allow(dead_code)]
#[path = "../../support/oracle_recorded_trace.rs"]
mod trace;
use support::Witness;
fn state(row: &Value) -> runtime::ledger::ContractState<runtime::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(row["before"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap()
}
fn context(row: &Value) -> CircuitContext<u64> {
    CircuitContext::from_contract_state(7, runtime::ledger::ContractAddress::default(), &state(row))
}
#[test]
fn independent_typescript_native_recorded_order_state_and_failures() {
    let capture: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/unit-composition.json"
    ))
    .unwrap();
    let rows: Vec<_> = capture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"] == "did")
        .collect();
    assert_eq!(rows.len(), 12);
    for row in rows {
        let signature = fixture::types::SchnorrSignature {
            announcement: runtime::ec_mul_generator(Field::from(2u64)).unwrap(),
            response: Field::from_le_bytes(
                &hex::decode(row["responseHex"].as_str().unwrap()).unwrap(),
            )
            .unwrap(),
        };
        let expected =
            BoundedUint::new(row["expected"].as_str().unwrap().parse().unwrap()).unwrap();
        let native_witness = Witness {
            options: row["options"].clone(),
            calls: RefCell::default(),
        };
        let recorded_witness = Witness {
            options: row["options"].clone(),
            calls: RefCell::default(),
        };
        let native = c::deactivate(context(row), &native_witness, signature.clone(), expected);
        let recorded =
            c::recorded::deactivate(context(row), &recorded_witness, signature, expected);
        assert_eq!(
            json!(*native_witness.calls.borrow()),
            row["witnessCalls"],
            "{}",
            row["id"]
        );
        assert_eq!(
            json!(*recorded_witness.calls.borrow()),
            row["witnessCalls"],
            "{}",
            row["id"]
        );
        if row.get("error").is_some() {
            let n = native.err().expect("native rejection");
            let r = recorded.err().expect("recorded rejection");
            assert_eq!(n, r, "{}", row["id"]);
            let expected = row["error"].as_str().unwrap();
            if expected.contains("arithmetic overflow") {
                assert!(format!("{n:?}").contains("ArithmeticOverflow"), "{n:?}");
            } else {
                assert_eq!(
                    n,
                    CompactError::AssertionFailed(
                        expected
                            .strip_prefix("failed assert: ")
                            .unwrap_or(expected)
                            .into()
                    ),
                    "{}",
                    row["id"]
                );
            }
        } else {
            let native = native.unwrap();
            let recorded = recorded.unwrap();
            let template = state(row);
            trace::assert_trace(&native, &recorded, row, |s| {
                let mut template = template.clone();
                template.data = runtime::ledger::ChargedState::new(s);
                let mut bytes = vec![];
                midnight_serialize::tagged_serialize(&template, &mut bytes).unwrap();
                hex::encode(bytes)
            });
            assert_eq!(
                native.context.private_state,
                row["privateAfter"].as_u64().unwrap()
            );
            if row["id"] == "crypto/unguarded-inactive" {
                assert_eq!(row["queries"].as_array().unwrap().len(), 2);
                assert_eq!(row["before"], row["after"]);
                assert_eq!(native.context.private_state, 8);
            }
        }
    }
}
