// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Append one explicitly synthetic witness/default/reset law to unchanged full source IR.
fn main() {
    let path = "tests/witnessed_mutations/original-ir.json";
    println!("cargo:rerun-if-changed={path}");
    let mut ir: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut root = ir["stateful_circuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "put_true")
        .unwrap()
        .clone();
    root["name"] = serde_json::json!("mutate_witnessed");
    root["source"] =
        serde_json::json!({"file":"synthetic-witnessed-map-mutations.ir","line":1,"column":1});
    root["parameters"] = serde_json::json!([{"name":"selected","ty":{"kind":"boolean"}}]);
    root["actions"] = serde_json::json!([{
        "kind":"if", "condition":{"kind":"parameter","name":"selected"},
        "then":{
            "kind":"let", "bindings":[{"name":"key","ty":{"kind":"boolean"},
                "value":{"kind":"witness_call","name":"has_true","arguments":[]}}],
            "action":{"kind":"sequence","actions":[
                {"kind":"map_insert_default","field":"table","index":0,
                    "key":{"kind":"parameter","name":"key"}},
                {"kind":"map_reset","field":"table","index":0}
            ]}
        },
        "otherwise":{"kind":"sequence","actions":[]}
    }]);
    ir["stateful_circuits"].as_array_mut().unwrap().push(root);
    let rendered =
        compact_rust_backend::render_with_capabilities(&serde_json::from_value(ir).unwrap())
            .unwrap();
    let c = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "mutate_witnessed")
        .unwrap();
    assert!(c.recorded && c.observed_call, "{c:?}");
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("witnessed_mutations.rs"),
        rendered.source,
    )
    .unwrap();
}
