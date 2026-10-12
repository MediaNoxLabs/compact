// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Synthetic trait-surface control appended to exact genuine IR, not a new source capture.
pub fn render(input: &str, output: &std::path::Path) {
    let mut ir: serde_json::Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let mut root = ir["stateful_circuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "commitSmall")
        .unwrap()
        .clone();
    root["name"] = serde_json::json!("commit_product");
    root["source"] =
        serde_json::json!({"file":"synthetic-fixed-commit-product.ir","line":1,"column":1});
    root["parameters"] = serde_json::json!([{"name":"payload","ty":{"kind":"struct","name":"CommitPayload","fields":[
        {"name":"fields","ty":{"kind":"vector","length":2,"element":{"kind":"field"}}},
        {"name":"flag","ty":{"kind":"boolean"}}
    ]}}]);
    assert_eq!(
        root["actions"][0]["bindings"][0]["value"]["kind"],
        "persistent_commit"
    );
    root["actions"][0]["bindings"][0]["value"]["value"] =
        serde_json::json!({"kind":"parameter","name":"payload"});
    ir["stateful_circuits"].as_array_mut().unwrap().push(root);
    std::fs::write(
        output.join("product-ir.json"),
        serde_json::to_vec_pretty(&ir).unwrap(),
    )
    .unwrap();
    let rendered =
        compact_rust_backend::render_with_capabilities(&serde_json::from_value(ir).unwrap())
            .unwrap();
    let root = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "commit_product")
        .unwrap();
    assert!(root.recorded && root.observed_call, "{root:?}");
    std::fs::write(output.join("product.rs"), rendered.source).unwrap();
}
