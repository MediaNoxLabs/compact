// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let input = "../../tools/compact-rust-backend/tests/effect-plan-jubjub-schema20-ir.json";
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
                "{} must be recorded",
                circuit.name
            );
            circuit.name.as_str()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        std::collections::BTreeSet::from(["compose", "read_branch", "raw_after_write"])
    );
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("contract.rs"), rendered.source).unwrap();
}
