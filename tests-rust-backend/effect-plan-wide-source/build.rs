// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let input = "../../tools/compact-rust-backend/tests/effect-plan-wide-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    let contract = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 1);
    let choose = &rendered.capabilities.circuits[0];
    assert_eq!(choose.name, "choose");
    assert!(choose.recorded && choose.observed_call);
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("contract.rs"), rendered.source).unwrap();
}
