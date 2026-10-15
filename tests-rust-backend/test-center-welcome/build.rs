// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicitly synthetic old-domain variants; not claimed as original source IR.
fn main() {
    let path = "../../tools/compact-rust-backend/tests/welcome-schema20-ir.json";
    println!("cargo:rerun-if-changed={path}");
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for result in ["unit", "field"] {
        let mut variant = original.clone();
        variant.as_object_mut().unwrap().remove("constructor");
        variant["type_aliases"] = serde_json::json!([]);
        variant["circuits"] = serde_json::json!([]);
        variant["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .retain(|c| c["name"] == "check_in");
        variant["witnesses"]
            .as_array_mut()
            .unwrap()
            .retain(|w| w["name"] == "set_local_id");
        variant["witnesses"][0]["result"] = serde_json::json!({"kind": result});
        let ir = serde_json::from_value(variant).unwrap();
        let rendered = compact_rust_backend::render_with_capabilities(&ir).unwrap();
        assert_eq!(rendered.capabilities.circuits.len(), 1);
        let capability = &rendered.capabilities.circuits[0];
        assert!(
            capability.recorded && capability.observed_call,
            "{capability:?}"
        );
        std::fs::write(
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
                .join(format!("check_in_{result}.rs")),
            rendered.source,
        )
        .unwrap();
    }
}
