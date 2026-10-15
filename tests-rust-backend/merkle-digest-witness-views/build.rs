// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let path =
        "../../tools/compact-rust-backend/tests/merkle-digest-witness-views-schema20-ir.json";
    println!("cargo:rerun-if-changed={path}");
    let ir = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let output = compact_rust_backend::render_with_capabilities(&ir).unwrap();
    assert_eq!(output.capabilities.circuits.len(), 1);
    assert_eq!(output.capabilities.circuits[0].name, "read_flag");
    assert!(output.capabilities.circuits[0].recorded);
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contract.rs"),
        output.source,
    )
    .unwrap();
}
