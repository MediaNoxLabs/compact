// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete, untouched consumer exports for separate compilation qualification.
use super::*;

pub(super) fn export(
    source: &str,
    contract: &Contract,
    root: &StatefulCircuit,
    label: &str,
    require_old: bool,
    old_status: &str,
    checked: serde_json::Value,
) {
    let (rendered, invocations) = trace::capture_json(|| crate::render_with_capabilities(contract));
    let rendered = rendered.unwrap_or_else(|error| panic!("{label}: full native render {error:?}"));
    let rows: Vec<_> = invocations
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["circuit"] == root.name
                && row["phase"] == "public_emission"
                && row["helper"] == false
        })
        .collect();
    assert_eq!(rows.len(), 1, "{label}: actual public emission");
    assert_eq!(rows[0]["outcome"]["status"], "supported");
    assert_eq!(
        rows[0]["body_owner"],
        if require_old {
            "selector.unit-composition"
        } else {
            "algebra.composite-values"
        }
    );
    let metadata = json!({
        "label":label,"root":root.name,"internal":root.internal,
        "source_variant":"unchanged retained full source IR",
        "whole_old_required":require_old,"whole_old_unit_composition":old_status,
        "checked":checked,"full_native_render":true,"public_emission":rows[0],
        "capabilities":rendered.capabilities,
        "claim":"Finite whole-root admission/native rendering, not Rust compilation, full family inclusion, runtime or proof."
    });
    eprintln!(
        "ORDERED_COMPARISON_PROBE {}",
        serde_json::to_string(&metadata).unwrap()
    );
    if let Some(base) = std::env::var_os("COMPACT_ORDERED_COMPARISONS_OUTPUT") {
        let directory = std::path::PathBuf::from(base).join(label);
        std::fs::create_dir_all(directory.parent().unwrap()).unwrap();
        std::fs::create_dir(&directory).expect("fresh case output required");
        std::fs::write(directory.join("compact-rust-ir.json"), source).unwrap();
        std::fs::write(directory.join("lib.rs"), rendered.source).unwrap();
        std::fs::write(
            directory.join("case.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    }
}
