// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
mod nested_layout;
fn main() {
    let input = "../../tools/compact-rust-backend/tests/effect-plan-jubjub-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    println!("cargo:rerun-if-changed=nested_layout.rs");
    let bytes = std::fs::read(input).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("contract.rs"), nested_layout::render(&bytes)).unwrap();
}
