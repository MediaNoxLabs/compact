// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let path = "../../tools/compact-rust-backend/tests/effect-plan-maps-schema20-ir.json";
    println!("cargo:rerun-if-changed={path}");
    let ir = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&ir).unwrap();
    assert_eq!(
        rendered
            .capabilities
            .circuits
            .iter()
            .map(|c| c.name.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "member",
            "lookup",
            "put",
            "remove",
            "lazy_put",
            "direct_member",
            "direct_lookup"
        ])
    );
    for circuit in &rendered.capabilities.circuits {
        let expected = true;
        assert_eq!(circuit.recorded, expected, "{circuit:?}");
        assert_eq!(circuit.observed_call, expected, "{circuit:?}");
    }
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contract.rs"),
        rendered.source,
    )
    .unwrap();
}
