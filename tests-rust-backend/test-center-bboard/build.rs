// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let input = "../../tools/compact-rust-backend/tests/bboard-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    let ir = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&ir).unwrap();
    let mut names = std::collections::BTreeSet::new();
    for circuit in &rendered.capabilities.circuits {
        assert!(circuit.recorded && circuit.observed_call, "{circuit:?}");
        names.insert(circuit.name.as_str());
    }
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["post", "take_down"])
    );
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contract.rs"),
        rendered.source,
    )
    .unwrap();
}
