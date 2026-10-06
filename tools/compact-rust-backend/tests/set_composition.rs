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
        "set" => include_str!("set-composition/set_string.json"),
        "guard" => include_str!("set-composition/pure_unit_guard.json"),
        "digest" => include_str!("set-composition/opaque_digest.json"),
        _ => unreachable!(),
    })
    .unwrap()
}
fn declaration<'a>(v: &'a mut Value, class: &str, name: &str) -> &'a mut Value {
    v[class]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn pure(v: &mut Value) -> &mut Value {
    declaration(v, "circuits", "defined")
}
fn recorded(v: Value, name: &str) -> bool {
    let ir: Contract = serde_json::from_value(v).unwrap();
    render_with_capabilities(&ir).is_ok_and(|r| {
        r.capabilities
            .circuits
            .iter()
            .any(|c| c.name == name && c.recorded)
    })
}
fn edit_kind(v: &mut Value, kind: &str, edit: &mut impl FnMut(&mut Value)) {
    if v.get("kind").is_some_and(|k| k == kind) {
        edit(v);
    }
    match v {
        Value::Object(fields) => {
            for child in fields.values_mut() {
                edit_kind(child, kind, edit);
            }
        }
        Value::Array(values) => {
            for child in values {
                edit_kind(child, kind, edit);
            }
        }
        _ => (),
    }
}
#[test]
fn string_set_and_unit_guard_composition_preserve_old_controls() {
    for name in ["insert_control", "member_control", "mutate"] {
        assert!(recorded(fixture("set"), name), "{name}");
    }
    assert!(!recorded(fixture("set"), "remove_control")); // No new global standalone profile.
    assert!(recorded(fixture("guard"), "run"));
    assert!(recorded(fixture("digest"), "run"));
}
#[test]
fn pure_unit_guards_validate_result_steps_conditions_arity_and_cycles() {
    for kind in ["result", "step", "condition", "arity", "cycle", "argument"] {
        let mut v = fixture("guard");
        match kind {
            "result" => pure(&mut v)["result"] = json!({"kind":"boolean"}),
            "step" => pure(&mut v)["body"]["steps"][0] = json!({"kind":"boolean","value":true}),
            "condition" => {
                pure(&mut v)["body"]["steps"][0]["condition"] =
                    json!({"kind":"field_literal","value":"1"})
            }
            "arity" => pure(&mut v)["parameters"] = json!([]),
            "cycle" => {
                pure(&mut v)["body"] = json!({"kind":"call","name":"defined","arguments":[{"kind":"parameter","name":"m"}]})
            }
            "argument" => {
                declaration(&mut v, "stateful_circuits", "run")["actions"][0]["arguments"][0] =
                    json!({"kind":"boolean","value":true})
            }
            _ => unreachable!(),
        }
        assert!(!recorded(v, "run"), "{kind}");
    }
}
#[test]
fn pure_unit_guards_audit_unused_and_unselected_effects() {
    for unused in [false, true] {
        for hidden in [
            json!({"kind":"cell_read","field":"active","index":0}),
            json!({"kind":"native_witness_call","builtin":"own_public_key"}),
            json!({"kind":"witness_call","name":"unknown","arguments":[]}),
            json!({"kind":"map_member","field":"active","index":0,"key":{"kind":"field_literal","value":"1"}}),
        ] {
            let mut v = fixture("guard");
            let body = pure(&mut v)["body"].clone();
            pure(&mut v)["body"] = if unused {
                json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"boolean"},"value":hidden}],"body":body})
            } else {
                json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":body,"otherwise":{"kind":"assert","condition":hidden,"message":"hidden"}})
            };
            assert!(!recorded(v, "run"));
        }
    }
}
#[test]
fn string_set_slot_key_and_enum_identity_stay_typed() {
    for kind in [
        "slot",
        "path",
        "key",
        "declaration",
        "enum",
        "escape",
        "hidden_map",
    ] {
        let mut v = fixture("set");
        match kind {
            "slot" => edit_kind(&mut v, "set_remove", &mut |n| n["index"] = json!(9)),
            "path" => v["ledger_fields"][0]["path"] = json!([9, 0]),
            "key" => edit_kind(&mut v, "set_remove", &mut |n| {
                n["value"] = json!({"kind":"field_literal","value":"1"})
            }),
            "declaration" => {
                v["ledger_fields"][0]["declaration"] =
                    json!({"kind":"cell","ty":{"kind":"opaque_string"}})
            }
            "enum" => pure(&mut v)["parameters"][0]["ty"]["name"] = json!("DifferentEnum"),
            "escape" => edit_kind(&mut v, "set_remove", &mut |n| {
                n["value"] = json!({"kind":"parameter","name":"escaped"})
            }),
            "hidden_map" => {
                v["ledger_fields"].as_array_mut().unwrap().push(json!({
                    "id":"other_map","index":2,"path":[2],
                    "declaration":{"kind":"map","key":{"kind":"field"},"value":{"kind":"field"}}
                }));
                let c = declaration(&mut v, "stateful_circuits", "mutate");
                c["actions"].as_array_mut().unwrap().push(json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":{"kind":"sequence","actions":[]},"otherwise":{"kind":"map_remove","field":"other_map","index":2,"key":{"kind":"field_literal","value":"1"}}}));
            }
            _ => unreachable!(),
        }
        assert!(!recorded(v, "mutate"), "{kind}");
    }
}
#[test]
fn renamed_declarations_record_without_contract_or_helper_name_gates() {
    let mut v = fixture("set");
    pure(&mut v)["name"] = json!("validate_mutation_renamed");
    edit_kind(&mut v, "pure_call", &mut |n| {
        if n["name"] == "defined" {
            n["name"] = json!("validate_mutation_renamed");
        }
    });
    v["ledger_fields"][0]["id"] = json!("labels");
    for kind in ["set_insert", "set_remove", "set_member"] {
        edit_kind(&mut v, kind, &mut |n| n["field"] = json!("labels"));
    }
    assert!(recorded(v, "mutate"));
}

#[test]
fn actual_unit_guards_retain_local_scope_and_transitive_calls() {
    for name in ["local_control", "transitive_control"] {
        assert!(recorded(fixture("guard"), name), "{name}");
    }
    for mutation in ["scope", "type", "hidden_read"] {
        let mut v = fixture("guard");
        let local = declaration(&mut v, "circuits", "localGuard");
        let binding = &mut local["body"]["steps"][0];
        match mutation {
            "scope" => binding["body"]["condition"]["name"] = json!("escaped"),
            "type" => binding["bindings"][0]["ty"] = json!({"kind":"field"}),
            "hidden_read" => {
                binding["bindings"][0]["value"] =
                    json!({"kind":"cell_read","field":"active","index":0});
            }
            _ => unreachable!(),
        }
        assert!(!recorded(v, "transitive_control"), "{mutation}");
    }
    let mut cycle = fixture("guard");
    declaration(&mut cycle, "circuits", "localGuard")["body"] = json!({
        "kind":"call", "name":"transitiveGuard",
        "arguments":[{"kind":"parameter","name":"m"}]
    });
    assert!(!recorded(cycle, "transitive_control"));
}
