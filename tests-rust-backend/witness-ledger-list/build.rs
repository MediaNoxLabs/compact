// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Append one explicitly synthetic witness/push law to unchanged full source IR.
fn main() {
    let path = "tests/witnessed_push/original-ir.json";
    println!("cargo:rerun-if-changed={path}");
    let mut ir: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut root = ir["stateful_circuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "prepend")
        .unwrap()
        .clone();
    root["name"] = serde_json::json!("prepend_witnessed");
    root["source"] =
        serde_json::json!({"file":"synthetic-witnessed-list-push.ir","line":1,"column":1});
    root["parameters"].as_array_mut().unwrap().insert(
        0,
        serde_json::json!({"name":"selected","ty":{"kind":"boolean"}}),
    );
    root["actions"] = serde_json::json!([{
        "kind":"if", "condition":{"kind":"parameter","name":"selected"},
        "then":{
            "kind":"let", "bindings":[{"name":"flag","ty":{"kind":"boolean"},
                "value":{"kind":"witness_call","name":"first_is_42","arguments":[]}}],
            "action":{"kind":"list_push_front","field":"items","index":0,
                "value":{"kind":"if","condition":{"kind":"parameter","name":"flag"},
                    "then":{"kind":"parameter","name":"value"},
                    "otherwise":{"kind":"field_literal","value":"0"}}}
        },
        "otherwise":{"kind":"list_push_front","field":"items","index":0,
            "value":{"kind":"field_literal","value":"0"}}
    }]);
    ir["stateful_circuits"].as_array_mut().unwrap().push(root);
    let rendered =
        compact_rust_backend::render_with_capabilities(&serde_json::from_value(ir).unwrap())
            .unwrap();
    let c = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "prepend_witnessed")
        .unwrap();
    assert!(c.recorded && c.observed_call, "{c:?}");
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("witnessed_push.rs"),
        rendered.source,
    )
    .unwrap();
}
