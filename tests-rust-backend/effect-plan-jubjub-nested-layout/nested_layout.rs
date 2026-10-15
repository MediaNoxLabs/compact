// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Synthetic native-valid 16-field layout; never used by the source proof adapter.
pub fn render(input: &[u8]) -> String {
    let mut ir: serde_json::Value = serde_json::from_slice(input).unwrap();
    let fields = ir["ledger_fields"].as_array_mut().unwrap();
    fields[0]["path"] = serde_json::json!([0, 0]);
    fields[1]["path"] = serde_json::json!([1, 0]);
    for ordinal in 2..16 {
        fields.push(serde_json::json!({
            "id": format!("padding_{ordinal}"),
            "index": 1,
            "path": [1, ordinal - 1],
            "declaration": {"kind": "cell", "ty": {"kind": "jubjub_point"}}
        }));
    }
    for field in fields {
        field.as_object_mut().unwrap().remove("source");
    }
    for circuit in ir["stateful_circuits"].as_array_mut().unwrap() {
        circuit.as_object_mut().unwrap().remove("source");
    }
    // Tests supply a complete nested state independently of source constructors.
    ir.as_object_mut().unwrap().remove("constructor");
    let contract = serde_json::from_value(ir).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 3);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    rendered.source
}
