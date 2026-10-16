// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    println!("cargo:rerun-if-changed=retained-ir.json");
    let contract = serde_json::from_str(include_str!("retained-ir.json")).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    let c = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "subtracted")
        .unwrap();
    assert!(c.recorded && c.observed_call, "{c:?}");
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contract.rs"),
        rendered.source,
    )
    .unwrap();
}
