// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#[path = "build/synthetic.rs"]
mod synthetic;
fn main() {
    let input = "tests/synthetic/original-ir.json";
    println!("cargo:rerun-if-changed={input}");
    println!("cargo:rerun-if-changed=build/synthetic.rs");
    let original = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let ir = synthetic::contract(original);
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(
        out.join("synthetic-ir.json"),
        serde_json::to_vec_pretty(&ir).unwrap(),
    )
    .unwrap();
    let rendered =
        compact_rust_backend::render_with_capabilities(&serde_json::from_value(ir).unwrap())
            .unwrap();
    for capability in &rendered.capabilities.circuits {
        assert!(
            capability.recorded && capability.observed_call,
            "{capability:?}"
        );
    }
    std::fs::write(out.join("synthetic.rs"), rendered.source).unwrap();
}
