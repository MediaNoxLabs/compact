// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let input = "../../tools/compact-rust-backend/tests/hmt-insert-oracle-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    let contract = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    for name in [
        "append",
        "place",
        "append_hash",
        "place_hash",
        "forget_history",
        "reset_tree",
        "full",
        "known",
    ] {
        let circuit = rendered
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(circuit.recorded && circuit.observed_call, "{circuit:?}");
    }
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contract.rs"),
        rendered.source,
    )
    .unwrap();
}
