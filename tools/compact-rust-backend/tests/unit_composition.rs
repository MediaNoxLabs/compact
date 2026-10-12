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
fn checked_hidden_effects_compose_but_unselected_unsupported_effects_are_refused() {
    for chunked in [false, true] {
        let mut v = fixture(chunked);
        let actions = circuit(&mut v, "guard")["actions"].as_array_mut().unwrap();
        let index = actions.len();
        actions.push(json!({
            "kind":"if", "condition":{"kind":"boolean","value":true},
            "then":{"kind":"sequence","actions":[]},
            "otherwise":{"kind":"expression","value":{"kind":"kernel_self","ty":{"kind":"struct","name":"ContractAddress","fields":[{"name":"bytes","ty":{"kind":"bytes","length":32}}]}}}
        }));
        // KernelSelf is a checked operation, including inside an unselected
        // stateful helper branch. The branch must remain in generated recording.
        let contract: Contract = serde_json::from_value(v.clone()).unwrap();
        let output = render_with_capabilities(&contract).unwrap();
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "close")
            .unwrap();
        assert!(
            capability.recorded && capability.observed_call,
            "chunked={chunked}"
        );
        assert!(
            output
                .source
                .split("pub mod recorded")
                .nth(1)
                .unwrap()
                .contains("kernel_self()")
        );

        // The configured-key observation is now a checked private effect.
        // Its unselected arm must remain present and lazy in generated code.
        circuit(&mut v, "guard")["actions"][index]["otherwise"] = json!({
            "kind":"native_witness_call", "builtin":"own_public_key"
        });
        let contract: Contract = serde_json::from_value(v.clone()).unwrap();
        let output = render_with_capabilities(&contract).unwrap();
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "close")
            .unwrap();
        assert!(
            capability.recorded && capability.observed_call,
            "chunked={chunked}"
        );
        assert!(
            output
                .source
                .split("pub mod recorded")
                .nth(1)
                .unwrap()
                .contains("own_coin_public_key()?")
        );

        // Generic Boolean inequality is now checked even in the lazy arm.
        circuit(&mut v, "guard")["actions"][index]["otherwise"] = json!({
            "kind":"expression", "value": {
                "kind":"not_equal", "left":{"kind":"boolean","value":true},
                "right":{"kind":"boolean","value":false}
            }
        });
        let contract: Contract = serde_json::from_value(v.clone()).unwrap();
        let output = render_with_capabilities(&contract).unwrap();
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "close")
            .unwrap();
        assert!(
            capability.recorded && capability.observed_call,
            "chunked={chunked}"
        );

        // Keccak is checked even inside this unselected helper arm.
        circuit(&mut v, "guard")["actions"][index]["otherwise"] = json!({
            "kind":"expression", "value": {
                "kind":"equal", "left": {
                    "kind":"keccak256", "value":{"kind":"bytes_literal","bytes":vec![7u8; 32]}
                }, "right":{"kind":"bytes_literal","bytes":vec![0u8; 32]}
            }
        });
        let contract: Contract = serde_json::from_value(v.clone()).unwrap();
        let output = render_with_capabilities(&contract).unwrap();
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "close")
            .unwrap();
        assert!(
            capability.recorded && capability.observed_call,
            "chunked={chunked}"
        );
        assert!(
            output
                .source
                .split("pub mod recorded")
                .nth(1)
                .unwrap()
                .contains("keccak256")
        );

        // Scalar defaults now lower in the unselected helper arm as well.
        circuit(&mut v, "guard")["actions"][index]["otherwise"] = json!({
            "kind":"expression", "value":{"kind":"default","ty":{"kind":"boolean"}}
        });
        let contract: Contract = serde_json::from_value(v.clone()).unwrap();
        let output = render_with_capabilities(&contract).unwrap();
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "close")
            .unwrap();
        assert!(
            capability.recorded && capability.observed_call,
            "chunked={chunked}"
        );

        // Fixed-FAB Field commitments now compose in the unselected helper arm.
        circuit(&mut v, "guard")["actions"][index]["otherwise"] = json!({
            "kind":"expression", "value": {"kind":"persistent_commit",
                "value":{"kind":"field_literal","value":"0"},
                "opening":{"kind":"bytes_literal","bytes":vec![0u8; 32]}}
        });
        assert!(recorded(v.clone()));
        // Keep whole-helper admission negative using a native-valid typed IR
        // VectorMap child, not an obsolete commitment input restriction.
        circuit(&mut v, "guard")["actions"][index]["otherwise"]["value"]["value"] = json!({"kind":"vector_map",
                "parameter":{"name":"sentinel_item","ty":{"kind":"field"}},
                "source":{"kind":"tuple","elements":[{"kind":"field_literal","value":"1"}]},
                "body":{"kind":"parameter","name":"sentinel_item"},
                "result":{"kind":"field"},"length":1});
        let contract: Contract = serde_json::from_value(v).unwrap();
        let output = render_with_capabilities(&contract).unwrap();
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "close")
            .unwrap();
        assert!(
            !capability.recorded && !capability.observed_call,
            "chunked={chunked}"
        );
        assert!(capability.recording_unavailable.is_some());
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
fn stateful_crypto_helpers_record_public_reads_and_reject_invalid_slots() {
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
        let output =
            render_with_capabilities(&serde_json::from_value::<Contract>(v.clone()).unwrap())
                .unwrap();
        assert!(
            output
                .capabilities
                .circuits
                .iter()
                .any(|c| c.name == "close" && c.recorded && c.observed_call)
        );
        let body = output.source.split("pub mod recorded").nth(1).unwrap();
        let baseline =
            render_with_capabilities(&serde_json::from_value::<Contract>(crypto()).unwrap())
                .unwrap();
        let before = baseline.source.split("pub mod recorded").nth(1).unwrap();
        assert!(body.matches("record_read(").count() > before.matches("record_read(").count());
        let actions = circuit(&mut v, "schnorrVerify")["actions"]
            .as_array_mut()
            .unwrap();
        let condition = &mut actions.last_mut().unwrap()["condition"];
        let read = if unused_binding {
            &mut condition["bindings"][0]["value"]
        } else {
            &mut condition["otherwise"]
        };
        read["index"] = json!(99);
        assert!(
            !recorded(v),
            "invalid read slot, unused_binding={unused_binding}"
        );
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
