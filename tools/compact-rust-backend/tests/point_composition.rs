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
        "store" => include_str!("point-composition/point_store.json"),
        "guard" => include_str!("point-composition/point_guard.json"),
        "digest" => include_str!("point-composition/point_digest.json"),
        _ => unreachable!(),
    })
    .unwrap()
}
fn circuit<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn pure<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
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
#[test]
fn point_controls_and_new_composition_leaves_record() {
    for name in ["through_helper", "direct_control", "guarded"] {
        assert!(recorded(fixture("store"), name));
    }
    for name in [
        "scalar_control",
        "point_write_only",
        "pure_projection_only",
        "update",
    ] {
        assert!(recorded(fixture("digest"), name), "{name}");
    }
    assert!(recorded(fixture("guard"), "check"));
}
#[test]
fn coordinates_and_inequality_validate_actual_operand_types() {
    for kind in ["boolean", "field_literal", "parameter"] {
        let mut v = fixture("guard");
        let input = match kind {
            "boolean" => json!({"kind":"boolean","value":true}),
            "field_literal" => json!({"kind":"field_literal","value":"1"}),
            _ => json!({"kind":"parameter","name":"escaped"}),
        };
        circuit(&mut v, "changes")["actions"][0]["condition"]["condition"]["left"]["value"] = input;
        assert!(!recorded(v, "check"), "{kind}");
    }
    for both in [false, true] {
        let mut v = fixture("guard");
        let condition = &mut circuit(&mut v, "changes")["actions"][0]["condition"]["condition"];
        condition["left"] = json!({"kind":"boolean","value":true});
        if both {
            condition["right"] = json!({"kind":"boolean","value":false});
        }
        assert!(!recorded(v, "check"));
    }
}
#[test]
fn pure_coordinate_audit_rejects_hidden_reads_witnesses_and_cycles() {
    for unused in [false, true] {
        for witness in [false, true] {
            let mut v = fixture("digest");
            let hidden = if witness {
                json!({"kind":"witness_call","name":"authorize","arguments":[{"kind":"field_literal","value":"1"}]})
            } else {
                json!({"kind":"cell_read","field":"active","index":1})
            };
            let original = pure(&mut v, "digest")["body"].clone();
            pure(&mut v, "digest")["body"] = if unused {
                json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"boolean"},"value":hidden}],"body":original})
            } else {
                json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":original,"otherwise":{"kind":"if","condition":hidden,"then":{"kind":"field_literal","value":"1"},"otherwise":{"kind":"field_literal","value":"2"}}})
            };
            assert!(!recorded(v, "update"));
        }
    }
    let mut v = fixture("digest");
    pure(&mut v, "digest")["body"] =
        json!({"kind":"call","name":"digest","arguments":[{"kind":"parameter","name":"value"}]});
    assert!(!recorded(v, "update"));
}
#[test]
fn helper_scope_type_slot_cycle_and_unselected_effects_stay_checked() {
    for mutation in ["slot", "cycle", "arity", "effect", "type"] {
        let mut v = fixture("guard");
        match mutation {
            "slot"=>circuit(&mut v,"changes")["actions"][0]["condition"]["condition"]["right"]["value"]["index"]=json!(99),
            "cycle"=>circuit(&mut v,"changes")["actions"].as_array_mut().unwrap().push(json!({"kind":"circuit_call","name":"changes","arguments":[{"kind":"parameter","name":"value"}]})),
            "arity"=>{circuit(&mut v,"changes")["parameters"]=json!([]);},
            "type"=>{circuit(&mut v,"changes")["parameters"][0]["ty"]=json!({"kind":"field"});},
            "effect"=>circuit(&mut v,"changes")["actions"].as_array_mut().unwrap().push(json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":{"kind":"sequence","actions":[]},"otherwise":{"kind":"expression","value":{"kind":"native_witness_call","builtin":"own_public_key"}}})),
            _=>unreachable!(),
        }
        assert!(!recorded(v, "check"), "{mutation}");
    }
}

#[test]
fn point_writes_and_pure_result_types_are_checked_without_name_gates() {
    for mutation in ["slot", "field", "value", "result"] {
        let mut v = fixture("digest");
        match mutation {
            "slot" => circuit(&mut v, "update")["actions"][1]["index"] = json!(99),
            "field" => circuit(&mut v, "update")["actions"][1]["field"] = json!("active"),
            "value" => {
                circuit(&mut v, "update")["actions"][1]["value"] =
                    json!({"kind": "field_literal", "value": "1"});
            }
            "result" => pure(&mut v, "digest")["result"] = json!({"kind": "boolean"}),
            _ => unreachable!(),
        }
        assert!(!recorded(v, "update"), "{mutation}");
    }
    let mut renamed = fixture("digest");
    pure(&mut renamed, "digest")["name"] = json!("coordinates_digest_renamed");
    for entry in ["update", "pure_projection_only"] {
        circuit(&mut renamed, entry)["actions"][0]["arguments"][0]["value"]["name"] =
            json!("coordinates_digest_renamed");
    }
    assert!(recorded(renamed, "update"));
}
