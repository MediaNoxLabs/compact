// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let input = "../../tools/compact-rust-backend/tests/native-zswap-intents-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    let contract = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    let mut names = std::collections::BTreeSet::new();
    for circuit in &rendered.capabilities.circuits {
        // Intent-only produce/consume now record through typed composition.
        // Recording capability does not make an empty public trace preparable.
        assert!(circuit.recorded, "{} recorded", circuit.name);
        assert!(circuit.observed_call, "{} observed", circuit.name);
        names.insert(circuit.name.as_str());
    }
    assert_eq!(
        names,
        std::collections::BTreeSet::from([
            "flow",
            "witness_order",
            "read_coin",
            "produce",
            "consume"
        ])
    );
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("contract.rs"), rendered.source).unwrap();
}
