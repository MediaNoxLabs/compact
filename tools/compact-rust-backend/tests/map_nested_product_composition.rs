// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_backend::{ir::Contract, render_with_capabilities};
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    serde_json::from_str(match name {
        "flat" => include_str!("map-nested-product/flat-enum.json"),
        "nested" => include_str!("map-nested-product/nested-enum.json"),
        "wide" => include_str!("map-nested-product/nested-enum-wide.json"),
        _ => unreachable!(),
    })
    .unwrap()
}

fn recorded(value: Value, name: &str) -> bool {
    let Ok(contract) = serde_json::from_value::<Contract>(value) else {
        return false;
    };
    render_with_capabilities(&contract)
        .ok()
        .is_some_and(|rendered| {
            rendered
                .capabilities
                .circuits
                .iter()
                .any(|row| row.name == name && row.recorded)
        })
}

fn circuit<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    value["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["name"] == name)
        .unwrap()
}

fn first_kind_mut<'a>(value: &'a mut Value, kind: &str) -> Option<&'a mut Value> {
    if value.get("kind").and_then(Value::as_str) == Some(kind) {
        return Some(value);
    }
    match value {
        Value::Array(values) => values
            .iter_mut()
            .find_map(|value| first_kind_mut(value, kind)),
        Value::Object(values) => values
            .values_mut()
            .find_map(|value| first_kind_mut(value, kind)),
        _ => None,
    }
}

#[test]
fn declared_flat_and_nested_enum_products_record_without_name_arity_or_field_order_gate() {
    for (fixture_name, names) in [
        ("flat", &["put", "remove"][..]),
        ("nested", &["upsert", "remove"][..]),
        ("wide", &["put", "drop"][..]),
    ] {
        let value = fixture(fixture_name);
        let contract: Contract = serde_json::from_value(value.clone()).unwrap();
        let rendered = render_with_capabilities(&contract).unwrap();
        for name in names {
            assert!(recorded(value.clone(), name), "{fixture_name}.{name}");
        }
        assert!(rendered.source.contains("record_insert"));
        assert!(rendered.source.contains("record_remove"));
    }
}

#[test]
fn nested_product_mutation_requires_exact_declared_value_and_key_slot() {
    for mutation in ["declared_name", "nested_name", "key", "slot", "empty_child"] {
        let mut value = fixture("nested");
        match mutation {
            "declared_name" => {
                circuit(&mut value, "upsert")["parameters"][0]["ty"]["name"] = json!("Twin")
            }
            "nested_name" => {
                circuit(&mut value, "upsert")["parameters"][0]["ty"]["fields"][1]["ty"]["name"] =
                    json!("TwinJwk")
            }
            "key" => value["ledger_fields"][0]["declaration"]["key"] = json!({"kind":"field"}),
            "slot" => value["ledger_fields"][0]["path"] = json!([9]),
            "empty_child" => {
                value["ledger_fields"][0]["declaration"]["value"]["fields"][1]["ty"]["fields"] =
                    json!([])
            }
            _ => unreachable!(),
        }
        assert!(!recorded(value, "upsert"), "{mutation}");
    }
}

#[test]
fn nested_product_helper_audit_rejects_hidden_effects_cycle_and_wrong_enum_type() {
    for mutation in ["hidden_branch", "helper_cycle", "wrong_arity", "wrong_enum"] {
        let mut value = fixture("nested");
        match mutation {
            "hidden_branch" => {
                let sequence =
                    first_kind_mut(&mut circuit(&mut value, "upsert")["actions"], "sequence")
                        .unwrap();
                sequence["actions"].as_array_mut().unwrap().push(json!({
                    "kind":"if", "condition":{"kind":"boolean","value":false},
                    "then":{"kind":"map_reset","field":"methods","index":0},
                    "otherwise":{"kind":"sequence","actions":[]}
                }));
            }
            "helper_cycle" => circuit(&mut value, "bump")["actions"]
                .as_array_mut()
                .unwrap()
                .push(json!({"kind":"circuit_call","name":"bump","arguments":[]})),
            "wrong_arity" => {
                let call =
                    first_kind_mut(&mut circuit(&mut value, "upsert")["actions"], "pure_call")
                        .unwrap();
                assert_eq!(call["name"], "allowed");
                call["arguments"] = json!([]);
            }
            "wrong_enum" => {
                let mut wide = fixture("wide");
                let inequality =
                    first_kind_mut(&mut circuit(&mut wide, "put")["actions"], "not_equal").unwrap();
                inequality["right"]["ty"]["name"] = json!("OtherKind");
                value = wide;
            }
            _ => unreachable!(),
        }
        let name = if mutation == "wrong_enum" {
            "put"
        } else {
            "upsert"
        };
        assert!(!recorded(value, name), "{mutation}");
    }
}

#[test]
fn nested_product_domain_does_not_admit_unreviewed_unsigned_or_empty_values() {
    for source in [
        include_str!("map-composition/mixed_map.json"),
        include_str!("map-composition/empty_map.json"),
    ] {
        let value: Value = serde_json::from_str(source).unwrap();
        assert!(!recorded(value, "put"));
    }
}
