// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    for (name, receipt, required) in [
        (
            "bboard",
            "bboard-schema20-ir.json",
            &["post", "take_down"][..],
        ),
        (
            "nested",
            "opaque-observed-nested-schema20-ir.json",
            &["accept"][..],
        ),
    ] {
        let path = format!("../../tools/compact-rust-backend/tests/{receipt}");
        println!("cargo:rerun-if-changed={path}");
        let ir = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let rendered = compact_rust_backend::render_with_capabilities(&ir).unwrap();
        for name in required {
            let circuit = rendered
                .capabilities
                .circuits
                .iter()
                .find(|c| c.name == *name)
                .unwrap();
            assert!(circuit.recorded && circuit.observed_call, "{circuit:?}");
        }
        std::fs::write(
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
                .join(format!("{name}.rs")),
            rendered.source,
        )
        .unwrap();
    }
    // Compatibility-only typed-IR variant: preserve the complete maintained
    // send contract and append unused formals; no source/owner expansion claim.
    let path = "../../tools/compact-rust-backend/tests/shielded-send-schema20-ir.json";
    println!("cargo:rerun-if-changed={path}");
    let mut ir: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let root = ir["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "send_to_user")
        .unwrap();
    let body = root["actions"].clone();
    let result = root["return_value"].clone();
    let tuple = serde_json::json!({"kind":"tuple", "elements": [
        {"kind":"opaque_bytes"}, {"kind":"field"}, {"kind":"field"},
        {"kind":"field"}, {"kind":"field"}, {"kind":"field"},
        {"kind":"field"}, {"kind":"field"}, {"kind":"field"}
    ]});
    let parameters = root["parameters"].as_array_mut().unwrap();
    parameters.push(serde_json::json!({"name":"unused_tuple", "ty":tuple.clone()}));
    parameters.push(serde_json::json!({"name":"unused_vector", "ty":{
        "kind":"vector", "element":tuple, "length":2
    }}));
    assert_eq!(root["actions"], body);
    assert_eq!(root["return_value"], result);
    let rendered =
        compact_rust_backend::render_with_capabilities(&serde_json::from_value(ir).unwrap())
            .unwrap();
    let circuit = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "send_to_user")
        .unwrap();
    assert!(circuit.recorded && circuit.observed_call, "{circuit:?}");
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("tuple.rs"),
        rendered.source,
    )
    .unwrap();
}
