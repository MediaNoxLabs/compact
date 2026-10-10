// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let path =
        "../../tools/compact-rust-backend/tests/effect-plan-internal-interfaces-schema20-ir.json";
    println!("cargo:rerun-if-changed={path}");
    let bytes = std::fs::read(path).unwrap();
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let roots = document["stateful_circuits"].as_array().unwrap();
    let touch = roots.iter().find(|root| root["name"] == "touch").unwrap();
    let project = roots.iter().find(|root| root["name"] == "project").unwrap();
    for root in [touch, project] {
        assert_eq!(root["internal"], true);
        assert_eq!(root["parameters"].as_array().unwrap().len(), 1);
        assert_eq!(
            root["parameters"][0]["ty"]["elements"]
                .as_array()
                .unwrap()
                .len(),
            12
        );
    }
    assert_eq!(touch["result"], touch["parameters"][0]["ty"]);
    let ir = serde_json::from_slice(&bytes).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&ir).unwrap();
    assert_eq!(
        rendered
            .capabilities
            .circuits
            .iter()
            .map(|c| c.name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["run"])
    );
    for circuit in &rendered.capabilities.circuits {
        assert!(circuit.recorded && circuit.observed_call, "{circuit:?}");
    }
    assert!(!rendered.source.contains("fn touch_call"));
    assert!(!rendered.source.contains("fn project_call"));
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contract.rs"),
        rendered.source,
    )
    .unwrap();
}
