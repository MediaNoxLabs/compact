// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_did_digest_read_reducer_fixture::{ledger_contract as c, runtime as r, types};
use r::context::ConstructorContext;
use serde_json::Value;
use std::collections::BTreeSet;

#[test]
fn constructor_derives_exact_ts_prestate_for_each_captured_case() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    assert_eq!(
        capture["format"],
        "compact-did-digest-read-reducer-capture/v1"
    );
    let rows = capture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows.iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["matching", "missing", "witness-denied", "wrong-point"])
    );
    // The independent capture.mjs constructor uses this id and generator scalar
    // for every row; expectedKey is a later verify argument, not a seed value.
    let id = r::OpaqueString("reducer-renamed".into());
    let point = r::ec_mul_generator(r::Field::from(5u64)).unwrap();
    for row in rows {
        let label = row["id"].as_str().unwrap();
        let private = row["privateBefore"].as_u64().unwrap();
        let initial =
            c::initial_state(ConstructorContext::new(private), id.clone(), point).unwrap();
        let ts: r::ledger::ContractState<r::ledger::DefaultDB> =
            midnight_serialize::tagged_deserialize(
                &mut hex::decode(row["before"].as_str().unwrap())
                    .unwrap()
                    .as_slice(),
            )
            .unwrap();
        assert_eq!(
            initial.ledger_state, ts.data,
            "constructor-derived TS state: {label}"
        );
        let view = c::PublicStateView::from(&initial);
        assert!(view.active().unwrap(), "active: {label}");
        let methods = view.methods().unwrap();
        assert_eq!(methods.size().unwrap().value(), 1, "size: {label}");
        assert_eq!(
            methods.lookup(id.clone()).unwrap(),
            types::Method {
                verificationPoint: point,
                marker: id.clone()
            },
            "complete Method: {label}"
        );
        assert_eq!(
            initial
                .into_circuit_context(Default::default())
                .private_state,
            private,
            "private state: {label}"
        );
    }
}
