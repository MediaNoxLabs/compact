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

use compact_rust_backend::{ir::Contract, render_with_capabilities};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("map-point-composition/point_nested.json")).unwrap()
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

#[test]
fn flat_point_product_mutations_and_nested_value_key_only_member_record() {
    let contract: Contract = serde_json::from_value(fixture()).unwrap();
    let rendered = render_with_capabilities(&contract).unwrap();
    for name in ["upsert", "remove"] {
        assert!(
            rendered
                .capabilities
                .circuits
                .iter()
                .any(|row| row.name == name && row.recorded)
        );
    }
    assert!(rendered.source.contains("record_member"));
    assert!(rendered.source.contains("record_insert"));
    assert!(rendered.source.contains("record_remove"));
    // The first membership is on a Map with nested V; only its key is read.
    assert!(
        rendered
            .source
            .contains("crate::ledger_slots::nestedMethods")
    );
    assert!(rendered.source.matches("record_member(frame").count() >= 2);
}

#[test]
fn renamed_three_and_four_field_scalar_products_record_without_name_or_arity_gate() {
    for source in [
        include_str!("map-point-composition/flat3.json"),
        include_str!("map-point-composition/flat4.json"),
    ] {
        let value: Value = serde_json::from_str(source).unwrap();
        for name in ["put", "drop"] {
            assert!(recorded(value.clone(), name), "{name}");
        }
    }
}

#[test]
fn nested_string_product_records_and_empty_product_remains_outside_domain() {
    let nested: Value =
        serde_json::from_str(include_str!("map-composition/nested_map.json")).unwrap();
    assert!(recorded(nested, "put"));
    let empty: Value =
        serde_json::from_str(include_str!("map-composition/empty_map.json")).unwrap();
    assert!(!recorded(empty, "put"));
}

#[test]
fn boolean_helper_rejects_hidden_witness_queries_and_effects() {
    for mutation in ["witness", "counter", "effect", "recursive", "wrong_result"] {
        let mut value = fixture();
        let helper = circuit(&mut value, "exists");
        match mutation {
            "witness" => {
                value["witnesses"] =
                    json!([{"name":"hidden","parameters":[],"result":{"kind":"boolean"}}]);
                circuit(&mut value, "exists")["return_value"]["value"]["otherwise"] =
                    json!({"kind":"witness_call","name":"hidden","arguments":[]});
            }
            "counter" => {
                helper["return_value"]["value"]["otherwise"] = json!({
                    "kind":"equal",
                    "left":{"kind":"counter_read","field":"count","index":2},
                    "right":{"kind":"unsigned_literal","value":"0","max":"18446744073709551615"}
                })
            }
            "effect" => {
                helper["actions"] = json!([{"kind":"map_remove","field":"pointMethods","index":0,"key":{"kind":"parameter","name":"id"}}])
            }
            "recursive" => {
                helper["return_value"]["value"]["otherwise"] = json!({"kind":"call","name":"exists","arguments":[{"kind":"parameter","name":"id"}]})
            }
            "wrong_result" => helper["result"] = json!({"kind":"field"}),
            _ => unreachable!(),
        }
        assert!(!recorded(value, "upsert"), "{mutation}");
    }
}

#[test]
fn nested_value_membership_is_bounded_to_an_audited_boolean_helper() {
    for unselected in [false, true] {
        let mut value = fixture();
        let root = circuit(&mut value, "upsert");
        let original = root["actions"][0].clone();
        let hidden = json!({
            "kind":"let",
            "bindings":[{"name":"hidden", "ty":{"kind":"boolean"}, "value":{
                "kind":"map_member", "field":"nestedMethods", "index":1,
                "key":{"kind":"struct_field", "value":{"kind":"parameter","name":"method"}, "field":"id", "index":0}
            }}],
            "action":{"kind":"sequence","actions":[]}
        });
        root["actions"] = if unselected {
            json!([{"kind":"if", "condition":{"kind":"boolean","value":false},
                    "then":hidden,"otherwise":original}])
        } else {
            let mut hidden = hidden;
            hidden["action"] = original;
            json!([hidden])
        };
        assert!(!recorded(value, "upsert"), "unselected={unselected}");
    }
}

#[test]
fn point_map_requires_declared_key_value_and_scoped_helper_arguments() {
    for mutation in [
        "key",
        "value",
        "slot",
        "helper_arity",
        "struct_name",
        "nested_mutation",
    ] {
        let mut value = fixture();
        match mutation {
            "key" => {
                let root = &mut circuit(&mut value, "upsert")["actions"][0]["action"]["action"]["actions"]
                    [1]["action"];
                assert_eq!(root["kind"], "map_insert");
                root["key"] = json!({"kind":"field_literal","value":"1"});
            }
            "value" => {
                let root = &mut circuit(&mut value, "upsert")["actions"][0]["action"]["action"]["actions"]
                    [1]["action"];
                root["value"] = json!({"kind":"parameter","name":"doUpdate"});
            }
            "slot" => value["ledger_fields"][0]["path"] = json!([9]),
            "helper_arity" => {
                let call = &mut circuit(&mut value, "upsert")["actions"][0]["action"]["action"]["actions"]
                    [0]["otherwise"]["condition"]["condition"];
                assert_eq!(call["kind"], "call");
                call["arguments"] = json!([]);
            }
            "struct_name" => {
                circuit(&mut value, "upsert")["parameters"][0]["ty"]["name"] = json!("Twin")
            }
            "nested_mutation" => {
                value["ledger_fields"][0]["declaration"]["value"] =
                    value["ledger_fields"][1]["declaration"]["value"].clone()
            }
            _ => unreachable!(),
        }
        assert!(!recorded(value, "upsert"), "{mutation}");
    }
}
