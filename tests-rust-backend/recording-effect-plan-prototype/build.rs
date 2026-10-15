// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "experimental-recording-plan")]
#[path = "support/metamorphic.rs"]
mod metamorphic;

#[cfg(feature = "experimental-recording-plan")]
fn main() {
    let input = "../../tools/compact-rust-backend/tests/effect-plan-prototype-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    println!("cargo:rerun-if-changed=support/metamorphic.rs");
    let mut ir = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    metamorphic::append_variants(&mut ir);
    let contract = serde_json::from_value(ir).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 11);
    for circuit in &rendered.capabilities.circuits {
        assert!(
            circuit.recorded && circuit.observed_call,
            "{}",
            circuit.name
        );
    }
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("contract.rs"), rendered.source).unwrap();
}

#[cfg(not(feature = "experimental-recording-plan"))]
fn main() {}
