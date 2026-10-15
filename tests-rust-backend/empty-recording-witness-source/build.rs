// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    println!(
        "cargo:rerun-if-changed=../../tools/compact-rust-backend/tests/empty-witness-guard-schema20-ir.json"
    );
    let contract = serde_json::from_str(include_str!(
        "../../tools/compact-rust-backend/tests/empty-witness-guard-schema20-ir.json"
    ))
    .unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    let root = rendered
        .capabilities
        .circuits
        .iter()
        .find(|row| row.name == "recordFreshEnough")
        .unwrap();
    assert!(root.recorded && root.observed_call, "{root:?}");
    let path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(path.join("contract.rs"), rendered.source).unwrap();
}
