// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

#[path = "build_support/wide_values.rs"]
mod wide_values;

#[path = "build_support/legacy_abi.rs"]
mod legacy_abi;

fn main() {
    let input = "../../tools/compact-rust-backend/tests/effectful-return-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    println!("cargo:rerun-if-changed=build_support/wide_values.rs");
    let contract = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    let choose = rendered
        .capabilities
        .circuits
        .iter()
        .find(|item| item.name == "choose")
        .unwrap();
    assert!(choose.recorded && choose.observed_call);
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("contract.rs"), rendered.source).unwrap();
    println!("cargo:rerun-if-changed=legacy_reference.rs");
    println!("cargo:rerun-if-changed=build_support/legacy_abi.rs");
    let frozen = std::fs::read_to_string("legacy_reference.rs").unwrap();
    let projected = legacy_abi::project(&frozen).expect("authenticated legacy test ABI projection");
    std::fs::write(output.join("legacy_reference_abi52.rs"), projected).unwrap();

    // The standalone original fixture remains unchanged. Retain the real wide
    // source's choose body in this separate output, then append IR-only probes.
    let wide_input = "../../tools/compact-rust-backend/tests/effect-plan-wide-schema20-ir.json";
    println!("cargo:rerun-if-changed={wide_input}");
    let source_wide = serde_json::from_slice(&std::fs::read(wide_input).unwrap()).unwrap();
    let wide = compact_rust_backend::render_with_capabilities(&wide_values::contract(&source_wide))
        .expect("wide IR probes remain native-valid and recordable");
    assert!(
        wide.capabilities
            .circuits
            .iter()
            .all(|item| item.recorded && item.observed_call)
    );
    std::fs::write(output.join("wide_contract.rs"), wide.source).unwrap();
}
