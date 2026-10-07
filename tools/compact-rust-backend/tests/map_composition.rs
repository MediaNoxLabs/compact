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

fn fixture(name: &str) -> Value {
    serde_json::from_str(match name {
        "service" => include_str!("map-composition/service_mutation.json"),
        "digest" => include_str!("map-composition/service_digest.json"),
        "pair" => include_str!("map-composition/flat2_map.json"),
        "quad" => include_str!("map-composition/flat4_map.json"),
        "empty" => include_str!("map-composition/empty_map.json"),
        "mixed" => include_str!("map-composition/mixed_map.json"),
        "nested" => include_str!("map-composition/nested_map.json"),
        _ => unreachable!(),
    })
    .unwrap()
}

fn render(value: Value) -> compact_rust_backend::RenderedContract {
    render_with_capabilities(&serde_json::from_value::<Contract>(value).unwrap()).unwrap()
}

fn recorded(value: Value, circuit: &str) -> bool {
    render_with_capabilities(&serde_json::from_value::<Contract>(value).unwrap())
        .ok()
        .is_some_and(|rendered| {
            rendered
                .capabilities
                .circuits
                .iter()
                .find(|row| row.name == circuit)
                .is_some_and(|row| row.recorded)
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
fn service_and_arbitrary_nonempty_flat_string_products_record() {
    for (fixture_name, circuits) in [
        ("service", &["set_service", "remove_service"][..]),
        ("pair", &["put", "drop"][..]),
        ("quad", &["put", "drop"][..]),
    ] {
        let rendered = render(fixture(fixture_name));
        for name in circuits {
            assert!(
                rendered
                    .capabilities
                    .circuits
                    .iter()
                    .any(|row| row.name == *name && row.recorded),
                "{fixture_name}.{name}"
            );
        }
        assert!(rendered.source.contains("record_insert"), "{fixture_name}");
        assert!(rendered.source.contains("record_remove"), "{fixture_name}");
    }
    assert!(recorded(fixture("digest"), "run"));
}

#[test]
fn empty_and_unsupported_mixed_map_values_remain_native_only() {
    for name in ["empty", "mixed"] {
        assert!(!recorded(fixture(name), "put"), "{name}");
    }
}

#[test]
fn nonempty_nested_string_product_records() {
    assert!(recorded(fixture("nested"), "put"));
}

#[test]
fn map_composition_requires_declared_slot_and_exact_operand_types() {
    for mutation in ["slot", "path", "key", "value", "declaration"] {
        let mut value = fixture("pair");
        match mutation {
            "slot" => circuit(&mut value, "put")["actions"][0]["index"] = json!(9),
            "path" => value["ledger_fields"][0]["path"] = json!([9]),
            "key" => {
                circuit(&mut value, "put")["actions"][0]["key"] =
                    json!({"kind":"field_literal","value":"1"})
            }
            "value" => {
                circuit(&mut value, "put")["actions"][0]["value"] =
                    json!({"kind":"parameter","name":"key"})
            }
            "declaration" => {
                value["ledger_fields"][0]["declaration"]["key"] = json!({"kind":"field"})
            }
            _ => unreachable!(),
        }
        assert!(!recorded(value, "put"), "{mutation}");
    }
}

#[test]
fn hidden_unselected_and_unused_map_effects_are_audited() {
    for unused in [false, true] {
        let mut value = fixture("pair");
        let hidden = json!({"kind":"map_reset","field":"entries","index":0});
        let otherwise = if unused {
            json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"opaque_string"},"value":{"kind":"parameter","name":"key"}}],"action":hidden})
        } else {
            hidden
        };
        circuit(&mut value, "put")["actions"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "kind":"if", "condition":{"kind":"boolean","value":true},
                "then":{"kind":"sequence","actions":[]},
                "otherwise":otherwise
            }));
        assert!(
            !render(value)
                .capabilities
                .circuits
                .iter()
                .find(|row| row.name == "put")
                .unwrap()
                .recorded
        );
    }
    let mut value = fixture("pair");
    circuit(&mut value, "put")["actions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "kind":"let",
            "bindings":[{"name":"unused","ty":{"kind":"boolean"},
                         "value":{"kind":"map_is_empty","field":"entries","index":0}}],
            "action":{"kind":"sequence","actions":[]}
        }));
    assert!(
        !render(value)
            .capabilities
            .circuits
            .iter()
            .find(|row| row.name == "put")
            .unwrap()
            .recorded
    );
}

#[test]
fn declaration_names_do_not_gate_map_composition() {
    let mut value = fixture("pair");
    value["ledger_fields"][0]["id"] = json!("labels");
    for row in value["stateful_circuits"].as_array_mut().unwrap() {
        let name = row["name"].as_str().unwrap().to_owned();
        if name == "put" || name == "drop" {
            let serialized = serde_json::to_string(row)
                .unwrap()
                .replace("\"entries\"", "\"labels\"");
            *row = serde_json::from_str(&serialized).unwrap();
        }
    }
    assert!(recorded(value.clone(), "put"));
    assert!(recorded(value, "drop"));
}

#[test]
fn map_composition_rejects_struct_identity_scope_and_helper_graph_mutants() {
    for mutation in [
        "struct_name",
        "escaped_local",
        "helper_arity",
        "helper_cycle",
        "helper_hidden_map",
    ] {
        let mut value = fixture("pair");
        match mutation {
            "struct_name" => {
                // Field types/arity agree, but the declared value is Pair, not Twin.
                circuit(&mut value, "put")["parameters"][1]["ty"]["name"] = json!("Twin");
            }
            "escaped_local" => {
                let put = circuit(&mut value, "put");
                put["actions"].as_array_mut().unwrap().insert(
                    0,
                    json!({
                        "kind":"let", "bindings":[{"name":"scoped","ty":{"kind":"opaque_string"},
                            "value":{"kind":"parameter","name":"key"}}],
                        "action":{"kind":"sequence","actions":[]}
                    }),
                );
                put["actions"][1]["key"] = json!({"kind":"parameter","name":"scoped"});
            }
            "helper_arity" => {
                circuit(&mut value, "put")["actions"][1]["arguments"] =
                    json!([{"kind":"parameter","name":"key"}])
            }
            "helper_cycle" => circuit(&mut value, "bump")["actions"]
                .as_array_mut()
                .unwrap()
                .push(json!({"kind":"circuit_call","name":"bump","arguments":[]})),
            "helper_hidden_map" => circuit(&mut value, "bump")["actions"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "kind":"if","condition":{"kind":"boolean","value":true},
                    "then":{"kind":"sequence","actions":[]},
                    "otherwise":{"kind":"map_reset","field":"entries","index":0}
                })),
            _ => unreachable!(),
        }
        assert!(!recorded(value, "put"), "{mutation}");
    }
}
