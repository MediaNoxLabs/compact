// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Authenticated finite whole-old inclusion checkpoint; never selects production routes.

#[path = "broad_unit_inclusion_tests/inputs.rs"]
mod inputs;
#[path = "broad_unit_inclusion_tests/measurement.rs"]
mod measurement;

use serde_json::json;
use std::path::PathBuf;

#[test]
#[ignore = "requires the authenticated external matrix and a fresh durable output directory"]
fn measure_complete_old_unit_rule_matrix() {
    let input = PathBuf::from(std::env::var_os("COMPACT_UNIT_RULE_MATRIX").expect("matrix"));
    let output =
        PathBuf::from(std::env::var_os("COMPACT_UNIT_RULE_OUTPUT").expect("output directory"));
    // Authenticate the entire corpus and old implementation before creating outputs
    // or evaluating a single checker; partial reports cannot masquerade as complete.
    let cases = inputs::load(&input).expect("authenticated original matrix");
    let authority = inputs::legacy_authority();
    assert!(!output.exists(), "preserve earlier measurements");
    std::fs::create_dir(&output).unwrap();
    std::fs::create_dir(output.join("exports")).unwrap();
    let rows: Vec<_> = cases
        .iter()
        .map(|case| measurement::measure(case, &output))
        .collect();
    let executable = std::env::current_exe().unwrap();
    let report = json!({
        "scope": "Finite typed-IR checkpoint, not specification completeness or Rust compilation evidence",
        "manifest_sha256": inputs::MANIFEST_SHA256,
        "legacy_authority": authority,
        "test_binary": {"path": executable, "sha256": inputs::digest(&std::fs::read(&executable).unwrap())},
        "rows": rows,
    });
    std::fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
