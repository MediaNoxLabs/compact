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
fn fixture(chunked: bool) -> Value {
    serde_json::from_str(if chunked {
        include_str!("unit-composition/chunked.json")
    } else {
        include_str!("unit-composition/flat.json")
    })
    .unwrap()
}
fn recorded(value: Value) -> bool {
    let contract: Contract = serde_json::from_value(value).unwrap();
    render_with_capabilities(&contract).is_ok_and(|r| {
        r.capabilities
            .circuits
            .iter()
            .find(|c| c.name == "close")
            .is_some_and(|c| c.recorded)
    })
}
fn circuit<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
#[test]
fn flat_and_chunked_declared_helpers_compose_on_one_recording_frame() {
    for chunked in [false, true] {
        assert!(recorded(fixture(chunked)), "chunked={chunked}");
    }
}
#[test]
fn complete_audit_rejects_hidden_unsupported_effects_in_unselected_branch() {
    for chunked in [false, true] {
        let mut v = fixture(chunked);
        circuit(&mut v,"guard")["actions"].as_array_mut().unwrap().push(json!({
            "kind":"if", "condition":{"kind":"boolean","value":true},
            "then":{"kind":"sequence","actions":[]},
            "otherwise":{"kind":"expression","value":{"kind":"kernel_self","ty":{"kind":"struct","name":"ContractAddress","fields":[{"name":"bytes","ty":{"kind":"bytes","length":32}}]}}}
        }));
        assert!(!recorded(v));
    }
}
#[test]
fn wrong_helper_arity_type_and_scope_never_fall_back_to_native_helper() {
    let mut v = fixture(false);
    circuit(&mut v, "close")["actions"][0]["arguments"] = json!([]);
    assert!(!recorded(v));
    let mut v = fixture(false);
    circuit(&mut v, "guard")["parameters"][0]["ty"] = json!({"kind":"boolean"});
    assert!(!recorded(v));
    let mut v = fixture(false);
    circuit(&mut v, "guard")["actions"][0]["condition"]["left"] =
        json!({"kind":"parameter","name":"caller_only"});
    assert!(!recorded(v));
}
#[test]
fn cycles_missing_helpers_and_wrong_slot_identity_are_refused() {
    let mut v = fixture(false);
    circuit(&mut v,"guard")["actions"].as_array_mut().unwrap().push(json!({"kind":"circuit_call","name":"guard","arguments":[{"kind":"parameter","name":"expected"}]}));
    assert!(!recorded(v));
    let mut v = fixture(false);
    circuit(&mut v, "close")["actions"][0]["name"] = json!("missing");
    assert!(!recorded(v));
    let mut v = fixture(true);
    circuit(&mut v, "guard")["actions"][0]["condition"]["right"]["index"] = json!(0);
    assert!(!recorded(v));
}

fn crypto() -> Value {
    serde_json::from_str(include_str!("unit-composition/crypto.json")).unwrap()
}
#[test]
fn native_crypto_helpers_cannot_hide_a_public_read_and_fall_back_to_recorded() {
    assert!(recorded(crypto()));
    for unused_binding in [false, true] {
        let mut v = crypto();
        let read = json!({"kind":"cell_read","field":"active","index":1});
        let hidden = if unused_binding {
            json!({"kind":"let","bindings":[{"name":"unused_read","ty":{"kind":"boolean"},"value":read}],"body":{"kind":"boolean","value":true}})
        } else {
            json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":{"kind":"boolean","value":true},"otherwise":read})
        };
        circuit(&mut v, "schnorrVerify")["actions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"assert","condition":hidden,"message":"hidden public effect"}));
        assert!(!recorded(v), "unused_binding={unused_binding}");
    }
}
#[test]
fn pure_helper_audit_includes_unused_bindings_and_unselected_branches() {
    for unused_binding in [false, true] {
        let mut v = crypto();
        let body = v["circuits"][0]["body"].clone();
        let read = json!({"kind":"cell_read","field":"active","index":1});
        v["circuits"][0]["body"] = if unused_binding {
            json!({"kind":"let","bindings":[{"name":"unused_read","ty":{"kind":"boolean"},"value":read}],"body":body})
        } else {
            // The unreachable arm still has an illegal ledger condition.
            json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":body,"otherwise":{"kind":"if","condition":read,"then":body,"otherwise":body}})
        };
        assert!(!recorded(v), "unused_binding={unused_binding}");
    }
}
#[test]
fn declaration_names_are_not_admission_keys() {
    fn rename(v: &mut Value) {
        match v {
            Value::String(s)
                if matches!(
                    s.as_str(),
                    "authorize" | "schnorrVerify" | "schnorrVerifyDigest" | "digest" | "note"
                ) =>
            {
                *s = format!("renamed_{s}")
            }
            Value::Array(a) => a.iter_mut().for_each(rename),
            Value::Object(o) => o.values_mut().for_each(rename),
            _ => {}
        }
    }
    let mut v = crypto();
    rename(&mut v);
    assert!(recorded(v));
}
