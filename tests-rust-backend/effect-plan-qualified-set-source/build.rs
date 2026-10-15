// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let input =
        "../../tools/compact-rust-backend/tests/effect-plan-qualified-set-source-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    let ir: serde_json::Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let fields = ir["ledger_fields"].as_array().unwrap();
    assert_eq!(fields.len(), 16);
    let coins = fields.iter().find(|field| field["id"] == "coins").unwrap();
    assert_eq!(coins["path"], serde_json::json!([1, 14]));
    let rendered =
        compact_rust_backend::render_with_capabilities(&serde_json::from_value(ir).unwrap())
            .unwrap();
    assert_eq!(
        rendered
            .capabilities
            .circuits
            .iter()
            .map(|c| c.name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["insert_once", "insert_selected"])
    );
    for circuit in &rendered.capabilities.circuits {
        assert!(
            circuit.recorded && circuit.observed_call,
            "{}: {circuit:?}",
            circuit.name
        );
    }
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contract.rs"),
        rendered.source,
    )
    .unwrap();
}
