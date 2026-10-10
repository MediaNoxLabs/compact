// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let input = "../../tools/compact-rust-backend/tests/effect-plan-merkle-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    let contract = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 7);
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
    let nested_input =
        "../../tools/compact-rust-backend/tests/effect-plan-merkle-nested-schema20-ir.json";
    println!("cargo:rerun-if-changed={nested_input}");
    let nested_ir: serde_json::Value =
        serde_json::from_slice(&std::fs::read(nested_input).unwrap()).unwrap();
    let fields = nested_ir["ledger_fields"].as_array().unwrap();
    let plain = fields.iter().find(|field| field["id"] == "plain").unwrap();
    let historic = fields
        .iter()
        .find(|field| field["id"] == "historic")
        .unwrap();
    assert_eq!(plain["path"], serde_json::json!([0, 0]));
    assert_eq!(historic["path"], serde_json::json!([1, 14]));
    let nested =
        compact_rust_backend::render_with_capabilities(&serde_json::from_value(nested_ir).unwrap())
            .unwrap();
    assert_eq!(nested.capabilities.circuits.len(), 1);
    assert!(
        nested.capabilities.circuits[0].recorded && nested.capabilities.circuits[0].observed_call,
        "nested Merkle: {:?}",
        nested.capabilities.circuits[0]
    );
    // This test crate hosts two generated contracts. Rebase only the secondary
    // generated crate's internal references into its dedicated Rust module.
    // Production emission and the retained IR remain unmodified.
    let source = nested.source.replace("crate::", "crate::nested::");
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("nested.rs"),
        source,
    )
    .unwrap();
}
