// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let input = "../../tools/compact-rust-backend/tests/effect-plan-owned-cells-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    let bytes = std::fs::read(input).unwrap();
    let contract = serde_json::from_slice(&bytes).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    let names = rendered
        .capabilities
        .circuits
        .iter()
        .map(|circuit| {
            assert!(
                circuit.recorded && circuit.observed_call,
                "{} must be recorded: {:?}",
                circuit.name,
                circuit
            );
            circuit.name.as_str()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["run", "inspectText"])
    );
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("contract.rs"), rendered.source).unwrap();
    let inputs = "../../tools/compact-rust-backend/tests/opaque-cell-inputs-schema20-ir.json";
    println!("cargo:rerun-if-changed={inputs}");
    let contract = serde_json::from_slice(&std::fs::read(inputs).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    let names = rendered
        .capabilities
        .circuits
        .iter()
        .map(|circuit| {
            assert!(
                !circuit.recorded && !circuit.observed_call,
                "opaque input interfaces must not expand: {:?}",
                circuit
            );
            circuit.name.as_str()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["setText", "setBytes", "setEnvelope"])
    );
    std::fs::write(output.join("inputs_contract.rs"), rendered.source).unwrap();
    let order = "../../tools/compact-rust-backend/tests/opaque-constructor-order-schema20-ir.json";
    println!("cargo:rerun-if-changed={order}");
    let contract = serde_json::from_slice(&std::fs::read(order).unwrap()).unwrap();
    let source = compact_rust_backend::render(&contract).unwrap();
    std::fs::write(output.join("constructor_order.rs"), source).unwrap();
}
